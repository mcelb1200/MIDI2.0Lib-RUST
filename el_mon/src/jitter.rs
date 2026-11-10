//! Real-Time Jitter Analyzer & Delta Clockstamp Profiler.
//!
//! M2-104-UM Utility Messages:
//! - MT=0x0, Status 0x1: JR Clock (16-bit clock timestamp). Frequency nominally 31,250 Hz (32 µs per tick).
//! - MT=0x0, Status 0x2: JR Timestamp (16-bit event timestamp).
//! - MT=0x0, Status 0x3: Delta Clockstamp Ticks (DCT ticks per quarter note).
//! - MT=0x0, Status 0x4: Delta Clockstamp Since Last Event (16-bit tick count since prior event).

use el_core::ump::Ump;

/// JR Clock tick resolution in microseconds: 1 / 31,250 Hz = 32 µs.
pub const JR_TICK_US: f64 = 32.0;

/// Statistical summary for time-series measurements (in microseconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitterStats {
    pub count: u64,
    pub min_us: f64,
    pub max_us: f64,
    pub mean_us: f64,
    pub m2: f64, // Used for Welford's algorithm to calculate variance
    pub dropped_packets: u64,
}

impl Default for JitterStats {
    fn default() -> Self {
        Self::new()
    }
}

impl JitterStats {
    /// Creates a new empty `JitterStats`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            count: 0,
            min_us: f64::MAX,
            max_us: 0.0,
            mean_us: 0.0,
            m2: 0.0,
            dropped_packets: 0,
        }
    }

    /// Records a jitter deviation measurement in microseconds.
    pub fn record(&mut self, jitter_us: f64) {
        let abs_jitter = jitter_us.abs();
        self.count += 1;
        if abs_jitter < self.min_us {
            self.min_us = abs_jitter;
        }
        if abs_jitter > self.max_us {
            self.max_us = abs_jitter;
        }

        // Welford's algorithm for online variance calculation
        let delta = abs_jitter - self.mean_us;
        self.mean_us += delta / (self.count as f64);
        let delta2 = abs_jitter - self.mean_us;
        self.m2 += delta * delta2;
    }

    /// Returns standard deviation of jitter in microseconds.
    #[must_use]
    pub fn std_dev_us(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            (self.m2 / ((self.count - 1) as f64)).sqrt()
        }
    }

    /// Increments the dropped or missing packet count.
    pub fn record_dropped(&mut self, count: u64) {
        self.dropped_packets += count;
    }
}

/// Jitter Reduction & Delta Clockstamp analyzer tracking packet timing.
#[derive(Debug, Clone)]
pub struct JitterAnalyzer {
    /// Current sender JR Clock (16-bit)
    pub last_jr_clock: Option<u16>,
    /// Last processed JR Timestamp (16-bit)
    pub last_jr_timestamp: Option<u16>,
    /// Nominal inter-packet interval expected in microseconds (if known, e.g. from constant stream)
    pub nominal_interval_us: Option<f64>,
    /// Last event timestamp in sender clock ticks (unwrapped)
    pub last_unwrapped_ticks: Option<u64>,
    /// Upper 48 bits of clock ticks unwrapping
    pub clock_wrap_counter: u64,
    /// Last raw 16-bit tick value seen for unwrap tracking
    pub last_raw_tick: Option<u16>,
    /// Accumulated Jitter Statistics
    pub stats: JitterStats,
    /// Last Delta Clockstamp Ticks (ticks per quarter note)
    pub dct_ticks_per_qn: Option<u16>,
    /// Last Delta Clockstamp Since Last Event (ticks)
    pub last_dct_since_last: Option<u16>,
    /// Count of JR Clock messages received
    pub jr_clock_count: u64,
    /// Count of JR Timestamp messages received
    pub jr_timestamp_count: u64,
    /// Count of DCT messages received
    pub dct_count: u64,
}

impl Default for JitterAnalyzer {
    fn default() -> Self {
        Self::new(None)
    }
}

impl JitterAnalyzer {
    /// Creates a new `JitterAnalyzer`.
    ///
    /// If `nominal_interval_us` is provided, intervals between timestamps are compared against it.
    /// Otherwise, jitter is calculated as deviations from the moving average interval.
    #[must_use]
    pub const fn new(nominal_interval_us: Option<f64>) -> Self {
        Self {
            last_jr_clock: None,
            last_jr_timestamp: None,
            nominal_interval_us,
            last_unwrapped_ticks: None,
            clock_wrap_counter: 0,
            last_raw_tick: None,
            stats: JitterStats::new(),
            dct_ticks_per_qn: None,
            last_dct_since_last: None,
            jr_clock_count: 0,
            jr_timestamp_count: 0,
            dct_count: 0,
        }
    }

    /// Ingests a Universal MIDI Packet (UMP).
    ///
    /// Detects Utility messages (MT=0x0):
    /// - Status 0x1: JR Clock
    /// - Status 0x2: JR Timestamp
    /// - Status 0x3: Delta Clockstamp Ticks
    /// - Status 0x4: Delta Clockstamp Since Last
    ///
    /// Returns `true` if a timing utility message was ingested.
    pub fn ingest_ump(&mut self, ump: &Ump) -> bool {
        let mt = (ump.data[0] >> 28) & 0xF;
        if mt != 0x0 {
            return false;
        }

        let status = ((ump.data[0] >> 20) & 0xF) as u8;
        let val = (ump.data[0] & 0xFFFF) as u16;

        match status {
            0x1 => {
                // JR Clock
                self.jr_clock_count += 1;
                self.process_jr_clock(val);
                true
            }
            0x2 => {
                // JR Timestamp
                self.jr_timestamp_count += 1;
                self.process_jr_timestamp(val);
                true
            }
            0x3 => {
                // Delta Clockstamp Ticks per QN
                self.dct_count += 1;
                self.dct_ticks_per_qn = Some(val);
                true
            }
            0x4 => {
                // Delta Clockstamp Since Last Event
                self.dct_count += 1;
                self.process_dct_since(val);
                true
            }
            _ => false,
        }
    }

    /// Handles an incoming 16-bit JR Clock.
    pub fn process_jr_clock(&mut self, clock_val: u16) {
        if let Some(prev) = self.last_jr_clock {
            // Check for backwards jumps or large drops in clock
            let diff = clock_val.wrapping_sub(prev);
            // In MIDI 2.0 JR Clock, values wrap at 0x0000. If diff is suspiciously negative, packet was out of order or dropped.
            if diff > 0x8000 {
                self.stats.record_dropped(1);
            }
        }
        self.last_jr_clock = Some(clock_val);
    }

    /// Handles an incoming 16-bit JR Timestamp.
    pub fn process_jr_timestamp(&mut self, ts_val: u16) {
        self.last_jr_timestamp = Some(ts_val);
        let unwrapped = self.unwrap_tick(ts_val);

        if let Some(prev_unwrapped) = self.last_unwrapped_ticks {
            if unwrapped >= prev_unwrapped {
                let delta_ticks = unwrapped - prev_unwrapped;
                let delta_us = (delta_ticks as f64) * JR_TICK_US;

                let expected_us = self.nominal_interval_us.unwrap_or(self.stats.mean_us);
                if self.stats.count > 0 || self.nominal_interval_us.is_some() {
                    let jitter = delta_us - expected_us;
                    self.stats.record(jitter);
                } else {
                    // First interval seeds initial stats
                    self.stats.record(0.0);
                }
            } else {
                // Timestamp went backwards, packet out of sequence
                self.stats.record_dropped(1);
            }
        }

        self.last_unwrapped_ticks = Some(unwrapped);
    }

    /// Handles Delta Clockstamp Since Last Event.
    pub fn process_dct_since(&mut self, ticks: u16) {
        self.last_dct_since_last = Some(ticks);
        // If nominal interval is configured in ticks, evaluate deviation
        if let Some(nominal_us) = self.nominal_interval_us {
            let actual_us = (ticks as f64) * JR_TICK_US;
            let jitter = actual_us - nominal_us;
            self.stats.record(jitter);
        }
    }

    /// Unwraps 16-bit circular counter into a monotonic 64-bit counter.
    fn unwrap_tick(&mut self, raw: u16) -> u64 {
        if let Some(prev_raw) = self.last_raw_tick {
            if raw < prev_raw && (prev_raw - raw) > 0x8000 {
                // Forward wrap past 0xFFFF -> 0x0000
                self.clock_wrap_counter = self.clock_wrap_counter.saturating_add(0x1_0000);
            }
        }
        self.last_raw_tick = Some(raw);
        self.clock_wrap_counter.saturating_add(raw as u64)
    }

    /// Resets all statistics.
    pub fn reset(&mut self) {
        self.last_jr_clock = None;
        self.last_jr_timestamp = None;
        self.last_unwrapped_ticks = None;
        self.clock_wrap_counter = 0;
        self.last_raw_tick = None;
        self.stats = JitterStats::new();
        self.dct_ticks_per_qn = None;
        self.last_dct_since_last = None;
        self.jr_clock_count = 0;
        self.jr_timestamp_count = 0;
        self.dct_count = 0;
    }
}

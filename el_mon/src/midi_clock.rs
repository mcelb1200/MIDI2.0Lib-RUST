//! MIDI Clock & BPM Drift Profiler.
//!
//! Measures physical/serial/UMP MIDI Timing Clock (0xF8) intervals.
//! In standard MIDI (1.0 and 2.0), Timing Clock ticks occur 24 times per quarter note (PPQN = 24).
//!
//! Formula:
//! - Nominal interval per tick (seconds): `t_tick = 60.0 / (BPM * 24.0)`
//! - Nominal interval in microseconds: `t_tick_us = (60.0 * 1_000_000.0) / (BPM * 24.0) = 2_500_000.0 / BPM`
//! - Real-time BPM from measured tick interval (in microseconds):
//!   `BPM = 2_500_000.0 / interval_us`
//! - BPM Drift: `BPM_measured - BPM_target`

use crate::jitter::JitterStats;
use el_core::ump::Ump;

/// Tracks MIDI 1.0 / MIDI 2.0 Timing Clock (0xF8) jitter, BPM, and BPM drift.
#[derive(Debug, Clone)]
pub struct MidiClockProfiler {
    /// Target nominal BPM if specified
    pub target_bpm: Option<f64>,
    /// Accumulated tick intervals in microseconds
    pub interval_stats: JitterStats,
    /// Real-time measured instantaneous BPM
    pub current_bpm: f64,
    /// Smoothed / average BPM
    pub average_bpm: f64,
    /// Real-time BPM drift relative to target
    pub bpm_drift: f64,
    /// Total timing clock ticks received
    pub total_ticks: u64,
    /// Last tick reception timestamp in microseconds
    pub last_tick_timestamp_us: Option<u64>,
    /// Number of ticks counted in current beat (0..23)
    pub beat_tick_counter: u8,
}

impl Default for MidiClockProfiler {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MidiClockProfiler {
    /// Creates a new `MidiClockProfiler`.
    #[must_use]
    pub const fn new(target_bpm: Option<f64>) -> Self {
        Self {
            target_bpm,
            interval_stats: JitterStats::new(),
            current_bpm: 0.0,
            average_bpm: 0.0,
            bpm_drift: 0.0,
            total_ticks: 0,
            last_tick_timestamp_us: None,
            beat_tick_counter: 0,
        }
    }

    /// Nominal interval in microseconds for a given BPM.
    #[must_use]
    pub fn nominal_interval_us(bpm: f64) -> f64 {
        if bpm <= 0.0 {
            0.0
        } else {
            2_500_000.0 / bpm
        }
    }

    /// Ingests a Universal MIDI Packet (UMP).
    ///
    /// Checks for System Real-Time Timing Clock (0xF8) in MT=0x1:
    /// - MT=0x1, Status=0xF8 -> `(data[0] >> 16) & 0xFF == 0xF8`
    ///
    /// If matched, records the tick at `timestamp_us`.
    pub fn ingest_ump(&mut self, ump: &Ump, timestamp_us: u64) -> bool {
        let mt = (ump.data[0] >> 28) & 0xF;
        if mt == 0x1 {
            let status = ((ump.data[0] >> 16) & 0xFF) as u8;
            if status == 0xF8 {
                self.record_tick(timestamp_us);
                return true;
            }
        }
        false
    }

    /// Ingests a raw byte (e.g. from DIN/TRS MIDI 1.0 byte stream).
    /// Returns true if it was 0xF8 (Timing Clock).
    pub fn ingest_byte(&mut self, byte: u8, timestamp_us: u64) -> bool {
        if byte == 0xF8 {
            self.record_tick(timestamp_us);
            true
        } else {
            false
        }
    }

    /// Records a timing clock tick at `timestamp_us` (microseconds).
    pub fn record_tick(&mut self, timestamp_us: u64) {
        self.total_ticks += 1;
        self.beat_tick_counter = (self.beat_tick_counter + 1) % 24;

        if let Some(prev) = self.last_tick_timestamp_us {
            if timestamp_us >= prev {
                let interval_us = (timestamp_us - prev) as f64;
                if interval_us > 0.0 {
                    // Compute instantaneous BPM
                    self.current_bpm = 2_500_000.0 / interval_us;

                    // If target BPM is provided, record deviation against nominal interval
                    if let Some(target) = self.target_bpm {
                        let nominal_us = Self::nominal_interval_us(target);
                        let jitter = interval_us - nominal_us;
                        self.interval_stats.record(jitter);
                        self.bpm_drift = self.current_bpm - target;
                    } else {
                        // Deviation against running average
                        if self.interval_stats.count > 0 {
                            let jitter = interval_us - self.interval_stats.mean_us;
                            self.interval_stats.record(jitter);
                        } else {
                            self.interval_stats.record(0.0);
                        }
                        if self.interval_stats.mean_us > 0.0 {
                            self.average_bpm = 2_500_000.0 / self.interval_stats.mean_us;
                        }
                    }

                    // Update moving average BPM
                    if self.target_bpm.is_some() && self.interval_stats.mean_us > 0.0 {
                        // Mean jitter was added to target interval
                        let target = self.target_bpm.unwrap_or(120.0);
                        let nominal_us = Self::nominal_interval_us(target);
                        let avg_interval = nominal_us + self.interval_stats.mean_us;
                        if avg_interval > 0.0 {
                            self.average_bpm = 2_500_000.0 / avg_interval;
                        }
                    }
                }
            } else {
                // Out of sequence or retrograde timestamp
                self.interval_stats.record_dropped(1);
            }
        }

        self.last_tick_timestamp_us = Some(timestamp_us);
    }

    /// Resets profiler state.
    pub fn reset(&mut self) {
        self.interval_stats = JitterStats::new();
        self.current_bpm = 0.0;
        self.average_bpm = 0.0;
        self.bpm_drift = 0.0;
        self.total_ticks = 0;
        self.last_tick_timestamp_us = None;
        self.beat_tick_counter = 0;
    }
}

//! Network MIDI 2.0 / AppleMIDI Clock Synchronization & Latency Profiler.
//!
//! Based on RFC 6295 / AppleMIDI and MIDI 2.0 Network Transport specifications.
//!
//! Evaluates 3-way/4-way clock synchronization packets (CK0, CK1, CK2):
//!
//! Formula:
//! - Offset = ((t1 + t3) / 2) - t2
//! - Round-Trip Time (RTT) = (t3 - t0) - (t2 - t1)
//!   (where in standard 3-way synchronization, t0 is t1, so RTT simplifies to (t3 - t1) - (t2 - t1) = t3 - t2,
//!   or when sender turnaround (t2 - t1) is subtracted from total elapsed roundtrip (t3 - t0)).
//!
//! Timestamps in AppleMIDI are 64-bit with 100-microsecond resolution (10 kHz, 0.1 ms per unit).

use crate::jitter::JitterStats;

/// 1 AppleMIDI timestamp unit = 100 microseconds.
pub const NETWORK_TIMESTAMP_UNIT_US: f64 = 100.0;

/// Snapshot of a single clock sync exchange.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockSyncMeasurement {
    pub ssrc: u32,
    pub count: u8,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub t3: u64,
    pub offset_units: i64,
    pub rtt_units: u64,
    pub offset_us: f64,
    pub rtt_us: f64,
}

/// Network MIDI 2.0 / AppleMIDI Clock Synchronization & Jitter Profiler.
#[derive(Debug, Clone)]
pub struct NetworkClockProfiler {
    pub local_ssrc: u32,
    pub remote_ssrc: Option<u32>,
    /// Last measured clock offset in units
    pub last_offset_units: i64,
    /// Last measured round-trip time in units
    pub last_rtt_units: u64,
    /// Count of completed sync cycles
    pub sync_cycles: u64,
    /// Jitter statistics across round-trip times (in microseconds)
    pub rtt_stats: JitterStats,
    /// Offset jitter statistics (deviations from running mean offset, in microseconds)
    pub offset_stats: JitterStats,
    /// Sequence tracking for dropped or out-of-order sync packets
    pub dropped_sync_packets: u64,
    /// Last measurement recorded
    pub last_measurement: Option<ClockSyncMeasurement>,
}

impl Default for NetworkClockProfiler {
    fn default() -> Self {
        Self::new(0)
    }
}

impl NetworkClockProfiler {
    /// Creates a new `NetworkClockProfiler` with the given local SSRC.
    #[must_use]
    pub const fn new(local_ssrc: u32) -> Self {
        Self {
            local_ssrc,
            remote_ssrc: None,
            last_offset_units: 0,
            last_rtt_units: 0,
            sync_cycles: 0,
            rtt_stats: JitterStats::new(),
            offset_stats: JitterStats::new(),
            dropped_sync_packets: 0,
            last_measurement: None,
        }
    }

    /// Evaluates timestamps from a 4-timestamp exchange (t0, t1, t2, t3):
    /// - `t0`: Initiator transmit time (local clock)
    /// - `t1`: Responder receive time (remote clock)
    /// - `t2`: Responder reply transmit time (remote clock)
    /// - `t3`: Initiator reply receive time (local clock)
    ///
    /// Offset = ((t1 + t3) / 2) - t2
    /// RTT = (t3 - t0) - (t2 - t1)
    pub fn record_exchange_4ts(
        &mut self,
        ssrc: u32,
        t0: u64,
        t1: u64,
        t2: u64,
        t3: u64,
    ) -> ClockSyncMeasurement {
        self.remote_ssrc = Some(ssrc);

        let avg_initiator_time = ((t1 as i128) + (t3 as i128)) / 2;
        let offset = avg_initiator_time - (t2 as i128);
        let offset_units = offset as i64;

        let total_elapsed = t3.saturating_sub(t0);
        let remote_dwell = t2.saturating_sub(t1);
        let rtt_units = total_elapsed.saturating_sub(remote_dwell);

        let offset_us = (offset_units as f64) * NETWORK_TIMESTAMP_UNIT_US;
        let rtt_us = (rtt_units as f64) * NETWORK_TIMESTAMP_UNIT_US;

        self.last_offset_units = offset_units;
        self.last_rtt_units = rtt_units;
        self.sync_cycles += 1;

        self.rtt_stats.record(rtt_us);

        // Record offset deviation from running mean
        let offset_deviation = if self.offset_stats.count > 0 {
            offset_us - self.offset_stats.mean_us
        } else {
            0.0
        };
        self.offset_stats.record(offset_deviation);

        let measurement = ClockSyncMeasurement {
            ssrc,
            count: 3,
            t0,
            t1,
            t2,
            t3,
            offset_units,
            rtt_units,
            offset_us,
            rtt_us,
        };

        self.last_measurement = Some(measurement);
        measurement
    }

    /// Evaluates a standard 3-timestamp AppleMIDI exchange (CK0 -> CK1 -> CK2):
    /// - `t1`: Initiator timestamp when CK0 is sent
    /// - `t2`: Responder timestamp when CK1 is replied
    /// - `t3`: Initiator local time when CK1 is received
    ///
    /// Here t0 = t1, so RTT = (t3 - t1) - (t2 - t1) = t3 - t2 (or standard network round trip t3 - t1).
    pub fn record_exchange_3ts(
        &mut self,
        ssrc: u32,
        t1: u64,
        t2: u64,
        t3: u64,
    ) -> ClockSyncMeasurement {
        self.record_exchange_4ts(ssrc, t1, t1, t2, t3)
    }

    /// Ingests a raw serialized AppleMIDI packet if it is a Clock Sync (`CK`) packet.
    ///
    /// Format:
    /// - 0..2: 0xFFFF
    /// - 2..4: 0x434B ('CK')
    /// - 4..8: SSRC
    /// - 8: Count (0, 1, or 2)
    /// - 9..12: Reserved
    /// - 12..20: Timestamp 1
    /// - 20..28: Timestamp 2
    /// - 28..36: Timestamp 3
    pub fn ingest_packet(
        &mut self,
        packet: &[u8],
        receive_timestamp: u64,
    ) -> Option<ClockSyncMeasurement> {
        if packet.len() < 36 {
            return None;
        }

        let sig = u16::from_be_bytes([packet[0], packet[1]]);
        let cmd = u16::from_be_bytes([packet[2], packet[3]]);
        if sig != 0xFFFF || cmd != 0x434B {
            return None;
        }

        let ssrc = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
        let count = packet[8];
        let t1 = u64::from_be_bytes([
            packet[12], packet[13], packet[14], packet[15], packet[16], packet[17], packet[18],
            packet[19],
        ]);
        let t2 = u64::from_be_bytes([
            packet[20], packet[21], packet[22], packet[23], packet[24], packet[25], packet[26],
            packet[27],
        ]);
        let t3 = u64::from_be_bytes([
            packet[28], packet[29], packet[30], packet[31], packet[32], packet[33], packet[34],
            packet[35],
        ]);

        match count {
            0 => {
                // CK0 received: Responder local reception
                None
            }
            1 => {
                // CK1 received: Initiator local reception, t3 is receive_timestamp
                Some(self.record_exchange_3ts(ssrc, t1, t2, receive_timestamp))
            }
            2 => {
                // CK2 received: Responder processes final t1, t2, t3
                Some(self.record_exchange_3ts(ssrc, t1, t2, t3))
            }
            _ => {
                self.dropped_sync_packets += 1;
                None
            }
        }
    }
}

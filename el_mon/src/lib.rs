//! Real-Time Hardware Jitter Analyzer & Clock Offset Profiler (`el_mon`).
//!
//! Provides audio-rate jitter analysis, network clock synchronization measurement,
//! and MIDI clock drift profiling across physical and network transports.

pub mod jitter;
pub mod midi_clock;
pub mod network_clock;
pub mod stats;

pub use jitter::{JitterAnalyzer, JitterStats, JR_TICK_US};
pub use midi_clock::MidiClockProfiler;
pub use network_clock::{ClockSyncMeasurement, NetworkClockProfiler, NETWORK_TIMESTAMP_UNIT_US};
pub use stats::{
    format_bpm_status, format_jitter_block, format_network_sync, render_live_status, AnsiColors,
};

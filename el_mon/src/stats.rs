//! ANSI Terminal Live Statistics & Formatter for `el_mon`.

use crate::jitter::JitterStats;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

/// Terminal color codes for ANSI rendering.
pub struct AnsiColors;

impl AnsiColors {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const RED: &'static str = "\x1b[31m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const CLEAR_LINE: &'static str = "\x1b[2K\r";
}

/// Formats a jitter statistics block into an ANSI string.
pub fn format_jitter_block(label: &str, stats: &JitterStats) -> String {
    let mut out = String::new();
    let min = if stats.count > 0 { stats.min_us } else { 0.0 };
    let std_dev = stats.std_dev_us();

    let jitter_color = if stats.mean_us < 50.0 {
        AnsiColors::GREEN
    } else if stats.mean_us < 200.0 {
        AnsiColors::YELLOW
    } else {
        AnsiColors::RED
    };

    let _ = writeln!(
        out,
        "{}{}[{}] {}{}Samples: {} | Min: {:.2} µs | Max: {:.2} µs | Avg: {:.2} µs | StdDev: {:.2} µs | Dropped: {}{}",
        AnsiColors::BOLD,
        AnsiColors::CYAN,
        label,
        jitter_color,
        AnsiColors::BOLD,
        stats.count,
        min,
        stats.max_us,
        stats.mean_us,
        std_dev,
        stats.dropped_packets,
        AnsiColors::RESET
    );
    out
}

/// Formats MIDI Timing Clock BPM and drift status.
pub fn format_bpm_status(
    current_bpm: f64,
    average_bpm: f64,
    target_bpm: Option<f64>,
    drift: f64,
) -> String {
    let mut out = String::new();
    let drift_color = if drift.abs() < 0.1 {
        AnsiColors::GREEN
    } else if drift.abs() < 1.0 {
        AnsiColors::YELLOW
    } else {
        AnsiColors::RED
    };

    let target_str = match target_bpm {
        Some(t) => format!("{:.2}", t),
        None => "N/A".to_string(),
    };

    let _ = write!(
        out,
        "{}[MIDI CLOCK] {}Instant BPM: {:.2} | Avg BPM: {:.2} | Target: {} | Drift: {}{:.3} BPM{}",
        AnsiColors::CYAN,
        AnsiColors::BOLD,
        current_bpm,
        average_bpm,
        target_str,
        drift_color,
        drift,
        AnsiColors::RESET
    );
    out
}

/// Formats an AppleMIDI / Network clock sync measurement report.
pub fn format_network_sync(ssrc: u32, offset_us: f64, rtt_us: f64, sync_count: u64) -> String {
    let mut out = String::new();
    let rtt_color = if rtt_us < 1_000.0 {
        AnsiColors::GREEN
    } else if rtt_us < 10_000.0 {
        AnsiColors::YELLOW
    } else {
        AnsiColors::RED
    };

    let _ = write!(
        out,
        "{}[NET SYNC] {}SSRC: 0x{:08X} | Cycles: {} | Offset: {:.2} µs | RTT: {}{:.2} µs{}",
        AnsiColors::CYAN,
        AnsiColors::BOLD,
        ssrc,
        sync_count,
        offset_us,
        rtt_color,
        rtt_us,
        AnsiColors::RESET
    );
    out
}

/// Writes live single-line status updating terminal line in place.
pub fn render_live_status<W: Write>(writer: &mut W, status: &str) -> io::Result<()> {
    write!(writer, "{}{}", AnsiColors::CLEAR_LINE, status)?;
    writer.flush()
}

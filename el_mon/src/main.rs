//! `el_mon`: Real-Time Hardware Jitter Analyzer & Clock Offset Profiler CLI.

use clap::{Parser, Subcommand};
use el_core::parser::UmpStreamParser;
use el_mon::{
    format_bpm_status, format_jitter_block, format_network_sync, render_live_status, AnsiColors,
    JitterAnalyzer, MidiClockProfiler, NetworkClockProfiler,
};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    name = "el_mon",
    author,
    version,
    about = "el_mon: Real-Time Hardware Jitter Analyzer & Clock Offset Profiler CLI for MIDI 2.0",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to a binary .ump file to inspect and profile. If omitted and no subcommand, reads from stdin.
    #[arg(short, long)]
    file: Option<String>,

    /// Target nominal BPM to measure clock drift against (defaults to 120.0 BPM).
    #[arg(short, long, default_value_t = 120.0)]
    bpm: f64,

    /// Nominal inter-packet interval in microseconds (e.g., 1000.0 for 1ms periodic events).
    #[arg(short, long)]
    interval_us: Option<f64>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Profile raw UMP stream from stdin or file
    Ump {
        /// Filepath to .ump binary file (reads stdin if omitted)
        #[arg(short, long)]
        file: Option<String>,
        /// Target nominal BPM
        #[arg(short, long, default_value_t = 120.0)]
        bpm: f64,
        /// Nominal inter-packet interval in microseconds
        #[arg(short, long)]
        interval_us: Option<f64>,
    },
    /// Profile simulated or network AppleMIDI clock synchronization
    Network {
        /// Local SSRC identifier
        #[arg(short, long, default_value_t = 0x12345678)]
        ssrc: u32,
        /// Number of test synchronization cycles to simulate/run
        #[arg(short, long, default_value_t = 100)]
        cycles: u64,
        /// Simulated base one-way network latency in microseconds
        #[arg(short, long, default_value_t = 1500.0)]
        latency_us: f64,
        /// Simulated random jitter amplitude in microseconds
        #[arg(short, long, default_value_t = 250.0)]
        jitter_us: f64,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Network {
            ssrc,
            cycles,
            latency_us,
            jitter_us,
        }) => run_network_profiler(ssrc, cycles, latency_us, jitter_us),
        Some(Commands::Ump {
            file,
            bpm,
            interval_us,
        }) => run_ump_profiler(file, bpm, interval_us),
        None => run_ump_profiler(cli.file, cli.bpm, cli.interval_us),
    }
}

fn run_network_profiler(ssrc: u32, cycles: u64, latency_us: f64, jitter_us: f64) -> io::Result<()> {
    println!(
        "{}=== el_mon: Network MIDI 2.0 / AppleMIDI Clock Profiler ==={}",
        AnsiColors::BOLD,
        AnsiColors::RESET
    );
    println!(
        "Local SSRC: 0x{:08X} | Simulating {} cycles | Base Latency: {:.1} µs | Jitter: ±{:.1} µs\n",
        ssrc, cycles, latency_us, jitter_us
    );

    let mut profiler = NetworkClockProfiler::new(ssrc);
    let remote_ssrc = 0x87654321;

    // Simulate Network Clock Sync exchanges (CK0 -> CK1 -> CK2)
    // 1 unit = 100 microseconds (10 kHz)
    let unit_scale = 100.0;
    let base_offset_us = 5_000.0; // 5 ms remote clock ahead

    let mut local_clock_us = 1_000_000.0;
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for i in 1..=cycles {
        // Pseudo-random pseudo-deterministic jitter based on LCG
        let seed = (i.wrapping_mul(1103515245).wrapping_add(12345)) % 1000;
        let jitter_factor = ((seed as f64) / 500.0) - 1.0; // -1.0 .. +1.0
        let current_jitter = jitter_factor * jitter_us;

        let one_way_us = (latency_us + current_jitter).max(10.0);

        // t0/t1: Initiator transmit time
        let t0_us = local_clock_us;
        let t1_unit = (t0_us / unit_scale) as u64;

        // Remote arrival time: t1 + one_way_us + remote_offset
        let remote_arrival_us = t0_us + one_way_us + base_offset_us;
        // Dwell time in remote node before replying: e.g. 50 µs
        let remote_dwell_us = 50.0;
        let remote_tx_us = remote_arrival_us + remote_dwell_us;
        let t2_unit = (remote_tx_us / unit_scale) as u64;

        // Return path one-way
        let return_one_way_us = (latency_us - (current_jitter * 0.5)).max(10.0);
        let t3_us = t0_us + one_way_us + remote_dwell_us + return_one_way_us;
        let t3_unit = (t3_us / unit_scale) as u64;

        let measurement =
            profiler.record_exchange_4ts(remote_ssrc, t1_unit, t1_unit, t2_unit, t3_unit);

        if i.is_multiple_of(10) || i == cycles {
            let status = format_network_sync(
                measurement.ssrc,
                measurement.offset_us,
                measurement.rtt_us,
                profiler.sync_cycles,
            );
            render_live_status(&mut handle, &status)?;
        }

        local_clock_us += 100_000.0; // Advance local clock by 100 ms between pings
    }

    println!(
        "\n\n{}--- Final Network Synchronization Statistics ---{}",
        AnsiColors::BOLD,
        AnsiColors::RESET
    );
    print!(
        "{}",
        format_jitter_block("Round-Trip Time (RTT)", &profiler.rtt_stats)
    );
    print!(
        "{}",
        format_jitter_block("Clock Offset Drift", &profiler.offset_stats)
    );
    println!("Total Completed Sync Cycles: {}", profiler.sync_cycles);
    println!("Dropped Sync Packets: {}", profiler.dropped_sync_packets);

    Ok(())
}

fn run_ump_profiler(file: Option<String>, bpm: f64, interval_us: Option<f64>) -> io::Result<()> {
    println!(
        "{}=== el_mon: Real-Time Hardware Jitter Analyzer & Clock Offset Profiler ==={}",
        AnsiColors::BOLD,
        AnsiColors::RESET
    );
    if let Some(ref path) = file {
        println!("Source: File '{}'", path);
    } else {
        println!("Source: Stdin");
    }
    println!("Target Nominal BPM: {:.2}", bpm);
    if let Some(target_interval) = interval_us {
        println!("Nominal Packet Interval: {:.1} µs", target_interval);
    }
    println!();

    let mut reader: Box<dyn BufRead> = if let Some(filepath) = file {
        Box::new(BufReader::with_capacity(64 * 1024, File::open(filepath)?))
    } else {
        Box::new(BufReader::with_capacity(64 * 1024, io::stdin()))
    };

    let mut jitter_analyzer = JitterAnalyzer::new(interval_us);
    let mut midi_clock = MidiClockProfiler::new(Some(bpm));

    let start_time = Instant::now();
    let mut packet_count = 0u64;

    let word_iter = std::iter::from_fn(move || {
        let mut buf = [0u8; 4];

        match reader.fill_buf() {
            Ok([]) => return None,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                return None;
            }
        }

        match reader.read_exact(&mut buf) {
            Ok(_) => Some(u32::from_le_bytes(buf)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                None
            }
        }
    });

    let parser = UmpStreamParser::new(word_iter);
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for ump in parser {
        packet_count += 1;
        let now_us = start_time.elapsed().as_micros() as u64;

        // Ingest into Jitter Reduction / DCT Analyzer
        jitter_analyzer.ingest_ump(&ump);

        // Ingest into MIDI Clock (0xF8) Profiler
        midi_clock.ingest_ump(&ump, now_us);

        // Update live status periodically
        if packet_count.is_multiple_of(50) {
            let clock_status = format_bpm_status(
                midi_clock.current_bpm,
                midi_clock.average_bpm,
                midi_clock.target_bpm,
                midi_clock.bpm_drift,
            );
            render_live_status(&mut handle, &clock_status)?;
        }
    }

    println!(
        "\n\n{}--- Final Analysis Summary ---{}",
        AnsiColors::BOLD,
        AnsiColors::RESET
    );
    println!("Total Universal MIDI Packets Processed: {}", packet_count);
    println!("JR Clock Messages: {}", jitter_analyzer.jr_clock_count);
    println!(
        "JR Timestamp Messages: {}",
        jitter_analyzer.jr_timestamp_count
    );
    println!("Delta Clockstamp Messages: {}", jitter_analyzer.dct_count);
    println!("MIDI Timing Clock Ticks: {}", midi_clock.total_ticks);
    println!();

    print!(
        "{}",
        format_jitter_block("Jitter Reduction Timing", &jitter_analyzer.stats)
    );
    print!(
        "{}",
        format_jitter_block("MIDI Clock Tick Intervals", &midi_clock.interval_stats)
    );

    if midi_clock.total_ticks > 0 {
        println!(
            "{}[MIDI CLOCK SUMMARY]{} Measured BPM: {:.2} | Target BPM: {:.2} | Final Drift: {:.3} BPM",
            AnsiColors::CYAN,
            AnsiColors::RESET,
            if midi_clock.average_bpm > 0.0 { midi_clock.average_bpm } else { midi_clock.current_bpm },
            bpm,
            midi_clock.bpm_drift
        );
    }

    Ok(())
}

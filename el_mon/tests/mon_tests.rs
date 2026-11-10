use el_core::builder::{SystemBuilder, UtilityBuilder};
use el_mon::{
    format_bpm_status, format_jitter_block, format_network_sync, JitterAnalyzer, JitterStats,
    MidiClockProfiler, NetworkClockProfiler, NETWORK_TIMESTAMP_UNIT_US,
};

#[test]
fn test_jitter_stats_welford_algorithm() {
    let mut stats = JitterStats::new();
    assert_eq!(stats.count, 0);
    assert_eq!(stats.std_dev_us(), 0.0);

    stats.record(10.0);
    stats.record(20.0);
    stats.record(30.0);

    assert_eq!(stats.count, 3);
    assert_eq!(stats.min_us, 10.0);
    assert_eq!(stats.max_us, 30.0);
    assert_eq!(stats.mean_us, 20.0);
    assert!((stats.std_dev_us() - 10.0).abs() < 1e-6);

    let output = format_jitter_block("Test Jitter", &stats);
    assert!(output.contains("Test Jitter"));
    assert!(output.contains("Samples: 3"));
}

#[test]
fn test_jitter_analyzer_jr_clock_and_timestamps() {
    let mut analyzer = JitterAnalyzer::new(Some(320.0)); // 10 ticks = 320 µs expected

    let jr_clock = UtilityBuilder::jitter_reduction_clock(0, 100);
    assert!(analyzer.ingest_ump(&jr_clock));
    assert_eq!(analyzer.jr_clock_count, 1);
    assert_eq!(analyzer.last_jr_clock, Some(100));

    // First timestamp
    let ts1 = UtilityBuilder::jitter_reduction_timestamp(0, 1000);
    assert!(analyzer.ingest_ump(&ts1));
    assert_eq!(analyzer.jr_timestamp_count, 1);

    // Second timestamp: exactly 10 ticks later (1010)
    let ts2 = UtilityBuilder::jitter_reduction_timestamp(0, 1010);
    assert!(analyzer.ingest_ump(&ts2));
    assert_eq!(analyzer.jr_timestamp_count, 2);

    // 10 ticks * 32.0 us = 320.0 us -> exactly 0 jitter
    assert_eq!(analyzer.stats.min_us, 0.0);
    assert_eq!(analyzer.stats.max_us, 0.0);

    // Third timestamp: 12 ticks later (1022) -> 384.0 us -> +64.0 us jitter
    let ts3 = UtilityBuilder::jitter_reduction_timestamp(0, 1022);
    assert!(analyzer.ingest_ump(&ts3));
    assert!((analyzer.stats.max_us - 64.0).abs() < 1e-4);
}

#[test]
fn test_jitter_analyzer_dct_since_last() {
    let mut analyzer = JitterAnalyzer::new(Some(640.0)); // 20 ticks expected = 640 us

    let dct_ticks = UtilityBuilder::delta_clockstamp_ticks(960);
    assert!(analyzer.ingest_ump(&dct_ticks));
    assert_eq!(analyzer.dct_ticks_per_qn, Some(960));

    let dct_since = UtilityBuilder::delta_clockstamp_since(20);
    assert!(analyzer.ingest_ump(&dct_since));
    assert_eq!(analyzer.last_dct_since_last, Some(20));
    assert_eq!(analyzer.stats.min_us, 0.0);

    // Another DCT with 25 ticks -> 800 us -> 160 us jitter
    let dct_since_jitter = UtilityBuilder::delta_clockstamp_since(25);
    assert!(analyzer.ingest_ump(&dct_since_jitter));
    assert!((analyzer.stats.max_us - 160.0).abs() < 1e-4);
}

#[test]
fn test_network_clock_profiler_offset_and_rtt() {
    let mut profiler = NetworkClockProfiler::new(0x1111);

    // In AppleMIDI:
    // t0: Initiator local send time
    // t1: Responder local receive time
    // t2: Responder local reply time
    // t3: Initiator local receive time
    // Offset = ((t1 + t3) / 2) - t2
    // RTT = (t3 - t0) - (t2 - t1)

    let t0 = 1000;
    let t1 = 1520; // 1000 + 20 one-way + 500 offset
    let t2 = 1530; // 10 units dwell time
    let t3 = 1050; // 1000 + 20 one-way + 10 dwell + 20 return one-way

    let meas = profiler.record_exchange_4ts(0x2222, t0, t1, t2, t3);

    // RTT = (t3 - t0) - (t2 - t1) = (1050 - 1000) - (1530 - 1520) = 50 - 10 = 40 units!
    assert_eq!(meas.rtt_units, 40);
    assert_eq!(meas.rtt_us, 40.0 * NETWORK_TIMESTAMP_UNIT_US);

    let expected_offset = (((1520 + 1050) / 2) as i64) - 1530;
    assert_eq!(meas.offset_units, expected_offset);

    assert_eq!(profiler.sync_cycles, 1);
    assert_eq!(profiler.rtt_stats.count, 1);

    let net_str = format_network_sync(meas.ssrc, meas.offset_us, meas.rtt_us, profiler.sync_cycles);
    assert!(net_str.contains("0x00002222"));
    assert!(net_str.contains("Cycles: 1"));
}

#[test]
fn test_network_clock_raw_packet_ingest() {
    let mut profiler = NetworkClockProfiler::new(0x1111);

    // Build raw AppleMIDI CK packet
    let mut packet = [0u8; 36];
    packet[0..2].copy_from_slice(&0xFFFFu16.to_be_bytes());
    packet[2..4].copy_from_slice(&0x434Bu16.to_be_bytes()); // 'CK'
    packet[4..8].copy_from_slice(&0x5555u32.to_be_bytes()); // SSRC
    packet[8] = 1; // Count = 1 (CK1 packet)
    packet[12..20].copy_from_slice(&1000u64.to_be_bytes()); // t1
    packet[20..28].copy_from_slice(&1200u64.to_be_bytes()); // t2
    packet[28..36].copy_from_slice(&0u64.to_be_bytes()); // t3 not filled by responder

    let meas = profiler.ingest_packet(&packet, 1050);
    assert!(meas.is_some());
    let m = meas.unwrap();
    assert_eq!(m.ssrc, 0x5555);
    assert_eq!(m.t1, 1000);
    assert_eq!(m.t2, 1200);
    assert_eq!(m.t3, 1050);
}

#[test]
fn test_midi_clock_profiler_and_bpm_drift() {
    let mut profiler = MidiClockProfiler::new(Some(120.0));
    // At 120 BPM:
    // interval = 2,500,000 / 120 = 20,833.33 µs
    let nominal_us = MidiClockProfiler::nominal_interval_us(120.0);
    assert!((nominal_us - 20833.333).abs() < 1e-2);

    let clock_ump = SystemBuilder::timing_clock(0);

    // Record first tick at t = 0
    assert!(profiler.ingest_ump(&clock_ump, 0));
    assert_eq!(profiler.total_ticks, 1);

    // Record second tick at t = 20,833 µs (exact nominal)
    let t1 = 20833u64;
    assert!(profiler.ingest_ump(&clock_ump, t1));
    assert_eq!(profiler.total_ticks, 2);
    assert!((profiler.current_bpm - 120.0).abs() < 0.1);
    assert!(profiler.bpm_drift.abs() < 0.1);

    // Record third tick with slight timing drift (at 125 BPM rate: 2,500,000 / 125 = 20,000 µs interval)
    let t2 = t1 + 20000;
    assert!(profiler.ingest_ump(&clock_ump, t2));
    assert_eq!(profiler.total_ticks, 3);
    assert!((profiler.current_bpm - 125.0).abs() < 0.1);
    assert!((profiler.bpm_drift - 5.0).abs() < 0.1);

    let status = format_bpm_status(
        profiler.current_bpm,
        profiler.average_bpm,
        profiler.target_bpm,
        profiler.bpm_drift,
    );
    assert!(status.contains("Instant BPM"));
    assert!(status.contains("Target: 120.00"));
}

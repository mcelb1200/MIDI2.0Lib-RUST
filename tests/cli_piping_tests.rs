use std::io::Write;
use std::process::{Command, Stdio};

fn get_bin_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("current exe");
    path.pop(); // remove test binary name
    if path.ends_with("deps") {
        path.pop(); // remove "deps"
    }
    path.push(name);
    if !path.exists() {
        // Fall back to target/debug/<name>
        let debug_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join(name);
        if debug_path.exists() {
            return debug_path;
        }
    }
    path
}

#[test]
fn test_cli_pipe_el_gen_to_el_dump() {
    let el_gen_bin = get_bin_path("el_gen");
    let el_dump_bin = get_bin_path("el_dump");

    // Spawn el_gen note-on command
    let mut gen_proc = Command::new(&el_gen_bin)
        .args([
            "note-on",
            "--group",
            "1",
            "--channel",
            "2",
            "--note",
            "64",
            "--velocity",
            "32768",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", el_gen_bin.display(), e));

    let gen_stdout = gen_proc.stdout.take().expect("Failed to open gen stdout");

    // Spawn el_dump reading from el_gen's stdout
    let dump_proc = Command::new(&el_dump_bin)
        .stdin(gen_stdout)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", el_dump_bin.display(), e));

    let dump_output = dump_proc
        .wait_with_output()
        .expect("Failed to wait on el_dump");
    let gen_status = gen_proc.wait().expect("Failed to wait on el_gen");

    assert!(gen_status.success());
    assert!(dump_output.status.success());

    let dump_str = String::from_utf8_lossy(&dump_output.stdout);
    assert!(
        dump_str.contains("--- el_dump: UMP Stream Analyzer ---"),
        "Output missing header: {}",
        dump_str
    );
    assert!(
        dump_str.contains("MT: Midi2ChannelVoice"),
        "Output missing Midi2ChannelVoice: {}",
        dump_str
    );
    assert!(
        dump_str.contains("Grp:  1"),
        "Output missing Grp 1: {}",
        dump_str
    );
}

#[test]
fn test_cli_pipe_el_gen_to_el_mon() {
    let el_gen_bin = get_bin_path("el_gen");
    let el_mon_bin = get_bin_path("el_mon");

    // Spawn el_gen note-on command
    let mut gen_proc = Command::new(&el_gen_bin)
        .args([
            "note-on",
            "--group",
            "0",
            "--channel",
            "0",
            "--note",
            "60",
            "--velocity",
            "1000",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", el_gen_bin.display(), e));

    let gen_stdout = gen_proc.stdout.take().expect("Failed to open gen stdout");

    // Spawn el_mon reading from stdin
    let mon_proc = Command::new(&el_mon_bin)
        .stdin(gen_stdout)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", el_mon_bin.display(), e));

    let mon_output = mon_proc
        .wait_with_output()
        .expect("Failed to wait on el_mon");
    let gen_status = gen_proc.wait().expect("Failed to wait on el_gen");

    assert!(gen_status.success());
    assert!(mon_output.status.success());

    let mon_str = String::from_utf8_lossy(&mon_output.stdout);
    assert!(
        mon_str.contains("el_mon: Real-Time Hardware Jitter Analyzer"),
        "Output missing el_mon banner: {}",
        mon_str
    );
    assert!(
        mon_str.contains("Total Universal MIDI Packets Processed: 1"),
        "Output missing processed packet count: {}",
        mon_str
    );
}

#[test]
fn test_cli_pipe_stdin_direct_to_el_dump() {
    let el_dump_bin = get_bin_path("el_dump");

    // Spawn el_dump reading directly from test process stdin pipe
    let mut dump_proc = Command::new(&el_dump_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", el_dump_bin.display(), e));

    let mut dump_stdin = dump_proc.stdin.take().expect("Failed to open dump stdin");

    // Send 32-bit word in Little-Endian: 0x20903C64 (MIDI 1.0 Note On)
    let word: u32 = 0x20903C64;
    dump_stdin
        .write_all(&word.to_le_bytes())
        .expect("write to dump stdin");
    drop(dump_stdin); // Close stdin to signal EOF

    let output = dump_proc.wait_with_output().expect("wait on dump process");
    assert!(output.status.success());

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_str.contains("MT: Midi1ChannelVoice"),
        "Missing Midi1ChannelVoice in output: {}",
        stdout_str
    );
    assert!(
        stdout_str.contains("20903C64"),
        "Missing hex word in output: {}",
        stdout_str
    );
}

use clap::{Parser, ValueEnum};
use el_bridge::backend::{connect_input, connect_output, list_input_ports, list_output_ports};
use el_bridge::queues::create_spsc_queue;
use el_bridge::translator::{ToMidi1Translator, ToUmpTranslator};
use el_core::ump::Ump;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "el_bridge: Bidirectional MIDI 1.0 <-> MIDI 2.0 UMP Protocol Translator"
)]
struct Cli {
    /// List available MIDI input and output ports
    #[arg(short, long)]
    list_ports: bool,

    /// Index of MIDI input port to listen on
    #[arg(short, long)]
    in_port: Option<usize>,

    /// Index of MIDI output port to send to
    #[arg(short, long)]
    out_port: Option<usize>,

    /// Translation direction
    #[arg(short, long, value_enum, default_value_t = Direction::Midi1ToUmp)]
    direction: Direction,

    /// Default UMP group (0-15)
    #[arg(short, long, default_value_t = 0)]
    group: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Direction {
    /// Convert legacy MIDI 1.0 bytes to MIDI 2.0 UMP
    Midi1ToUmp,
    /// Convert MIDI 2.0 UMP to legacy MIDI 1.0 bytes
    UmpToMidi1,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if cli.list_ports {
        println!("Available MIDI Input Ports:");
        for p in list_input_ports()? {
            println!("  [{}] {}", p.index, p.name);
        }
        println!("\nAvailable MIDI Output Ports:");
        for p in list_output_ports()? {
            println!("  [{}] {}", p.index, p.name);
        }
        return Ok(());
    }

    let in_idx = match cli.in_port {
        Some(idx) => idx,
        None => {
            eprintln!("Error: --in-port is required (or use --list-ports to see ports)");
            std::process::exit(1);
        }
    };

    let out_idx = match cli.out_port {
        Some(idx) => idx,
        None => {
            eprintln!("Error: --out-port is required (or use --list-ports to see ports)");
            std::process::exit(1);
        }
    };

    println!(
        "Starting el_bridge: InPort={}, OutPort={}, Direction={:?}, Group={}",
        in_idx, out_idx, cli.direction, cli.group
    );

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    let (mut byte_prod, mut byte_cons) = create_spsc_queue::<u8>(1024);
    let (mut ump_prod, mut ump_cons) = create_spsc_queue::<Ump>(512);

    match cli.direction {
        Direction::Midi1ToUmp => {
            let _conn_in = connect_input(in_idx, move |_ts, data, _| {
                for &b in data {
                    let _ = byte_prod.try_push(b);
                }
            })?;

            let mut translator = ToUmpTranslator::new(cli.group);

            let run_flag = running.clone();
            let handle = thread::spawn(move || {
                while run_flag.load(Ordering::Relaxed) {
                    let mut processed = false;
                    while let Some(b) = byte_cons.try_pop() {
                        processed = true;
                        translator.process_byte(b);
                        while let Some(ump) = translator.pop_ump() {
                            let _ = ump_prod.try_push(ump);
                        }
                    }
                    if !processed {
                        thread::sleep(Duration::from_micros(200));
                    }
                }
            });

            let mut conn_out = connect_output(out_idx)?;
            while running.load(Ordering::Relaxed) {
                while let Some(ump) = ump_cons.try_pop() {
                    let wc = ump.word_count();
                    let mut raw_bytes = [0_u8; 16];
                    for i in 0..wc {
                        let w = ump.data[i];
                        raw_bytes[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
                    }
                    let _ = conn_out.send(&raw_bytes[..wc * 4]);
                }
                thread::sleep(Duration::from_millis(1));
            }
            let _ = handle.join();
        }

        Direction::UmpToMidi1 => {
            let (mut out_byte_prod, mut out_byte_cons) = create_spsc_queue::<u8>(1024);

            let _conn_in = connect_input(in_idx, move |_ts, data, _| {
                let mut i = 0;
                while i + 4 <= data.len() {
                    let w = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
                    let ump = Ump::new(w, 0, 0, 0);
                    let _ = ump_prod.try_push(ump);
                    i += 4;
                }
            })?;

            let mut translator = ToMidi1Translator::new();
            translator.filter_group = Some(cli.group);

            let run_flag = running.clone();
            let handle = thread::spawn(move || {
                while run_flag.load(Ordering::Relaxed) {
                    let mut processed = false;
                    while let Some(ump) = ump_cons.try_pop() {
                        processed = true;
                        translator.process_ump(&ump);
                        while let Some(b) = translator.pop_byte() {
                            let _ = out_byte_prod.try_push(b);
                        }
                    }
                    if !processed {
                        thread::sleep(Duration::from_micros(200));
                    }
                }
            });

            let mut conn_out = connect_output(out_idx)?;
            let mut out_batch = [0_u8; 64];
            while running.load(Ordering::Relaxed) {
                let mut count = 0;
                while count < out_batch.len() {
                    if let Some(b) = out_byte_cons.try_pop() {
                        out_batch[count] = b;
                        count += 1;
                    } else {
                        break;
                    }
                }
                if count > 0 {
                    let _ = conn_out.send(&out_batch[..count]);
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            }
            let _ = handle.join();
        }
    }

    Ok(())
}

fn ctrlc_handler(running: Arc<AtomicBool>) {
    let _ = running;
}

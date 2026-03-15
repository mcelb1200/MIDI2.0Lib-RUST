use clap::{Parser, Subcommand};
use el_core::builder::{UtilityBuilder, VoiceBuilder};
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "el_gen: Generates Universal MIDI Packets (UMP) and writes to stdout."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a MIDI 2.0 Note On message
    NoteOn {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
        #[arg(short, long, default_value_t = 0)]
        channel: u8,
        #[arg(short, long, default_value_t = 60)]
        note: u8,
        #[arg(short, long, default_value_t = 0x8000)]
        velocity: u16,
    },
    /// Generate a MIDI 2.0 Note Off message
    NoteOff {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
        #[arg(short, long, default_value_t = 0)]
        channel: u8,
        #[arg(short, long, default_value_t = 60)]
        note: u8,
        #[arg(short, long, default_value_t = 0x0000)]
        velocity: u16,
    },
    /// Generate a MIDI 2.0 32-bit Control Change message
    Cc {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
        #[arg(short, long, default_value_t = 0)]
        channel: u8,
        #[arg(short, long, default_value_t = 1)]
        index: u8,
        #[arg(short, long, default_value_t = 0x7FFFFFFF)]
        value: u32,
    },
    /// Generate a Utility NOOP message
    Noop,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let ump = match cli.command {
        Commands::NoteOn {
            group,
            channel,
            note,
            velocity,
        } => VoiceBuilder::midi2_note_on(group, channel, note, 0, velocity, 0),
        Commands::NoteOff {
            group,
            channel,
            note,
            velocity,
        } => VoiceBuilder::midi2_note_off(group, channel, note, 0, velocity, 0),
        Commands::Cc {
            group,
            channel,
            index,
            value,
        } => VoiceBuilder::midi2_cc(group, channel, index, value),
        Commands::Noop => UtilityBuilder::noop(),
    };

    let word_count = ump.word_count();
    let mut stdout = io::stdout().lock();
    for i in 0..word_count {
        // Output strictly as network-endian (Little Endian for UMP) raw bytes
        let bytes = ump.data[i].to_le_bytes();
        stdout.write_all(&bytes)?;
    }
    stdout.flush()?;

    Ok(())
}

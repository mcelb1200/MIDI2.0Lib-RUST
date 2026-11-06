use clap::{Parser, Subcommand};
use el_core::builder::{UtilityBuilder, VoiceBuilder};
use std::io::{self, BufWriter, Write};

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
    /// Generate a sample MIDI Clip (.midiclip) file with Note On/Off sequence
    Clip {
        #[arg(short, long, default_value_t = 60)]
        note: u8,
        #[arg(short, long, default_value_t = 480)]
        ppqn: u16,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

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
        Commands::Clip { note, ppqn } => {
            let mut hdr = [0_u8; 14];
            hdr[0..4].copy_from_slice(b"SMF2");
            hdr[4..8].copy_from_slice(&6_u32.to_be_bytes());
            hdr[8..10].copy_from_slice(&0_u16.to_be_bytes());
            hdr[10..12].copy_from_slice(&1_u16.to_be_bytes());
            hdr[12..14].copy_from_slice(&ppqn.to_be_bytes());
            writer.write_all(&hdr)?;

            let on_ump = VoiceBuilder::midi2_note_on(0, 0, note, 0, 0x9000, 0);
            let off_ump = VoiceBuilder::midi2_note_off(0, 0, note, 0, 0x0000, 0);

            let mut track_buf = [0_u8; 256];
            let mut cursor = 8;

            track_buf[cursor] = 0x00;
            cursor += 1;
            track_buf[cursor..cursor + 4].copy_from_slice(&on_ump.data[0].to_be_bytes());
            cursor += 4;
            track_buf[cursor..cursor + 4].copy_from_slice(&on_ump.data[1].to_be_bytes());
            cursor += 4;

            let mut vlq_temp = [0_u8; 4];
            let mut vlq_len = 0;
            let mut v = ppqn as u32;
            vlq_temp[vlq_len] = (v & 0x7F) as u8;
            vlq_len += 1;
            v >>= 7;
            while v > 0 {
                vlq_temp[vlq_len] = ((v & 0x7F) as u8) | 0x80;
                vlq_len += 1;
                v >>= 7;
            }
            for i in (0..vlq_len).rev() {
                track_buf[cursor] = vlq_temp[i];
                cursor += 1;
            }

            track_buf[cursor..cursor + 4].copy_from_slice(&off_ump.data[0].to_be_bytes());
            cursor += 4;
            track_buf[cursor..cursor + 4].copy_from_slice(&off_ump.data[1].to_be_bytes());
            cursor += 4;

            let payload_len = (cursor - 8) as u32;
            track_buf[0..4].copy_from_slice(b"MTrk");
            track_buf[4..8].copy_from_slice(&payload_len.to_be_bytes());

            writer.write_all(&track_buf[..cursor])?;
            writer.flush()?;
            return Ok(());
        }
    };

    let word_count = ump.word_count();

    // ⚡ Bolt Optimization: Wrap stdout in a BufWriter.
    // Instead of making multiple small 4-byte system calls, this aggregates
    // the sequential writes into a single buffer, significantly improving I/O throughput.
    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);
    for i in 0..word_count {
        // Output strictly as network-endian (Little Endian for UMP) raw bytes
        let bytes = ump.data[i].to_le_bytes();
        writer.write_all(&bytes)?;
    }
    writer.flush()?;

    Ok(())
}

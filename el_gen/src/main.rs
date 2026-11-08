use clap::{Parser, Subcommand};
use el_core::builder::{SystemBuilder, UtilityBuilder, VoiceBuilder};
use el_core::ump::Ump;
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

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
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
    /// Generate a Timing Clock message (0xF8)
    Clock {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate a Start Sequence message (0xFA)
    Start {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate a Continue Sequence message (0xFB)
    Continue {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate a Stop Sequence message (0xFC)
    Stop {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate an Active Sensing message (0xFE)
    ActiveSensing {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate a System Reset message (0xFF)
    Reset {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate a Song Position Pointer message (0xF2)
    Spp {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
        #[arg(short, long, default_value_t = 0)]
        position: u16,
    },
    /// Generate a Song Select message (0xF3)
    SongSelect {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
        #[arg(short, long, default_value_t = 0)]
        song: u8,
    },
    /// Generate a Tune Request message (0xF6)
    TuneRequest {
        #[arg(short, long, default_value_t = 0)]
        group: u8,
    },
    /// Generate a sample MIDI Clip (.midiclip) file with Note On/Off sequence
    Clip {
        #[arg(short, long, default_value_t = 60)]
        note: u8,
        #[arg(short, long, default_value_t = 480)]
        ppqn: u16,
    },
}

pub fn generate_ump_for_command(cmd: &Commands) -> Option<Ump> {
    match cmd {
        Commands::NoteOn {
            group,
            channel,
            note,
            velocity,
        } => Some(VoiceBuilder::midi2_note_on(
            *group, *channel, *note, 0, *velocity, 0,
        )),
        Commands::NoteOff {
            group,
            channel,
            note,
            velocity,
        } => Some(VoiceBuilder::midi2_note_off(
            *group, *channel, *note, 0, *velocity, 0,
        )),
        Commands::Cc {
            group,
            channel,
            index,
            value,
        } => Some(VoiceBuilder::midi2_cc(*group, *channel, *index, *value)),
        Commands::Noop => Some(UtilityBuilder::noop()),
        Commands::Clock { group } => Some(SystemBuilder::timing_clock(*group)),
        Commands::Start { group } => Some(SystemBuilder::start(*group)),
        Commands::Continue { group } => Some(SystemBuilder::continue_seq(*group)),
        Commands::Stop { group } => Some(SystemBuilder::stop(*group)),
        Commands::ActiveSensing { group } => Some(SystemBuilder::active_sensing(*group)),
        Commands::Reset { group } => Some(SystemBuilder::system_reset(*group)),
        Commands::Spp { group, position } => {
            Some(SystemBuilder::song_position_pointer(*group, *position))
        }
        Commands::SongSelect { group, song } => Some(SystemBuilder::song_select(*group, *song)),
        Commands::TuneRequest { group } => Some(SystemBuilder::tune_request(*group)),
        Commands::Clip { .. } => None,
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    if let Commands::Clip { note, ppqn } = cli.command {
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

    let ump = generate_ump_for_command(&cli.command).unwrap();
    let word_count = ump.word_count();

    for i in 0..word_count {
        // Output strictly as network-endian (Little Endian for UMP) raw bytes
        let bytes = ump.data[i].to_le_bytes();
        writer.write_all(&bytes)?;
    }
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_el_gen_system_commands() {
        let clock = generate_ump_for_command(&Commands::Clock { group: 1 }).unwrap();
        assert_eq!(clock.data[0], 0x11F80000);

        let start = generate_ump_for_command(&Commands::Start { group: 2 }).unwrap();
        assert_eq!(start.data[0], 0x12FA0000);

        let stop = generate_ump_for_command(&Commands::Stop { group: 3 }).unwrap();
        assert_eq!(stop.data[0], 0x13FC0000);

        let spp = generate_ump_for_command(&Commands::Spp {
            group: 0,
            position: 128,
        })
        .unwrap();
        // position 128 = 0x80 -> lsb 0x00, msb 0x01
        assert_eq!(spp.data[0], 0x10F20001);
    }
}

use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

mod converter;
mod smf;
#[cfg(test)]
mod tests;

use converter::{convert_midiclip_to_smf, convert_smf_to_midiclip, ConverterConfig};
use smf::SmfFile;

/// el_clip_conv: Bidirectional Standard MIDI File 1.0 (SMF1) <-> MIDI Clip File (.midiclip, M2-116) Converter Tool.
#[derive(Parser, Debug)]
#[command(
    name = "el_clip_conv",
    author,
    version,
    about = "Bidirectional converter between Standard MIDI Files (.mid) and MIDI 2.0 MIDI Clip Files (.midiclip, M2-116 standard)."
)]
struct Args {
    /// Input file path (.mid or .midiclip)
    #[arg(short, long)]
    input: PathBuf,

    /// Output file path (.midiclip or .mid)
    #[arg(short, long)]
    output: PathBuf,

    /// Force conversion direction: to Standard MIDI File (.mid)
    #[arg(long, default_value_t = false)]
    to_smf: bool,

    /// Force conversion direction: to MIDI Clip File (.midiclip)
    #[arg(long, default_value_t = false)]
    to_clip: bool,

    /// When converting SMF to Clip: Up-convert Channel Voice events to MIDI 2.0 (MT=0x4) with high-resolution velocity & controller scaling
    #[arg(short, long, default_value_t = false)]
    midi2: bool,

    /// Universal MIDI Packet Group index (0-15) when converting SMF to Clip
    #[arg(short, long, default_value_t = 0)]
    group: u8,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    if args.group > 15 {
        eprintln!("Error: Group must be between 0 and 15 inclusive.");
        std::process::exit(1);
    }

    let input_bytes = fs::read(&args.input).map_err(|e| {
        eprintln!("Error reading input file {:?}: {}", args.input, e);
        e
    })?;

    // Determine direction: SMF -> Clip or Clip -> SMF
    let is_clip_input = if args.to_smf {
        true
    } else if args.to_clip {
        false
    } else if input_bytes.len() >= 4 && &input_bytes[0..4] == b"SMF2" {
        true
    } else if input_bytes.len() >= 4 && &input_bytes[0..4] == b"MThd" {
        false
    } else if let Some(ext) = args.input.extension().and_then(|s| s.to_str()) {
        ext.eq_ignore_ascii_case("midiclip")
    } else {
        false
    };

    let stdout = io::stdout().lock();
    let mut writer = io::BufWriter::new(stdout);

    if is_clip_input {
        // Convert .midiclip -> SMF1 (.mid)
        let smf = convert_midiclip_to_smf(&input_bytes).map_err(|e| {
            eprintln!("Error converting MIDI Clip to SMF: {:?}", e);
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Clip to SMF conversion error: {:?}", e),
            )
        })?;

        let smf_bytes = smf.to_bytes();
        fs::write(&args.output, &smf_bytes).map_err(|e| {
            eprintln!("Error writing output file {:?}: {}", args.output, e);
            e
        })?;

        writeln!(
            writer,
            "Successfully converted MIDI Clip {:?} -> SMF {:?} (Format {}, Tracks {}, Size {} bytes)",
            args.input,
            args.output,
            smf.header.format,
            smf.tracks.len(),
            smf_bytes.len()
        )?;
    } else {
        // Convert SMF1 (.mid) -> .midiclip
        let smf = SmfFile::parse(&input_bytes).map_err(|e| {
            eprintln!("Error parsing SMF1 file: {:?}", e);
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SMF parse error: {:?}", e),
            )
        })?;

        let config = ConverterConfig {
            group: args.group,
            midi2: args.midi2,
        };

        let clip_bytes = convert_smf_to_midiclip(&smf, &config).map_err(|e| {
            eprintln!("Error converting to MIDI Clip: {:?}", e);
            io::Error::other(format!("Conversion error: {:?}", e))
        })?;

        fs::write(&args.output, &clip_bytes).map_err(|e| {
            eprintln!("Error writing output file {:?}: {}", args.output, e);
            e
        })?;

        writeln!(
            writer,
            "Successfully converted SMF {:?} -> MIDI Clip {:?} (Format {}, Tracks {}, Size {} bytes, MIDI 2.0: {})",
            args.input,
            args.output,
            smf.header.format,
            smf.tracks.len(),
            clip_bytes.len(),
            args.midi2
        )?;
    }

    writer.flush()?;

    Ok(())
}

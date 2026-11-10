use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

mod converter;
mod smf;
#[cfg(test)]
mod tests;

use converter::{convert_smf_to_midiclip, ConverterConfig};
use smf::SmfFile;

/// el_clip_conv: Standard MIDI File 1.0 (SMF1) to MIDI Clip File (.midiclip) Converter Tool.
#[derive(Parser, Debug)]
#[command(
    name = "el_clip_conv",
    author,
    version,
    about = "Converts Standard MIDI Files (.mid) into MIDI 2.0 MIDI Clip Files (.midiclip, M2-116 standard)."
)]
struct Args {
    /// Input Standard MIDI File path (.mid)
    #[arg(short, long)]
    input: PathBuf,

    /// Output MIDI Clip File path (.midiclip)
    #[arg(short, long)]
    output: PathBuf,

    /// Up-convert Channel Voice events to MIDI 2.0 (MT=0x4) with high-resolution velocity & controller scaling
    #[arg(short, long, default_value_t = false)]
    midi2: bool,

    /// Universal MIDI Packet Group index (0-15)
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

    let stdout = io::stdout().lock();
    let mut writer = io::BufWriter::new(stdout);
    writeln!(
        writer,
        "Successfully converted {:?} -> {:?} (Format {}, Tracks {}, Size {} bytes, MIDI 2.0: {})",
        args.input,
        args.output,
        smf.header.format,
        smf.tracks.len(),
        clip_bytes.len(),
        args.midi2
    )?;
    writer.flush()?;

    Ok(())
}

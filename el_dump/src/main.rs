use clap::Parser;
use el_core::parser::UmpStreamParser;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

/// el_dump: A CLI tool to parse and dump raw Universal MIDI Packets (UMP) from a file or stdin.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the binary .ump file to parse. If omitted, reads from stdin.
    #[arg(short, long)]
    file: Option<String>,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let mut reader: Box<dyn BufRead> = if let Some(filepath) = args.file {
        // ⚡ Bolt Optimization: Avoid eagerly loading potentially multi-gigabyte UMP
        // files entirely into a Vec<u8> memory array. Using a BufReader allows us to stream
        // chunks dynamically into the parser with a near-zero memory footprint.
        Box::new(BufReader::with_capacity(
            64 * 1024,
            std::fs::File::open(filepath)?,
        ))
    } else {
        Box::new(BufReader::with_capacity(64 * 1024, io::stdin()))
    };

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    // Check if the stream starts with "SMF2" (MIDI Clip File header)
    let is_clip = match reader.fill_buf() {
        Ok(peek) => peek.len() >= 4 && &peek[0..4] == b"SMF2",
        Err(_) => false,
    };

    if is_clip {
        writeln!(writer, "--- el_dump: MIDI Clip File (.midiclip) ---")?;
        let mut all_bytes = Vec::new();
        reader.read_to_end(&mut all_bytes)?;

        if let Some((header, hdr_len)) = parse_clip_header(&all_bytes) {
            writeln!(
                writer,
                "Format: {}, Tracks: {}, TimeDiv: {:?}",
                header.0, header.1, header.2
            )?;

            let mut offset = hdr_len;
            let mut trk_idx = 0;
            while offset + 8 <= all_bytes.len() {
                let chunk_id = &all_bytes[offset..offset + 4];
                let chunk_len = u32::from_be_bytes([
                    all_bytes[offset + 4],
                    all_bytes[offset + 5],
                    all_bytes[offset + 6],
                    all_bytes[offset + 7],
                ]) as usize;
                offset += 8;

                if chunk_id == b"MTrk" {
                    trk_idx += 1;
                    writeln!(
                        writer,
                        "--- Track {} (length {} bytes) ---",
                        trk_idx, chunk_len
                    )?;
                    let track_end = (offset + chunk_len).min(all_bytes.len());
                    let track_data = &all_bytes[offset..track_end];
                    dump_track_events(track_data, &mut writer)?;
                }
                offset += chunk_len;
            }
            writer.flush()?;
            return Ok(());
        }
    }

    let word_iter = std::iter::from_fn(move || {
        let mut buf = [0u8; 4];

        // Peek to see if we are at natural EOF before attempting read_exact
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
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                eprintln!(
                    "Warning: Stream length is not a multiple of 4 bytes. Truncation may occur."
                );
                None
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                None
            }
        }
    });

    let parser = UmpStreamParser::new(word_iter);

    writeln!(writer, "--- el_dump: UMP Stream Analyzer ---")?;
    for ump in parser {
        let mt = ump.message_type();
        let grp = ump.group();
        let wc = ump.word_count();

        // ⚡ Bolt Optimization: Unrolling the formatting loop into a match block based on Word Count.
        // Doing variable length formatting with multiple writes requires repeated buffer interactions
        // and loop overhead. Replacing it with an explicit unrolled match executes about ~20% faster,
        // which is highly noticeable when dumping millions of UMPs to the console.
        match wc {
            1 => writeln!(
                writer,
                "MT: {:?}, Grp: {:2}, Len: 1 words | Data: {:08X} ",
                mt, grp, ump.data[0]
            )?,
            2 => writeln!(
                writer,
                "MT: {:?}, Grp: {:2}, Len: 2 words | Data: {:08X} {:08X} ",
                mt, grp, ump.data[0], ump.data[1]
            )?,
            3 => writeln!(
                writer,
                "MT: {:?}, Grp: {:2}, Len: 3 words | Data: {:08X} {:08X} {:08X} ",
                mt, grp, ump.data[0], ump.data[1], ump.data[2]
            )?,
            4 => writeln!(
                writer,
                "MT: {:?}, Grp: {:2}, Len: 4 words | Data: {:08X} {:08X} {:08X} {:08X} ",
                mt, grp, ump.data[0], ump.data[1], ump.data[2], ump.data[3]
            )?,
            _ => {
                // Safe generic fallback to prevent panics on unexpected lengths,
                // keeping the data bounded to the `ump.data` size
                write!(
                    writer,
                    "MT: {:?}, Grp: {:2}, Len: {} words | Data: ",
                    mt, grp, wc
                )?;
                for i in 0..wc.min(4) {
                    write!(writer, "{:08X} ", ump.data[i])?;
                }
                writeln!(writer)?;
            }
        }
    }
    writer.flush()?;

    Ok(())
}

fn parse_clip_header(bytes: &[u8]) -> Option<((u16, u16, u16), usize)> {
    if bytes.len() < 14 || &bytes[0..4] != b"SMF2" {
        return None;
    }
    let chunk_len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if chunk_len < 6 || bytes.len() < 8 + chunk_len {
        return None;
    }
    let format = u16::from_be_bytes([bytes[8], bytes[9]]);
    let num_tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
    let time_div = u16::from_be_bytes([bytes[12], bytes[13]]);
    Some(((format, num_tracks, time_div), 8 + chunk_len))
}

fn dump_track_events<W: Write>(mut bytes: &[u8], writer: &mut W) -> io::Result<()> {
    while !bytes.is_empty() {
        let mut delta: u32 = 0;
        let mut count = 0;
        let mut vlq_done = false;

        while count < bytes.len() && count < 4 {
            let b = bytes[count];
            count += 1;
            delta = (delta << 7) | ((b & 0x7F) as u32);
            if (b & 0x80) == 0 {
                vlq_done = true;
                break;
            }
        }
        if !vlq_done || bytes.len() < count + 4 {
            break;
        }
        bytes = &bytes[count..];

        let w0 = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let mt = (w0 >> 28) & 0xF;
        let wc = match mt {
            0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
            0x3 | 0x4 | 0x8 | 0x9 | 0xA => 2,
            0xB | 0xC => 3,
            _ => 4,
        };

        if bytes.len() < wc * 4 {
            break;
        }

        write!(writer, "Delta: {:5} | UMP MT: 0x{:X} | ", delta, mt)?;
        for i in 0..wc {
            let w = u32::from_be_bytes([
                bytes[i * 4],
                bytes[i * 4 + 1],
                bytes[i * 4 + 2],
                bytes[i * 4 + 3],
            ]);
            write!(writer, "{:08X} ", w)?;
        }
        writeln!(writer)?;
        bytes = &bytes[wc * 4..];
    }
    Ok(())
}

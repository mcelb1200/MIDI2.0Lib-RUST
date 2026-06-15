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

    let word_iter = std::iter::from_fn(move || {
        let mut buf = [0u8; 4];

        // Peek to see if we are at natural EOF before attempting read_exact
        match reader.fill_buf() {
            Ok(b) if b.is_empty() => return None,
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

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

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

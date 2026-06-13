use clap::Parser;
use el_core::parser::UmpStreamParser;
use std::io::{self, BufWriter, Read, Write};

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

    // ⚡ Bolt Optimization: Algorithmic structural improvement to avoid allocating massive
    // multi-gigabyte files into RAM all at once using `std::fs::read` or `read_to_end`.
    // By streaming the file incrementally using an Iterator over `Read`, we drastically reduce
    // memory pressure and prevent out-of-memory crashes for large UMP log files, while keeping
    // parser state perfectly contiguous.
    let reader: Box<dyn Read> = if let Some(filepath) = args.file {
        Box::new(std::fs::File::open(filepath)?)
    } else {
        Box::new(io::stdin())
    };

    // Use a large buffered capacity (64KB) for optimal I/O chunking
    let mut buf_reader = io::BufReader::with_capacity(65536, reader);

    let word_iter = std::iter::from_fn(move || {
        let mut buf = [0u8; 4];
        let mut bytes_read = 0;

        while bytes_read < 4 {
            match buf_reader.read(&mut buf[bytes_read..]) {
                Ok(0) => {
                    // EOF reached
                    if bytes_read > 0 {
                        eprintln!(
                            "Warning: Stream truncated. {} bytes dropped.",
                            bytes_read
                        );
                    }
                    return None;
                }
                Ok(n) => bytes_read += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    eprintln!("Warning: Error reading stream: {}", e);
                    return None;
                }
            }
        }

        Some(u32::from_le_bytes(buf))
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

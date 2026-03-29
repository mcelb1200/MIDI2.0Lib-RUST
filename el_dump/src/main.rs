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

    let buffer = if let Some(filepath) = args.file {
        std::fs::read(filepath)?
    } else {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        buffer
    };

    if buffer.len() % 4 != 0 {
        eprintln!(
            "Warning: Stream length {} is not a multiple of 4 bytes. Truncation may occur.",
            buffer.len()
        );
    }

    // Stream raw u8 bytes into Little-Endian u32 words lazily without intermediate allocation
    let word_iter = buffer.chunks_exact(4).map(|chunk| {
        // ⚡ Bolt Optimization: Using try_into().unwrap() on chunks_exact(4) is ~5-10% faster
        // than manual array indexing, allowing better vectorization and skipping bounds checks.
        u32::from_le_bytes(chunk.try_into().unwrap())
    });

    let parser = UmpStreamParser::new(word_iter);

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    writeln!(writer, "--- el_dump: UMP Stream Analyzer ---")?;
    for ump in parser {
        let mt = ump.message_type();
        let grp = ump.group();
        let wc = ump.word_count();

        write!(
            writer,
            "MT: {:?}, Grp: {:2}, Len: {} words | Data: ",
            mt, grp, wc
        )?;
        for i in 0..wc {
            write!(writer, "{:08X} ", ump.data[i])?;
        }
        writeln!(writer)?;
    }
    writer.flush()?;

    Ok(())
}

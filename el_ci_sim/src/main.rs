use clap::{Parser, Subcommand};
use el_midi2::ci_pe::{
    hash_resource, CiPe, CiProcessInquiry, PeReplyChunker, PeTransactionManager,
    SubscriptionManager, MIDICI_PE_GET, MIDICI_PE_SET, MIDICI_PE_SUB,
};

#[derive(Parser, Debug)]
#[command(author, version, about = "MIDI-CI Capability Inquiry Simulator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Property Exchange simulation commands
    Pe {
        #[command(subcommand)]
        action: PeCommands,
    },
    /// Process Inquiry simulation commands
    Pi {
        #[command(subcommand)]
        action: PiCommands,
    },
}

#[derive(Subcommand, Debug)]
enum PeCommands {
    /// Generate PE Get Inquiry
    Get {
        #[arg(short, long)]
        resource: String,
        #[arg(short, long, default_value_t = 0x11111)]
        src: u32,
        #[arg(short, long, default_value_t = 0x22222)]
        dest: u32,
    },
    /// Generate PE Set Inquiry
    Set {
        #[arg(short, long)]
        resource: String,
        #[arg(short, long)]
        value: String,
        #[arg(short, long, default_value_t = 0x11111)]
        src: u32,
        #[arg(short, long, default_value_t = 0x22222)]
        dest: u32,
    },
    /// Generate PE Subscribe Inquiry
    Subscribe {
        #[arg(short, long)]
        resource: String,
        #[arg(short, long, default_value_t = 0x11111)]
        src: u32,
        #[arg(short, long, default_value_t = 0x22222)]
        dest: u32,
    },
}

#[derive(Subcommand, Debug)]
enum PiCommands {
    /// Generate Process Inquiry MIDI Message Report Inquiry
    Report {
        #[arg(short, long, default_value_t = 0x11111)]
        src: u32,
        #[arg(short, long, default_value_t = 0x22222)]
        dest: u32,
        #[arg(short, long, default_value_t = 0)]
        system: u8,
        #[arg(short, long, default_value_t = 0x7F)]
        channel_controller: u8,
        #[arg(short, long, default_value_t = 0x7F)]
        channel_note: u8,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pe { action } => match action {
            PeCommands::Get {
                resource,
                src,
                dest,
            } => {
                let mut tx_mgr = PeTransactionManager::<16>::new();
                let req_id = tx_mgr.start_request(dest, 100).unwrap_or(1);
                let json_header = format!("{{\"resource\":\"{}\"}}", resource);

                let mut buf = [0_u8; 256];
                let len = CiPe::serialize_pe_header_only(
                    &mut buf,
                    MIDICI_PE_GET,
                    2,
                    src,
                    dest,
                    req_id,
                    json_header.as_bytes(),
                )
                .expect("Failed to serialize PE Get message");

                println!(
                    "PE Get Inquiry generated (Request ID={}, Bytes={}): {:02X?}",
                    req_id,
                    len,
                    &buf[..len]
                );

                // Simulate chunking to MT=0x3 UMPs
                let chunker = PeReplyChunker::new_sysex7(0, &buf[..len]);
                let count = chunker.count();
                println!("Chunked into {} MT=0x3 UMP packets", count);
            }
            PeCommands::Set {
                resource,
                value,
                src,
                dest,
            } => {
                let mut tx_mgr = PeTransactionManager::<16>::new();
                let req_id = tx_mgr.start_request(dest, 100).unwrap_or(1);
                let json_header = format!("{{\"resource\":\"{}\"}}", resource);
                let body = value.into_bytes();

                let mut buf = [0_u8; 512];
                let len = CiPe::serialize_pe_with_body(
                    &mut buf,
                    MIDICI_PE_SET,
                    2,
                    src,
                    dest,
                    req_id,
                    json_header.as_bytes(),
                    1,
                    1,
                    &body,
                )
                .expect("Failed to serialize PE Set message");

                println!(
                    "PE Set Inquiry generated (Request ID={}, Bytes={}): {:02X?}",
                    req_id,
                    len,
                    &buf[..len]
                );
            }
            PeCommands::Subscribe {
                resource,
                src,
                dest,
            } => {
                let mut tx_mgr = PeTransactionManager::<16>::new();
                let req_id = tx_mgr.start_request(dest, 100).unwrap_or(1);
                let hash = hash_resource(&resource);
                let mut sub_mgr = SubscriptionManager::<16>::new();
                sub_mgr.add_subscription(src, hash);

                let json_header =
                    format!("{{\"resource\":\"{}\",\"action\":\"subscribe\"}}", resource);
                let mut buf = [0_u8; 256];
                let _len = CiPe::serialize_pe_header_only(
                    &mut buf,
                    MIDICI_PE_SUB,
                    2,
                    src,
                    dest,
                    req_id,
                    json_header.as_bytes(),
                )
                .expect("Failed to serialize PE Subscribe message");

                println!(
                    "PE Subscribe Inquiry generated (Request ID={}, ResourceHash={:#08X}): ActiveSubs={}",
                    req_id,
                    hash,
                    sub_mgr.len()
                );
            }
        },
        Commands::Pi { action } => match action {
            PiCommands::Report {
                src,
                dest,
                system,
                channel_controller,
                channel_note,
            } => {
                let payload = CiProcessInquiry::build_midi_message_report(
                    src,
                    dest,
                    0x7F,
                    2,
                    0,
                    system,
                    channel_controller,
                    channel_note,
                );
                println!(
                    "Process Inquiry MIDI Message Report generated: {:02X?}",
                    payload
                );
            }
        },
    }
}

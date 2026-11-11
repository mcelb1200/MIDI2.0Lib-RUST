//! UMP decoding and formatting engine for `el_dump`.
//!
//! Provides human-readable decoding of Universal MIDI Packets (UMP) across all
//! Message Types (MT=0x0 Utility, MT=0x1 System, MT=0x2 MIDI 1.0 Voice, MT=0x3 SysEx7,
//! MT=0x4 MIDI 2.0 Voice, MT=0x5 SysEx8/MDS, MT=0xD Flex Data, MT=0xF Stream).

use el_core::ump::{MessageType, Ump};

/// Configuration options for formatting decoded UMP output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecoderConfig {
    /// Enable ANSI terminal color formatting.
    pub color: bool,
    /// Enable detailed verbose breakdown.
    pub verbose: bool,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            color: true,
            verbose: false,
        }
    }
}

/// ANSI color escape sequences.
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
    pub const BRIGHT_BLACK: &str = "\x1b[90m";
}

/// Decodes a UMP into a formatted human-readable string.
pub fn decode_ump(ump: &Ump, config: &DecoderConfig) -> String {
    let mt = ump.message_type();
    let grp = ump.group();
    let wc = ump.word_count();

    let hex_dump = format_raw_hex(ump, wc);
    let desc = decode_ump_description(ump);

    if config.color {
        let (mt_color, type_str) = get_mt_styling(mt);
        if config.verbose {
            format!(
                "{}{}{:17}{} | Grp: {}{:2}{} | Len: {}w | Hex: {}{}{} | {}{}{}",
                colors::BOLD,
                mt_color,
                type_str,
                colors::RESET,
                colors::CYAN,
                grp,
                colors::RESET,
                wc,
                colors::BRIGHT_BLACK,
                hex_dump,
                colors::RESET,
                colors::WHITE,
                desc,
                colors::RESET
            )
        } else {
            format!(
                "{}{}{:17}{} Grp {:2} | {}{}{}",
                colors::BOLD,
                mt_color,
                type_str,
                colors::RESET,
                grp,
                colors::WHITE,
                desc,
                colors::RESET
            )
        }
    } else if config.verbose {
        format!(
            "{:17} | Grp: {:2} | Len: {}w | Hex: {} | {}",
            format!("{:?}", mt),
            grp,
            wc,
            hex_dump,
            desc
        )
    } else {
        format!("{:17} Grp {:2} | {}", format!("{:?}", mt), grp, desc)
    }
}

/// Returns raw hex representation of valid words in the UMP.
fn format_raw_hex(ump: &Ump, word_count: usize) -> String {
    match word_count {
        1 => format!("{:08X}", ump.data[0]),
        2 => format!("{:08X} {:08X}", ump.data[0], ump.data[1]),
        3 => format!(
            "{:08X} {:08X} {:08X}",
            ump.data[0], ump.data[1], ump.data[2]
        ),
        _ => format!(
            "{:08X} {:08X} {:08X} {:08X}",
            ump.data[0], ump.data[1], ump.data[2], ump.data[3]
        ),
    }
}

fn get_mt_styling(mt: MessageType) -> (&'static str, &'static str) {
    match mt {
        MessageType::Utility => (colors::BRIGHT_BLACK, "Utility (0x0)"),
        MessageType::System => (colors::YELLOW, "System (0x1)"),
        MessageType::Midi1ChannelVoice => (colors::GREEN, "MIDI 1.0 (0x2)"),
        MessageType::Data64 => (colors::CYAN, "SysEx7 (0x3)"),
        MessageType::Midi2ChannelVoice => (colors::BLUE, "MIDI 2.0 (0x4)"),
        MessageType::Data128 => (colors::MAGENTA, "Data128/MDS (0x5)"),
        MessageType::ReservedD => (colors::RED, "Flex Data (0xD)"),
        MessageType::UmpStream => (colors::YELLOW, "Stream (0xF)"),
        _ => (colors::WHITE, "Reserved"),
    }
}

/// Decodes the packet semantics into an analytical description string.
pub fn decode_ump_description(ump: &Ump) -> String {
    let w0 = ump.data[0];
    let mt = (w0 >> 28) & 0xF;

    match mt {
        0x0 => decode_utility(w0),
        0x1 => decode_system(w0),
        0x2 => decode_midi1_voice(w0),
        0x3 => decode_sysex7(ump),
        0x4 => decode_midi2_voice(ump),
        0x5 => decode_data128_or_mds(ump),
        0xD => decode_flex_data(ump),
        0xF => decode_stream(ump),
        _ => format!("Reserved Message Type 0x{:X}", mt),
    }
}

fn decode_utility(w0: u32) -> String {
    let status = (w0 >> 20) & 0xF;
    match status {
        0x0 => "NOOP (No Operation)".to_string(),
        0x1 => {
            let clock = (w0 & 0xFFFF) as u16;
            format!("JR Clock: timestamp = {} ({} µs)", clock, clock as u32 * 32)
        }
        0x2 => {
            let ts = (w0 & 0xFFFF) as u16;
            format!("JR Timestamp: timestamp = {} ({} µs)", ts, ts as u32 * 32)
        }
        0x3 => {
            let ticks = (w0 & 0xFFFF) as u16;
            format!("Delta Clockstamp Ticks: {} ticks per quarter note", ticks)
        }
        0x4 => {
            let ticks = (w0 & 0xFFFF) as u16;
            format!("Delta Clockstamp Since Last Event: {} ticks elapsed", ticks)
        }
        _ => format!("Utility Unknown Status 0x{:X}", status),
    }
}

fn decode_system(w0: u32) -> String {
    let status = ((w0 >> 16) & 0xFF) as u8;
    match status {
        0xF8 => "Timing Clock (24 PPQN)".to_string(),
        0xFA => "Start Sequence".to_string(),
        0xFB => "Continue Sequence".to_string(),
        0xFC => "Stop Sequence".to_string(),
        0xFE => "Active Sensing".to_string(),
        0xFF => "System Reset".to_string(),
        0xF1 => {
            let data = ((w0 >> 8) & 0x7F) as u8;
            let piece = (data >> 4) & 0x7;
            let val = data & 0xF;
            format!("MTC Quarter Frame: piece = {}, val = {}", piece, val)
        }
        0xF2 => {
            let lsb = ((w0 >> 8) & 0x7F) as u16;
            let msb = (w0 & 0x7F) as u16;
            let pos = lsb | (msb << 7);
            format!("Song Position Pointer: pos = {} (beats = {})", pos, pos * 6)
        }
        0xF3 => {
            let song = ((w0 >> 8) & 0x7F) as u8;
            format!("Song Select: song = {}", song)
        }
        0xF6 => "Tune Request".to_string(),
        _ => format!("System Common/Real-Time (0x{:02X})", status),
    }
}

fn decode_midi1_voice(w0: u32) -> String {
    let status = ((w0 >> 16) & 0xFF) as u8;
    let ch = status & 0x0F;
    let opcode = status & 0xF0;
    let d1 = ((w0 >> 8) & 0x7F) as u8;
    let d2 = (w0 & 0x7F) as u8;

    match opcode {
        0x80 => format!("Note Off: Ch {}, Note {}, Velocity {}", ch, d1, d2),
        0x90 => {
            if d2 == 0 {
                format!("Note Off (Vel 0): Ch {}, Note {}", ch, d1)
            } else {
                format!("Note On: Ch {}, Note {}, Velocity {}", ch, d1, d2)
            }
        }
        0xA0 => format!("Poly Pressure: Ch {}, Note {}, Pressure {}", ch, d1, d2),
        0xB0 => {
            let cc_desc = match d1 {
                120 => " [All Sound Off]",
                121 => " [Reset All Controllers]",
                122 => " [Local Control]",
                123 => " [All Notes Off]",
                124 => " [Omni Mode Off]",
                125 => " [Omni Mode On]",
                126 => " [Mono Mode On]",
                127 => " [Poly Mode On]",
                0..=31 => " [MSB]",
                32..=63 => " [LSB]",
                _ => "",
            };
            format!(
                "Control Change: Ch {}, CC #{}{}, Value {}",
                ch, d1, cc_desc, d2
            )
        }
        0xC0 => format!("Program Change: Ch {}, Program {}", ch, d1),
        0xD0 => format!("Channel Pressure: Ch {}, Pressure {}", ch, d1),
        0xE0 => {
            let lsb = d1 as u16;
            let msb = d2 as u16;
            let bend = lsb | (msb << 7);
            let rel = bend as i32 - 8192;
            format!("Pitch Bend: Ch {}, Value {} (offset {:+})", ch, bend, rel)
        }
        _ => format!("MIDI 1.0 Ch {}, Status 0x{:02X}", ch, status),
    }
}

fn decode_sysex7(ump: &Ump) -> String {
    let status_nibble = (ump.data[0] >> 20) & 0xF;
    let num_bytes = ((ump.data[0] >> 16) & 0xF) as usize;

    let format_str = match status_nibble {
        0x0 => "Complete",
        0x1 => "Start",
        0x2 => "Continue",
        0x3 => "End",
        _ => "Reserved",
    };

    let raw_bytes = [
        ((ump.data[0] >> 8) & 0xFF) as u8,
        (ump.data[0] & 0xFF) as u8,
        ((ump.data[1] >> 24) & 0xFF) as u8,
        ((ump.data[1] >> 16) & 0xFF) as u8,
        ((ump.data[1] >> 8) & 0xFF) as u8,
        (ump.data[1] & 0xFF) as u8,
    ];

    let valid_count = num_bytes.min(6);
    let mut payload_hex = String::new();
    for b in raw_bytes.iter().take(valid_count) {
        payload_hex.push_str(&format!("{:02X} ", b));
    }

    // Check if starting Universal SysEx
    let mut extra = String::new();
    if valid_count >= 3 && raw_bytes[0] == 0x7E {
        if raw_bytes[2] == 0x06 {
            extra = " [Device Identity Inquiry]".to_string();
        } else if raw_bytes[2] == 0x0D {
            extra = " [MIDI-CI]".to_string();
        }
    }

    format!(
        "SysEx7 {}: {} bytes [ {}{}]",
        format_str,
        valid_count,
        payload_hex.trim(),
        extra
    )
}

fn decode_midi2_voice(ump: &Ump) -> String {
    let status_byte = ((ump.data[0] >> 16) & 0xFF) as u8;
    let ch = status_byte & 0x0F;
    let opcode = status_byte & 0xF0;
    let d1 = ((ump.data[0] >> 8) & 0x7F) as u8;
    let d2 = (ump.data[0] & 0x7F) as u8;
    let val32 = ump.data[1];

    match opcode {
        0x80 => {
            let vel16 = (val32 >> 16) as u16;
            let attr_type = d2;
            let attr_data = (val32 & 0xFFFF) as u16;
            format!(
                "MIDI 2.0 Note Off: Ch {}, Note {}, Vel16 {} (0x{:04X}), Attr Type {}, Attr Data 0x{:04X}",
                ch, d1, vel16, vel16, attr_type, attr_data
            )
        }
        0x90 => {
            let vel16 = (val32 >> 16) as u16;
            let attr_type = d2;
            let attr_data = (val32 & 0xFFFF) as u16;
            format!(
                "MIDI 2.0 Note On: Ch {}, Note {}, Vel16 {} (0x{:04X}), Attr Type {}, Attr Data 0x{:04X}",
                ch, d1, vel16, vel16, attr_type, attr_data
            )
        }
        0xA0 => {
            format!(
                "MIDI 2.0 Poly Pressure: Ch {}, Note {}, Pressure32 {} (0x{:08X})",
                ch, d1, val32, val32
            )
        }
        0xB0 => {
            format!(
                "MIDI 2.0 Control Change: Ch {}, CC #{}, Value32 {} (0x{:08X})",
                ch, d1, val32, val32
            )
        }
        0xC0 => {
            let prog = ((val32 >> 24) & 0x7F) as u8;
            let bank_valid = (val32 & 0x1) != 0;
            let bank_msb = ((val32 >> 8) & 0x7F) as u8;
            let bank_lsb = (val32 & 0x7F) as u8;
            if bank_valid {
                format!(
                    "MIDI 2.0 Program Change: Ch {}, Program {}, Bank MSB {}, Bank LSB {}",
                    ch, prog, bank_msb, bank_lsb
                )
            } else {
                format!("MIDI 2.0 Program Change: Ch {}, Program {}", ch, prog)
            }
        }
        0xD0 => {
            format!(
                "MIDI 2.0 Channel Pressure: Ch {}, Pressure32 {} (0x{:08X})",
                ch, val32, val32
            )
        }
        0xE0 => {
            let rel = val32 as i64 - 0x8000_0000_i64;
            format!(
                "MIDI 2.0 Pitch Bend: Ch {}, Value32 0x{:08X} (offset {:+})",
                ch, val32, rel
            )
        }
        0x00 => {
            // Registered Per-Note Controller (RPN)
            format!(
                "Per-Note RPN: Ch {}, Note {}, Index {}, Data 0x{:08X}",
                ch, d1, d2, val32
            )
        }
        0x10 => {
            // Assignable Per-Note Controller (CC/NRPN)
            format!(
                "Per-Note Assignable CC: Ch {}, Note {}, Index {}, Data 0x{:08X}",
                ch, d1, d2, val32
            )
        }
        0x20 => {
            // Registered Parameter Number (RPN)
            let bank = d1;
            let index = d2;
            format!(
                "MIDI 2.0 RPN: Ch {}, Bank {}, Index {}, Value32 0x{:08X}",
                ch, bank, index, val32
            )
        }
        0x30 => {
            // Assignable Parameter Number (NRPN)
            let bank = d1;
            let index = d2;
            format!(
                "MIDI 2.0 NRPN: Ch {}, Bank {}, Index {}, Value32 0x{:08X}",
                ch, bank, index, val32
            )
        }
        0x60 => {
            // Per-Note Pitch Bend
            format!(
                "Per-Note Pitch Bend: Ch {}, Note {}, Value32 0x{:08X}",
                ch, d1, val32
            )
        }
        0xF0 => {
            // Per-Note Management
            let detach = (d2 & 0x2) != 0;
            let reset = (d2 & 0x1) != 0;
            format!(
                "Per-Note Management: Ch {}, Note {}, Detach: {}, Reset: {}",
                ch, d1, detach, reset
            )
        }
        _ => format!("MIDI 2.0 Voice Opcode 0x{:02X}, Ch {}", opcode, ch),
    }
}

fn decode_data128_or_mds(ump: &Ump) -> String {
    let status_nibble = (ump.data[0] >> 20) & 0xF;
    match status_nibble {
        0x8 => {
            // MDS Header
            let mds_id = ((ump.data[0] >> 16) & 0xF) as u8;
            let byte_count = (ump.data[0] & 0xFFFF) as u16;
            let total_chunks = (ump.data[1] >> 16) as u16;
            let chunk_num = (ump.data[1] & 0xFFFF) as u16;
            format!(
                "MDS Header: ID {}, ByteCount {}, Chunks {}/{})",
                mds_id, byte_count, chunk_num, total_chunks
            )
        }
        0x9 => {
            // MDS Payload
            let mds_id = ((ump.data[0] >> 16) & 0xF) as u8;
            format!("MDS Payload: ID {}", mds_id)
        }
        0x0..=0x3 => {
            // SysEx8
            let format_str = match status_nibble {
                0x0 => "Complete",
                0x1 => "Start",
                0x2 => "Continue",
                0x3 => "End",
                _ => "",
            };
            let num_bytes = ((ump.data[0] >> 16) & 0xF) as usize;
            let stream_id = ((ump.data[0] >> 8) & 0xFF) as u8;
            format!(
                "SysEx8 {}: Stream ID {}, Num Bytes {}",
                format_str, stream_id, num_bytes
            )
        }
        _ => format!("Data128 Status 0x{:X}", status_nibble),
    }
}

fn decode_flex_data(ump: &Ump) -> String {
    let status_bank = ((ump.data[0] >> 8) & 0xFF) as u8;
    let status = (ump.data[0] & 0xFF) as u8;

    match status_bank {
        0x00 => {
            // Common
            match status {
                0x00 => {
                    let tempo_10ns = ump.data[1];
                    let bpm = if tempo_10ns > 0 {
                        6_000_000_000.0 / (tempo_10ns as f64)
                    } else {
                        120.0
                    };
                    format!(
                        "Flex Data Common Tempo: {:.2} BPM ({} 10-ns units PQN)",
                        bpm, tempo_10ns
                    )
                }
                0x07 => {
                    let bar = (ump.data[1] >> 16) as u16;
                    format!("Flex Data Bar Marker: Bar #{}", bar)
                }
                0x08 => {
                    let beat = (ump.data[1] >> 16) as u16;
                    format!("Flex Data Beat Marker: Beat #{}", beat)
                }
                _ => format!("Flex Data Common Status 0x{:02X}", status),
            }
        }
        0x03 => {
            // Project Metadata
            let field_desc = match status {
                0x00 => "Project Name",
                0x01 => "Composition Name",
                0x02 => "Arranger Name",
                0x03 => "Composer Name",
                0x04 => "Copyright Notice",
                _ => "Project Text",
            };
            let text = extract_flex_text(ump);
            format!("Flex Data {}: \"{}\"", field_desc, text)
        }
        0x04 => {
            // Track Metadata
            let field_desc = match status {
                0x00 => "Track Name",
                0x01 => "Channel Name",
                0x02 => "Clip Name",
                0x03 => "Marker Note",
                _ => "Track Text",
            };
            let text = extract_flex_text(ump);
            format!("Flex Data {}: \"{}\"", field_desc, text)
        }
        _ => format!(
            "Flex Data Bank 0x{:02X}, Status 0x{:02X}",
            status_bank, status
        ),
    }
}

fn extract_flex_text(ump: &Ump) -> String {
    let mut bytes = Vec::new();
    for &word in &[ump.data[1], ump.data[2], ump.data[3]] {
        for b in (0..4).rev() {
            let byte = ((word >> (b * 8)) & 0xFF) as u8;
            if byte != 0 {
                bytes.push(byte);
            }
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn decode_stream(ump: &Ump) -> String {
    let status_code = ((ump.data[0] >> 16) & 0x3FF) as u16;
    match status_code {
        0x000 => "Endpoint Discovery".to_string(),
        0x001 => {
            let major = ((ump.data[0] >> 8) & 0xFF) as u8;
            let minor = (ump.data[0] & 0xFF) as u8;
            let num_fb = ((ump.data[1] >> 24) & 0xFF) as u8;
            format!(
                "Endpoint Info: UMP v{}.{}, Function Blocks: {}",
                major, minor, num_fb
            )
        }
        0x002 => "Device Identity Notification".to_string(),
        0x003 => "Endpoint Name Notification".to_string(),
        0x004 => "Product Instance ID Notification".to_string(),
        0x005 => {
            let proto = ((ump.data[0] >> 8) & 0xFF) as u8;
            let proto_str = if proto == 2 { "MIDI 2.0" } else { "MIDI 1.0" };
            format!("Stream Configuration Request: Protocol {}", proto_str)
        }
        0x006 => {
            let proto = ((ump.data[0] >> 8) & 0xFF) as u8;
            let proto_str = if proto == 2 { "MIDI 2.0" } else { "MIDI 1.0" };
            format!("Stream Configuration Notification: Protocol {}", proto_str)
        }
        0x011 => {
            let fb_idx = ((ump.data[0] >> 8) & 0x7F) as u8;
            let first_grp = ((ump.data[1] >> 24) & 0xFF) as u8;
            let grp_count = ((ump.data[1] >> 16) & 0xFF) as u8;
            format!(
                "Function Block Info: Block #{}, First Group {}, Count {}",
                fb_idx, first_grp, grp_count
            )
        }
        0x012 => "Function Block Name Notification".to_string(),
        _ => format!("Stream Message Status 0x{:03X}", status_code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_midi1_note_on_off() {
        let ump = Ump::new(0x20903C64, 0, 0, 0);
        let desc = decode_ump_description(&ump);
        assert_eq!(desc, "Note On: Ch 0, Note 60, Velocity 100");

        let ump_off = Ump::new(0x20803C40, 0, 0, 0);
        let desc_off = decode_ump_description(&ump_off);
        assert_eq!(desc_off, "Note Off: Ch 0, Note 60, Velocity 64");
    }

    #[test]
    fn test_decode_midi2_note_on_cc_and_pitchbend() {
        // MIDI 2.0 Note On: Ch 1, Note 72, Vel16 0xC000
        let ump_note2 = Ump::new(0x40914800, 0xC0000000, 0, 0);
        let desc_note2 = decode_ump_description(&ump_note2);
        assert!(desc_note2.contains("MIDI 2.0 Note On: Ch 1, Note 72, Vel16 49152"));

        // MIDI 2.0 CC: Ch 2, CC #7, Value32 0x80000000
        let ump_cc2 = Ump::new(0x40B20700, 0x80000000, 0, 0);
        let desc_cc2 = decode_ump_description(&ump_cc2);
        assert!(desc_cc2.contains("MIDI 2.0 Control Change: Ch 2, CC #7"));

        // MIDI 2.0 Pitch Bend center
        let ump_pb2 = Ump::new(0x40E00000, 0x80000000, 0, 0);
        let desc_pb2 = decode_ump_description(&ump_pb2);
        assert!(desc_pb2.contains("MIDI 2.0 Pitch Bend: Ch 0, Value32 0x80000000 (offset +0)"));
    }

    #[test]
    fn test_decode_flex_tempo_and_track_name() {
        // Common Tempo: 50,000,000 10-ns units = 120.00 BPM
        let ump_tempo = Ump::new(0xD0000000, 50_000_000, 0, 0);
        let desc_tempo = decode_ump_description(&ump_tempo);
        assert!(desc_tempo.contains("Flex Data Common Tempo: 120.00 BPM"));

        // Track Name: Bank 0x04, Status 0x00
        let ump_name = Ump::new(0xD0000400, 0x54657374, 0, 0);
        let desc_name = decode_ump_description(&ump_name);
        assert!(desc_name.contains("Flex Data Track Name: \"Test\""));
    }

    #[test]
    fn test_decode_system_and_utility() {
        let clock = Ump::new(0x10F80000, 0, 0, 0);
        assert_eq!(decode_ump_description(&clock), "Timing Clock (24 PPQN)");

        let jr = Ump::new(0x00100064, 0, 0, 0);
        assert_eq!(
            decode_ump_description(&jr),
            "JR Clock: timestamp = 100 (3200 µs)"
        );
    }
}

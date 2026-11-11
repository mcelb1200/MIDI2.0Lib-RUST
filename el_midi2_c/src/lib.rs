//! C-ABI dynamic and static library bindings for `el_midi2` and `el_core`.
//!
//! Provides stable C-ABI exported functions and structures matching `include/el_midi2.h`
//! for interoperability with C++20 and embedded C RTOS codebases.

use el_core::builder::VoiceBuilder;
use el_core::ump::{MessageType, Ump};
use el_core::utils::{scale_down, scale_up};
use std::panic::catch_unwind;

/// C-compatible representation of a Universal MIDI Packet (UMP).
///
/// Contains up to 4 32-bit words (128 bits). For 32-bit or 64-bit messages,
/// unused high words are zeroed.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct el_ump_t {
    pub data: [u32; 4],
}

impl From<Ump> for el_ump_t {
    #[inline]
    fn from(ump: Ump) -> Self {
        Self { data: ump.data }
    }
}

impl From<el_ump_t> for Ump {
    #[inline]
    fn from(ump: el_ump_t) -> Self {
        Self { data: ump.data }
    }
}

/// Creates a MIDI 1.0 Channel Voice Note On message (MT=0x2) as a 32-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi1_note_on(
    group: u8,
    channel: u8,
    note: u8,
    velocity: u8,
) -> el_ump_t {
    let res = catch_unwind(|| VoiceBuilder::midi1_note_on(group, channel, note, velocity));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Channel Voice Note On message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi2_note_on(
    group: u8,
    channel: u8,
    note: u8,
    attribute_type: u8,
    velocity: u16,
    attribute_data: u16,
) -> el_ump_t {
    let res = catch_unwind(|| {
        VoiceBuilder::midi2_note_on(
            group,
            channel,
            note,
            attribute_type,
            velocity,
            attribute_data,
        )
    });
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Control Change message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi2_cc(
    group: u8,
    channel: u8,
    index: u8,
    value: u32,
) -> el_ump_t {
    let res = catch_unwind(|| VoiceBuilder::midi2_cc(group, channel, index, value));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Pitch Bend message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_pitch_bend(group: u8, channel: u8, value: u32) -> el_ump_t {
    let res = catch_unwind(|| VoiceBuilder::midi2_pitch_bend(group, channel, value));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Registered Parameter Number (RPN) message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi2_rpn(
    group: u8,
    channel: u8,
    bank: u8,
    index: u8,
    value: u32,
) -> el_ump_t {
    let res = catch_unwind(|| VoiceBuilder::midi2_rpn(group, channel, bank, index, value));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Non-Registered Parameter Number (NRPN) message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi2_nrpn(
    group: u8,
    channel: u8,
    bank: u8,
    index: u8,
    value: u32,
) -> el_ump_t {
    let res = catch_unwind(|| VoiceBuilder::midi2_nrpn(group, channel, bank, index, value));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Relative RPN message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi2_relative_rpn(
    group: u8,
    channel: u8,
    bank: u8,
    index: u8,
    value: i32,
) -> el_ump_t {
    let res = catch_unwind(|| VoiceBuilder::midi2_relative_rpn(group, channel, bank, index, value));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Creates a MIDI 2.0 Relative NRPN message (MT=0x4) as a 64-bit UMP.
#[no_mangle]
pub extern "C" fn el_ump_create_midi2_relative_nrpn(
    group: u8,
    channel: u8,
    bank: u8,
    index: u8,
    value: i32,
) -> el_ump_t {
    let res =
        catch_unwind(|| VoiceBuilder::midi2_relative_nrpn(group, channel, bank, index, value));
    match res {
        Ok(ump) => ump.into(),
        Err(_) => el_ump_t { data: [0; 4] },
    }
}

/// Scales a value up to a higher bit depth using MIDI 2.0 bit-duplication scaling.
#[no_mangle]
pub extern "C" fn el_scale_up(val: u32, src_bits: u8, dst_bits: u8) -> u32 {
    let res = catch_unwind(|| scale_up(val, src_bits, dst_bits));
    res.unwrap_or(0)
}

/// Scales a value down to a lower bit depth using bit-shift truncation.
#[no_mangle]
pub extern "C" fn el_scale_down(val: u32, src_bits: u8, dst_bits: u8) -> u32 {
    let res = catch_unwind(|| scale_down(val, src_bits, dst_bits));
    res.unwrap_or(0)
}

/// Incremental UMP stream parser state machine.
///
/// Accumulates incoming 32-bit words until a full Universal MIDI Packet (1..4 words)
/// is completed according to its Message Type header.
#[repr(C)]
#[derive(Debug, Default)]
pub struct el_stream_parser_t {
    buffer: [u32; 4],
    words_expected: usize,
    words_received: usize,
}

impl el_stream_parser_t {
    /// Creates a new, reset stream parser.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; 4],
            words_expected: 0,
            words_received: 0,
        }
    }

    /// Resets the internal word buffer.
    pub fn reset(&mut self) {
        self.buffer = [0; 4];
        self.words_expected = 0;
        self.words_received = 0;
    }

    /// Feeds a single 32-bit word. Returns `Some(el_ump_t)` when a full UMP is parsed.
    pub fn feed_word(&mut self, word: u32) -> Option<el_ump_t> {
        if self.words_received == 0 {
            let mt = MessageType::from_u32(word);
            self.words_expected = mt.word_count();
            self.buffer[0] = word;
            self.words_received = 1;
        } else {
            self.buffer[self.words_received] = word;
            self.words_received += 1;
        }

        if self.words_received == self.words_expected {
            let ump = el_ump_t { data: self.buffer };
            self.reset();
            Some(ump)
        } else {
            None
        }
    }
}

/// Allocates and initializes a new UMP stream parser.
///
/// The returned pointer must be freed with `el_stream_parser_destroy`.
#[no_mangle]
pub extern "C" fn el_stream_parser_create() -> *mut el_stream_parser_t {
    let res = catch_unwind(|| Box::into_raw(Box::new(el_stream_parser_t::new())));
    res.unwrap_or(std::ptr::null_mut())
}

/// Feeds a 32-bit word into the stream parser.
///
/// If a complete UMP packet is assembled, writes it to `out_ump` and returns `true`.
/// Otherwise, returns `false`.
///
/// # Safety
/// If non-null, `parser` must point to a valid, initialized `el_stream_parser_t` created by
/// `el_stream_parser_create`. If non-null, `out_ump` must point to valid, writable memory for
/// `el_ump_t`. Passing null pointers is safely checked and returns `false`.
#[no_mangle]
pub unsafe extern "C" fn el_stream_parser_feed_word(
    parser: *mut el_stream_parser_t,
    word: u32,
    out_ump: *mut el_ump_t,
) -> bool {
    if parser.is_null() || out_ump.is_null() {
        return false;
    }

    let res = catch_unwind(|| {
        let p = unsafe { &mut *parser };
        if let Some(ump) = p.feed_word(word) {
            unsafe {
                *out_ump = ump;
            }
            true
        } else {
            false
        }
    });

    res.unwrap_or(false)
}

/// Frees an allocated UMP stream parser.
///
/// # Safety
/// If non-null, `parser` must point to an active `el_stream_parser_t` allocated by
/// `el_stream_parser_create` and not previously freed. Passing `NULL` is safely ignored.
#[no_mangle]
pub unsafe extern "C" fn el_stream_parser_destroy(parser: *mut el_stream_parser_t) {
    if parser.is_null() {
        return;
    }

    let _ = catch_unwind(|| unsafe {
        drop(Box::from_raw(parser));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ump_creation_factories() {
        let m1_note = el_ump_create_midi1_note_on(1, 2, 60, 100);
        assert_eq!(m1_note.data[0], 0x21923C64);
        assert_eq!(m1_note.data[1], 0);

        let m2_note = el_ump_create_midi2_note_on(0, 5, 72, 0, 0xC000, 0);
        assert_eq!(m2_note.data[0], 0x40954800);
        assert_eq!(m2_note.data[1], 0xC0000000);

        let cc = el_ump_create_midi2_cc(2, 3, 7, 0x12345678);
        assert_eq!(cc.data[0], 0x42B30700);
        assert_eq!(cc.data[1], 0x12345678);

        let pb = el_ump_create_pitch_bend(3, 4, 0x80000000);
        assert_eq!(pb.data[0], 0x43E40000);
        assert_eq!(pb.data[1], 0x80000000);

        let rpn = el_ump_create_midi2_rpn(1, 2, 0x12, 0x34, 0x55667788);
        assert_eq!(rpn.data[0], 0x41221234);
        assert_eq!(rpn.data[1], 0x55667788);

        let nrpn = el_ump_create_midi2_nrpn(2, 3, 0x0A, 0x0B, 0x11223344);
        assert_eq!(nrpn.data[0], 0x42330A0B);
        assert_eq!(nrpn.data[1], 0x11223344);

        let rel_rpn = el_ump_create_midi2_relative_rpn(0, 1, 0x01, 0x02, -100);
        assert_eq!(rel_rpn.data[0], 0x40410102);
        assert_eq!(rel_rpn.data[1], (-100_i32) as u32);

        let rel_nrpn = el_ump_create_midi2_relative_nrpn(3, 4, 0x05, 0x06, 250);
        assert_eq!(rel_nrpn.data[0], 0x43540506);
        assert_eq!(rel_nrpn.data[1], 250_u32);
    }

    #[test]
    fn test_scaling_utilities() {
        assert_eq!(el_scale_up(64, 7, 32), 0x80000000);
        assert_eq!(el_scale_down(0x80000000, 32, 7), 64);
        assert_eq!(el_scale_up(127, 7, 16), 0xFFFF);
        assert_eq!(el_scale_down(0xFFFF, 16, 7), 127);
    }

    #[test]
    fn test_stream_parser_lifecycle_and_feeding() {
        let parser = el_stream_parser_create();
        assert!(!parser.is_null());

        let mut out_ump = el_ump_t { data: [0; 4] };

        // Test 1-word UMP (MT=0x2 Note On)
        let complete =
            unsafe { el_stream_parser_feed_word(parser, 0x20903C64, &mut out_ump as *mut _) };
        assert!(complete);
        assert_eq!(out_ump.data[0], 0x20903C64);
        assert_eq!(out_ump.data[1], 0);

        // Test 2-word UMP (MT=0x4 MIDI 2.0 CC)
        let complete =
            unsafe { el_stream_parser_feed_word(parser, 0x40B00700, &mut out_ump as *mut _) };
        assert!(!complete);
        let complete =
            unsafe { el_stream_parser_feed_word(parser, 0xAABBCCDD, &mut out_ump as *mut _) };
        assert!(complete);
        assert_eq!(out_ump.data[0], 0x40B00700);
        assert_eq!(out_ump.data[1], 0xAABBCCDD);

        // Test 4-word UMP (MT=0xF Stream)
        assert!(!unsafe { el_stream_parser_feed_word(parser, 0xF0000000, &mut out_ump as *mut _) });
        assert!(!unsafe { el_stream_parser_feed_word(parser, 0x11111111, &mut out_ump as *mut _) });
        assert!(!unsafe { el_stream_parser_feed_word(parser, 0x22222222, &mut out_ump as *mut _) });
        assert!(unsafe { el_stream_parser_feed_word(parser, 0x33333333, &mut out_ump as *mut _) });
        assert_eq!(out_ump.data[0], 0xF0000000);
        assert_eq!(out_ump.data[1], 0x11111111);
        assert_eq!(out_ump.data[2], 0x22222222);
        assert_eq!(out_ump.data[3], 0x33333333);

        // Test NULL safety
        assert!(!unsafe {
            el_stream_parser_feed_word(std::ptr::null_mut(), 0x1234, &mut out_ump as *mut _)
        });
        assert!(!unsafe { el_stream_parser_feed_word(parser, 0x1234, std::ptr::null_mut()) });

        unsafe {
            el_stream_parser_destroy(parser);
            // Calling destroy with null should not crash
            el_stream_parser_destroy(std::ptr::null_mut());
        }
    }
}

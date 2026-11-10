//! Streaming Zero-Allocation Deflate (RFC 1951) & zlib (RFC 1950) Decoder (WP-03)
//!
//! Provides `#![no_std]` zero-dynamic-allocation streaming decompression for
//! MIDI-CI Property Exchange (PE) JSON payloads transferred with
//! `"mediaType": "application/json; charset=utf-8; compression=zlib"`.
//!
//! Features:
//! - Const-generic bounded sliding window (`WINDOW_SIZE`, default 2048) and output buffer (`OUT_CAP`, default 1024).
//! - Streaming byte-by-byte or small chunk feeding (e.g. 13-byte SysEx8 / MDS packets).
//! - RFC 1950 zlib wrapper support with Adler-32 checksum calculation and verification.
//! - RFC 1951 Deflate support:
//!   - Non-compressed blocks (`BTYPE = 00`)
//!   - Fixed Huffman code blocks (`BTYPE = 01`)
//!   - Dynamic Huffman code blocks (`BTYPE = 10`)
//!   - LZ77 sliding window history
//! - Safe, panic-free error handling with `Result<_, PeDeflateError>`.

/// Errors that can occur during streaming Deflate or zlib decompression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeDeflateError {
    /// Invalid zlib header (CMF/FLG validation failed).
    InvalidZlibHeader,
    /// Unsupported compression method (only CM=8 Deflate is supported).
    UnsupportedCompressionMethod,
    /// Preset dictionary is requested in zlib header (FDICT set, unsupported in MIDI-CI).
    PresetDictionaryUnsupported,
    /// Invalid Deflate block type (`BTYPE = 11` reserved).
    InvalidBlockType,
    /// Non-compressed block length / one's complement check failed.
    InvalidStoredBlockLength,
    /// Invalid Huffman code encountered in stream.
    InvalidHuffmanCode,
    /// Invalid distance code or out-of-bounds backward reference.
    InvalidDistance,
    /// Dynamic Huffman code lengths configuration invalid.
    InvalidDynamicHeader,
    /// Output buffer capacity exceeded without caller draining bytes.
    OutputBufferFull,
    /// Adler-32 checksum mismatch at end of zlib stream.
    ChecksumMismatch,
    /// Internal decoding state error.
    InternalError,
}

/// Base length code lookup according to RFC 1951 Section 3.2.5.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];

/// Extra bits for length codes 257..=285.
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// Base distance code lookup according to RFC 1951 Section 3.2.5.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

/// Extra bits for distance codes 0..=29.
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Code length order for dynamic Huffman header according to RFC 1951 Section 3.2.7.
const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Compact Huffman decoding table for canonical Huffman codes.
#[derive(Clone, Copy)]
pub struct HuffmanTable {
    counts: [u16; 16],
    symbols: [u16; 288],
    max_bits: u8,
}

impl HuffmanTable {
    /// Constructs an empty Huffman table.
    pub const fn empty() -> Self {
        Self {
            counts: [0; 16],
            symbols: [0; 288],
            max_bits: 0,
        }
    }

    /// Builds a canonical Huffman decoding table from bit lengths.
    pub fn build(&mut self, bit_lengths: &[u8]) -> Result<(), PeDeflateError> {
        self.counts = [0; 16];
        self.symbols = [0; 288];
        self.max_bits = 0;

        let mut max_len = 0;
        for &len in bit_lengths {
            if len > 15 {
                return Err(PeDeflateError::InvalidHuffmanCode);
            }
            if len > 0 {
                self.counts[len as usize] += 1;
                if len > max_len {
                    max_len = len;
                }
            }
        }
        self.max_bits = max_len;

        // Compute starting symbol offset for each bit length
        let mut offsets = [0_usize; 16];
        let mut curr_offset = 0;
        for (len, off) in offsets.iter_mut().enumerate().skip(1) {
            *off = curr_offset;
            curr_offset += self.counts[len] as usize;
        }

        if curr_offset > 288 {
            return Err(PeDeflateError::InvalidHuffmanCode);
        }

        for (sym, &len) in bit_lengths.iter().enumerate() {
            if len > 0 {
                let off = offsets[len as usize];
                if off >= 288 {
                    return Err(PeDeflateError::InvalidHuffmanCode);
                }
                self.symbols[off] = sym as u16;
                offsets[len as usize] += 1;
            }
        }

        Ok(())
    }
}

/// Bit-level stream reader over byte slices.
#[derive(Clone, Copy, Default)]
struct BitReader {
    bit_buf: u32,
    bits_in_buf: u8,
}

impl BitReader {
    const fn new() -> Self {
        Self {
            bit_buf: 0,
            bits_in_buf: 0,
        }
    }

    fn push_byte(&mut self, byte: u8) {
        self.bit_buf |= (byte as u32) << self.bits_in_buf;
        self.bits_in_buf += 8;
    }

    fn peek_bits(&self, count: u8) -> u32 {
        if count == 0 {
            0
        } else if count >= 32 {
            self.bit_buf
        } else {
            self.bit_buf & ((1 << count) - 1)
        }
    }

    fn drop_bits(&mut self, count: u8) {
        let actual = if count > self.bits_in_buf {
            self.bits_in_buf
        } else {
            count
        };
        self.bit_buf >>= actual;
        self.bits_in_buf -= actual;
    }

    fn read_bits(&mut self, count: u8) -> u32 {
        let val = self.peek_bits(count);
        self.drop_bits(count);
        val
    }

    fn align_to_byte(&mut self) {
        let rem = self.bits_in_buf % 8;
        if rem != 0 {
            self.drop_bits(rem);
        }
    }
}

/// Rolling Adler-32 checksum computer (RFC 1950).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adler32 {
    s1: u32,
    s2: u32,
}

impl Default for Adler32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Adler32 {
    /// Creates a new Adler-32 computer initialized to 1.
    #[must_use]
    pub const fn new() -> Self {
        Self { s1: 1, s2: 0 }
    }

    /// Updates the Adler-32 checksum with a slice of bytes.
    pub fn update(&mut self, data: &[u8]) {
        const MOD_ADLER: u32 = 65521;
        for &b in data {
            self.s1 = (self.s1 + b as u32) % MOD_ADLER;
            self.s2 = (self.s2 + self.s1) % MOD_ADLER;
        }
    }

    /// Updates the Adler-32 checksum with a single byte.
    pub fn update_byte(&mut self, b: u8) {
        const MOD_ADLER: u32 = 65521;
        self.s1 = (self.s1 + b as u32) % MOD_ADLER;
        self.s2 = (self.s2 + self.s1) % MOD_ADLER;
    }

    /// Returns the current 32-bit Adler-32 checksum value.
    #[must_use]
    pub const fn finish(&self) -> u32 {
        (self.s2 << 16) | self.s1
    }
}

/// State of the Deflate decompressor state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DecodeState {
    ZlibCmf,
    ZlibFlg {
        cmf: u8,
    },
    BlockHeader,
    StoredLen1 {
        bfinal: bool,
    },
    StoredLen2 {
        bfinal: bool,
        len_lo: u8,
    },
    StoredNlen1 {
        bfinal: bool,
        len: u16,
    },
    StoredNlen2 {
        bfinal: bool,
        len: u16,
        nlen_lo: u8,
    },
    StoredData {
        bfinal: bool,
        remaining: u16,
    },
    DynamicHlitHdistHclen {
        bfinal: bool,
    },
    DynamicClenLengths {
        bfinal: bool,
        hlit: usize,
        hdist: usize,
        hclen: usize,
        cl_idx: usize,
    },
    DynamicLitDistLengths {
        bfinal: bool,
        hlit: usize,
        hdist: usize,
        total_lens: usize,
        curr_len_idx: usize,
    },
    HuffmanStream {
        bfinal: bool,
    },
    DecodeDistance {
        bfinal: bool,
        length: usize,
    },
    CopyMatch {
        bfinal: bool,
        dist: usize,
        remaining_len: usize,
    },
    ZlibChecksum {
        byte_idx: usize,
        expected: u32,
    },
    Done,
}

/// Fixed Huffman tables computed at const-time or initialization.
static FIXED_LIT_TABLE: HuffmanTable = {
    let mut table = HuffmanTable::empty();
    let mut lengths = [0_u8; 288];
    let mut i = 0;
    while i <= 143 {
        lengths[i] = 8;
        i += 1;
    }
    while i <= 255 {
        lengths[i] = 9;
        i += 1;
    }
    while i <= 279 {
        lengths[i] = 7;
        i += 1;
    }
    while i <= 287 {
        lengths[i] = 8;
        i += 1;
    }

    // Build manually for const evaluation
    let mut counts = [0_u16; 16];
    let mut symbols = [0_u16; 288];
    let mut j = 0;
    while j < 288 {
        counts[lengths[j] as usize] += 1;
        j += 1;
    }

    let mut offsets = [0_usize; 16];
    let mut curr_offset = 0;
    let mut l = 1;
    while l <= 15 {
        offsets[l] = curr_offset;
        curr_offset += counts[l] as usize;
        l += 1;
    }

    let mut k = 0;
    while k < 288 {
        let len = lengths[k] as usize;
        symbols[offsets[len]] = k as u16;
        offsets[len] += 1;
        k += 1;
    }

    table.counts = counts;
    table.symbols = symbols;
    table.max_bits = 9;
    table
};

static FIXED_DIST_TABLE: HuffmanTable = {
    let mut table = HuffmanTable::empty();
    let mut counts = [0_u16; 16];
    let mut symbols = [0_u16; 288];
    counts[5] = 32;
    let mut i = 0;
    while i < 32 {
        symbols[i] = i as u16;
        i += 1;
    }
    table.counts = counts;
    table.symbols = symbols;
    table.max_bits = 5;
    table
};

/// Streaming zero-allocation Deflate and zlib decompressor for Property Exchange.
///
/// Designed to decode incoming SysEx packets without dynamic heap memory allocation.
///
/// Parameters:
/// - `WINDOW_SIZE`: Size of LZ77 sliding history window in bytes (standard 2048 or up to 32768).
/// - `OUT_CAP`: Capacity of output buffer holding decompressed bytes awaiting consumption.
pub struct PeDeflateDecoder<const WINDOW_SIZE: usize = 2048, const OUT_CAP: usize = 1024> {
    is_zlib: bool,
    state: DecodeState,
    bit_reader: BitReader,
    adler: Adler32,
    window: [u8; WINDOW_SIZE],
    window_pos: usize,
    window_len: usize,
    out_buf: [u8; OUT_CAP],
    out_head: usize,
    out_tail: usize,

    // Dynamic Huffman decoding scratch tables
    lit_table: HuffmanTable,
    dist_table: HuffmanTable,
    cl_table: HuffmanTable,
    dyn_lengths: [u8; 320], // up to 288 literals + 32 distances
}

impl<const WINDOW_SIZE: usize, const OUT_CAP: usize> Default
    for PeDeflateDecoder<WINDOW_SIZE, OUT_CAP>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const WINDOW_SIZE: usize, const OUT_CAP: usize> PeDeflateDecoder<WINDOW_SIZE, OUT_CAP> {
    /// Creates a new streaming zlib decompressor (RFC 1950 with Adler-32).
    #[must_use]
    pub const fn new() -> Self {
        assert!(WINDOW_SIZE > 0, "WINDOW_SIZE must be greater than zero");
        assert!(OUT_CAP > 0, "OUT_CAP must be greater than zero");

        Self {
            is_zlib: true,
            state: DecodeState::ZlibCmf,
            bit_reader: BitReader::new(),
            adler: Adler32::new(),
            window: [0; WINDOW_SIZE],
            window_pos: 0,
            window_len: 0,
            out_buf: [0; OUT_CAP],
            out_head: 0,
            out_tail: 0,
            lit_table: HuffmanTable::empty(),
            dist_table: HuffmanTable::empty(),
            cl_table: HuffmanTable::empty(),
            dyn_lengths: [0; 320],
        }
    }

    /// Creates a new raw Deflate streaming decompressor without zlib header/checksum (RFC 1951).
    #[must_use]
    pub const fn new_raw() -> Self {
        assert!(WINDOW_SIZE > 0, "WINDOW_SIZE must be greater than zero");
        assert!(OUT_CAP > 0, "OUT_CAP must be greater than zero");

        Self {
            is_zlib: false,
            state: DecodeState::BlockHeader,
            bit_reader: BitReader::new(),
            adler: Adler32::new(),
            window: [0; WINDOW_SIZE],
            window_pos: 0,
            window_len: 0,
            out_buf: [0; OUT_CAP],
            out_head: 0,
            out_tail: 0,
            lit_table: HuffmanTable::empty(),
            dist_table: HuffmanTable::empty(),
            cl_table: HuffmanTable::empty(),
            dyn_lengths: [0; 320],
        }
    }

    /// Resets the decoder state to process a new stream.
    pub fn reset(&mut self) {
        self.state = if self.is_zlib {
            DecodeState::ZlibCmf
        } else {
            DecodeState::BlockHeader
        };
        self.bit_reader = BitReader::new();
        self.adler = Adler32::new();
        self.window_pos = 0;
        self.window_len = 0;
        self.out_head = 0;
        self.out_tail = 0;
    }

    /// Returns `true` if decompression has completed successfully.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state == DecodeState::Done
    }

    /// Returns the number of unread decompressed bytes available in the output buffer.
    #[must_use]
    pub fn available_decoded(&self) -> usize {
        self.out_tail.saturating_sub(self.out_head)
    }

    /// Returns a slice of the decoded bytes currently buffered and waiting to be consumed.
    #[must_use]
    pub fn peek_decoded(&self) -> &[u8] {
        &self.out_buf[self.out_head..self.out_tail]
    }

    /// Marks `count` bytes from the output buffer as consumed.
    pub fn consume_decoded(&mut self, count: usize) {
        let avail = self.available_decoded();
        let to_consume = if count > avail { avail } else { count };
        self.out_head += to_consume;

        if self.out_head == self.out_tail {
            self.out_head = 0;
            self.out_tail = 0;
        }
    }

    /// Reads decoded bytes into `out`, consuming them from the output buffer.
    ///
    /// Returns the number of bytes read.
    pub fn read_decoded(&mut self, out: &mut [u8]) -> usize {
        let avail = self.available_decoded();
        let to_read = if out.len() < avail { out.len() } else { avail };
        if to_read > 0 {
            out[..to_read].copy_from_slice(&self.out_buf[self.out_head..self.out_head + to_read]);
            self.consume_decoded(to_read);
        }
        to_read
    }

    fn push_output_byte(&mut self, b: u8) -> Result<(), PeDeflateError> {
        if self.out_tail >= OUT_CAP {
            return Err(PeDeflateError::OutputBufferFull);
        }

        self.out_buf[self.out_tail] = b;
        self.out_tail += 1;

        // Update sliding window
        self.window[self.window_pos] = b;
        self.window_pos = (self.window_pos + 1) % WINDOW_SIZE;
        if self.window_len < WINDOW_SIZE {
            self.window_len += 1;
        }

        // Update Adler32
        if self.is_zlib {
            self.adler.update_byte(b);
        }

        Ok(())
    }

    /// Peeks a single Huffman symbol and its bit length from a bit reader without consuming bits.
    fn peek_huffman_symbol(
        bit_reader: &BitReader,
        table: &HuffmanTable,
    ) -> Result<Option<(u16, u8)>, PeDeflateError> {
        if table.max_bits == 0 {
            return Err(PeDeflateError::InvalidHuffmanCode);
        }

        let mut code: u32 = 0;
        let mut first: u32 = 0;
        let mut index: usize = 0;

        for len in 1..=table.max_bits {
            if bit_reader.bits_in_buf < len {
                return Ok(None);
            }

            // In Deflate Huffman codes, bit order in stream is MSB first for the code,
            // but LSB first in the byte stream.
            let next_bit = (bit_reader.bit_buf >> (len - 1)) & 1;
            code = (code << 1) | next_bit;
            let count = table.counts[len as usize] as u32;

            if count > 0 && code >= first && code < first + count {
                let sym_idx = index + (code - first) as usize;
                if sym_idx >= 288 {
                    return Err(PeDeflateError::InvalidHuffmanCode);
                }
                return Ok(Some((table.symbols[sym_idx], len)));
            }

            index += count as usize;
            first = (first + count) << 1;
        }

        Err(PeDeflateError::InvalidHuffmanCode)
    }

    /// Feeds incoming compressed bytes in a streaming fashion.
    ///
    /// Returns `(bytes_consumed, bytes_produced)`.
    /// - `bytes_consumed`: number of bytes ingested from `input`.
    /// - `bytes_produced`: number of newly decompressed bytes written to output buffer.
    pub fn feed_stream(&mut self, input: &[u8]) -> Result<(usize, usize), PeDeflateError> {
        let initial_out = self.available_decoded();
        let mut in_idx = 0;

        while in_idx < input.len() || self.has_pending_work() {
            // Stop if output buffer is full
            if self.out_tail >= OUT_CAP && !matches!(self.state, DecodeState::Done) {
                break;
            }

            // Buffer bits if reader is low and input is available
            while self.bit_reader.bits_in_buf <= 24 && in_idx < input.len() {
                match self.state {
                    DecodeState::ZlibCmf
                    | DecodeState::ZlibFlg { .. }
                    | DecodeState::StoredLen1 { .. }
                    | DecodeState::StoredLen2 { .. }
                    | DecodeState::StoredNlen1 { .. }
                    | DecodeState::StoredNlen2 { .. }
                    | DecodeState::StoredData { .. }
                    | DecodeState::ZlibChecksum { .. } => break,
                    _ => {
                        self.bit_reader.push_byte(input[in_idx]);
                        in_idx += 1;
                    }
                }
            }

            let progress = self.step(input, &mut in_idx)?;
            if !progress {
                break;
            }
        }

        let new_produced = self.available_decoded().saturating_sub(initial_out);
        Ok((in_idx, new_produced))
    }

    /// Feeds a chunk of incoming compressed bytes into the decompressor.
    ///
    /// Returns the number of newly decompressed bytes produced into the output buffer.
    pub fn feed_bytes(&mut self, input: &[u8]) -> Result<usize, PeDeflateError> {
        let (_, produced) = self.feed_stream(input)?;
        Ok(produced)
    }

    fn has_pending_work(&self) -> bool {
        match self.state {
            DecodeState::CopyMatch { remaining_len, .. } => remaining_len > 0,
            DecodeState::Done => false,
            _ => self.bit_reader.bits_in_buf >= 3,
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn step(&mut self, input: &[u8], in_idx: &mut usize) -> Result<bool, PeDeflateError> {
        match self.state {
            DecodeState::ZlibCmf => {
                if *in_idx >= input.len() {
                    return Ok(false);
                }
                let cmf = input[*in_idx];
                *in_idx += 1;

                let cm = cmf & 0x0F;
                let cinfo = (cmf >> 4) & 0x0F;

                if cm != 8 {
                    return Err(PeDeflateError::UnsupportedCompressionMethod);
                }
                if cinfo > 7 {
                    return Err(PeDeflateError::InvalidZlibHeader);
                }

                self.state = DecodeState::ZlibFlg { cmf };
                Ok(true)
            }
            DecodeState::ZlibFlg { cmf } => {
                if *in_idx >= input.len() {
                    return Ok(false);
                }
                let flg = input[*in_idx];
                *in_idx += 1;

                let check = ((cmf as u16) << 8) | (flg as u16);
                if !check.is_multiple_of(31) {
                    return Err(PeDeflateError::InvalidZlibHeader);
                }

                let fdict = (flg & 0x20) != 0;
                if fdict {
                    return Err(PeDeflateError::PresetDictionaryUnsupported);
                }

                self.state = DecodeState::BlockHeader;
                Ok(true)
            }
            DecodeState::BlockHeader => {
                if self.bit_reader.bits_in_buf < 3 {
                    return Ok(false);
                }
                let bfinal = (self.bit_reader.read_bits(1)) != 0;
                let btype = self.bit_reader.read_bits(2);

                match btype {
                    0b00 => {
                        // Stored / uncompressed block
                        self.bit_reader.align_to_byte();
                        self.state = DecodeState::StoredLen1 { bfinal };
                        Ok(true)
                    }
                    0b01 => {
                        // Fixed Huffman block
                        self.lit_table = FIXED_LIT_TABLE;
                        self.dist_table = FIXED_DIST_TABLE;
                        self.state = DecodeState::HuffmanStream { bfinal };
                        Ok(true)
                    }
                    0b10 => {
                        // Dynamic Huffman block
                        self.state = DecodeState::DynamicHlitHdistHclen { bfinal };
                        Ok(true)
                    }
                    _ => Err(PeDeflateError::InvalidBlockType),
                }
            }
            DecodeState::StoredLen1 { bfinal } => {
                if self.bit_reader.bits_in_buf >= 8 {
                    let len_lo = self.bit_reader.read_bits(8) as u8;
                    self.state = DecodeState::StoredLen2 { bfinal, len_lo };
                    Ok(true)
                } else if *in_idx < input.len() {
                    let len_lo = input[*in_idx];
                    *in_idx += 1;
                    self.state = DecodeState::StoredLen2 { bfinal, len_lo };
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            DecodeState::StoredLen2 { bfinal, len_lo } => {
                let len_hi = if self.bit_reader.bits_in_buf >= 8 {
                    self.bit_reader.read_bits(8) as u8
                } else if *in_idx < input.len() {
                    let b = input[*in_idx];
                    *in_idx += 1;
                    b
                } else {
                    return Ok(false);
                };
                let len = ((len_hi as u16) << 8) | (len_lo as u16);
                self.state = DecodeState::StoredNlen1 { bfinal, len };
                Ok(true)
            }
            DecodeState::StoredNlen1 { bfinal, len } => {
                let nlen_lo = if self.bit_reader.bits_in_buf >= 8 {
                    self.bit_reader.read_bits(8) as u8
                } else if *in_idx < input.len() {
                    let b = input[*in_idx];
                    *in_idx += 1;
                    b
                } else {
                    return Ok(false);
                };
                self.state = DecodeState::StoredNlen2 {
                    bfinal,
                    len,
                    nlen_lo,
                };
                Ok(true)
            }
            DecodeState::StoredNlen2 {
                bfinal,
                len,
                nlen_lo,
            } => {
                let nlen_hi = if self.bit_reader.bits_in_buf >= 8 {
                    self.bit_reader.read_bits(8) as u8
                } else if *in_idx < input.len() {
                    let b = input[*in_idx];
                    *in_idx += 1;
                    b
                } else {
                    return Ok(false);
                };
                let nlen = ((nlen_hi as u16) << 8) | (nlen_lo as u16);
                if len != !nlen {
                    return Err(PeDeflateError::InvalidStoredBlockLength);
                }
                self.state = DecodeState::StoredData {
                    bfinal,
                    remaining: len,
                };
                Ok(true)
            }
            DecodeState::StoredData {
                bfinal,
                mut remaining,
            } => {
                while remaining > 0 {
                    if self.out_tail >= OUT_CAP {
                        return Ok(false);
                    }
                    let byte = if self.bit_reader.bits_in_buf >= 8 {
                        self.bit_reader.read_bits(8) as u8
                    } else if *in_idx < input.len() {
                        let b = input[*in_idx];
                        *in_idx += 1;
                        b
                    } else {
                        self.state = DecodeState::StoredData { bfinal, remaining };
                        return Ok(false);
                    };

                    self.push_output_byte(byte)?;
                    remaining -= 1;
                }

                if bfinal {
                    self.state = if self.is_zlib {
                        DecodeState::ZlibChecksum {
                            byte_idx: 0,
                            expected: 0,
                        }
                    } else {
                        DecodeState::Done
                    };
                } else {
                    self.state = DecodeState::BlockHeader;
                }
                Ok(true)
            }
            DecodeState::DynamicHlitHdistHclen { bfinal } => {
                if self.bit_reader.bits_in_buf < 14 {
                    return Ok(false);
                }
                let hlit = (self.bit_reader.read_bits(5) + 257) as usize;
                let hdist = (self.bit_reader.read_bits(5) + 1) as usize;
                let hclen = (self.bit_reader.read_bits(4) + 4) as usize;

                if hlit > 286 || hdist > 30 {
                    return Err(PeDeflateError::InvalidDynamicHeader);
                }

                self.dyn_lengths = [0; 320];
                self.state = DecodeState::DynamicClenLengths {
                    bfinal,
                    hlit,
                    hdist,
                    hclen,
                    cl_idx: 0,
                };
                Ok(true)
            }
            DecodeState::DynamicClenLengths {
                bfinal,
                hlit,
                hdist,
                hclen,
                mut cl_idx,
            } => {
                while cl_idx < hclen {
                    if self.bit_reader.bits_in_buf < 3 {
                        self.state = DecodeState::DynamicClenLengths {
                            bfinal,
                            hlit,
                            hdist,
                            hclen,
                            cl_idx,
                        };
                        return Ok(false);
                    }
                    let len = self.bit_reader.read_bits(3) as u8;
                    self.dyn_lengths[CL_ORDER[cl_idx]] = len;
                    cl_idx += 1;
                }

                let mut cl_lengths = [0_u8; 19];
                cl_lengths.copy_from_slice(&self.dyn_lengths[..19]);
                self.cl_table.build(&cl_lengths)?;

                self.dyn_lengths = [0; 320];
                self.state = DecodeState::DynamicLitDistLengths {
                    bfinal,
                    hlit,
                    hdist,
                    total_lens: hlit + hdist,
                    curr_len_idx: 0,
                };
                Ok(true)
            }
            DecodeState::DynamicLitDistLengths {
                bfinal,
                hlit,
                hdist,
                total_lens,
                mut curr_len_idx,
            } => {
                while curr_len_idx < total_lens {
                    let (sym, code_len) =
                        match Self::peek_huffman_symbol(&self.bit_reader, &self.cl_table)? {
                            Some(res) => res,
                            None => {
                                self.state = DecodeState::DynamicLitDistLengths {
                                    bfinal,
                                    hlit,
                                    hdist,
                                    total_lens,
                                    curr_len_idx,
                                };
                                return Ok(false);
                            }
                        };

                    let extra_needed = match sym {
                        16 => 2,
                        17 => 3,
                        18 => 7,
                        _ => 0,
                    };

                    if self.bit_reader.bits_in_buf < code_len + extra_needed {
                        self.state = DecodeState::DynamicLitDistLengths {
                            bfinal,
                            hlit,
                            hdist,
                            total_lens,
                            curr_len_idx,
                        };
                        return Ok(false);
                    }

                    self.bit_reader.drop_bits(code_len);

                    match sym {
                        0..=15 => {
                            self.dyn_lengths[curr_len_idx] = sym as u8;
                            curr_len_idx += 1;
                        }
                        16 => {
                            if curr_len_idx == 0 {
                                return Err(PeDeflateError::InvalidDynamicHeader);
                            }
                            let repeat = (self.bit_reader.read_bits(2) + 3) as usize;
                            let prev = self.dyn_lengths[curr_len_idx - 1];
                            for _ in 0..repeat {
                                if curr_len_idx >= total_lens {
                                    return Err(PeDeflateError::InvalidDynamicHeader);
                                }
                                self.dyn_lengths[curr_len_idx] = prev;
                                curr_len_idx += 1;
                            }
                        }
                        17 => {
                            let repeat = (self.bit_reader.read_bits(3) + 3) as usize;
                            for _ in 0..repeat {
                                if curr_len_idx >= total_lens {
                                    return Err(PeDeflateError::InvalidDynamicHeader);
                                }
                                self.dyn_lengths[curr_len_idx] = 0;
                                curr_len_idx += 1;
                            }
                        }
                        18 => {
                            let repeat = (self.bit_reader.read_bits(7) + 11) as usize;
                            for _ in 0..repeat {
                                if curr_len_idx >= total_lens {
                                    return Err(PeDeflateError::InvalidDynamicHeader);
                                }
                                self.dyn_lengths[curr_len_idx] = 0;
                                curr_len_idx += 1;
                            }
                        }
                        _ => return Err(PeDeflateError::InvalidDynamicHeader),
                    }
                }

                // Build literal and distance tables
                self.lit_table.build(&self.dyn_lengths[..hlit])?;
                self.dist_table
                    .build(&self.dyn_lengths[hlit..hlit + hdist])?;

                self.state = DecodeState::HuffmanStream { bfinal };
                Ok(true)
            }
            DecodeState::HuffmanStream { bfinal } => {
                if self.out_tail >= OUT_CAP {
                    return Ok(false);
                }

                let (sym, code_len) =
                    match Self::peek_huffman_symbol(&self.bit_reader, &self.lit_table)? {
                        Some(res) => res,
                        None => return Ok(false),
                    };

                match sym {
                    0..=255 => {
                        self.bit_reader.drop_bits(code_len);
                        self.push_output_byte(sym as u8)?;
                        Ok(true)
                    }
                    256 => {
                        self.bit_reader.drop_bits(code_len);
                        if bfinal {
                            self.bit_reader.align_to_byte();
                            self.state = if self.is_zlib {
                                DecodeState::ZlibChecksum {
                                    byte_idx: 0,
                                    expected: 0,
                                }
                            } else {
                                DecodeState::Done
                            };
                        } else {
                            self.state = DecodeState::BlockHeader;
                        }
                        Ok(true)
                    }
                    257..=285 => {
                        let len_idx = (sym - 257) as usize;
                        let extra_bits = LENGTH_EXTRA[len_idx];
                        if self.bit_reader.bits_in_buf < code_len + extra_bits {
                            return Ok(false);
                        }

                        self.bit_reader.drop_bits(code_len);
                        let extra = self.bit_reader.read_bits(extra_bits);
                        let length = LENGTH_BASE[len_idx] as usize + extra as usize;

                        self.state = DecodeState::DecodeDistance { bfinal, length };
                        Ok(true)
                    }
                    _ => Err(PeDeflateError::InvalidHuffmanCode),
                }
            }
            DecodeState::DecodeDistance { bfinal, length } => {
                let (dist_sym, code_len) =
                    match Self::peek_huffman_symbol(&self.bit_reader, &self.dist_table)? {
                        Some(res) => res,
                        None => return Ok(false),
                    };

                if dist_sym >= 30 {
                    return Err(PeDeflateError::InvalidDistance);
                }

                let dist_idx = dist_sym as usize;
                let dist_extra_bits = DISTANCE_EXTRA[dist_idx];
                if self.bit_reader.bits_in_buf < code_len + dist_extra_bits {
                    return Ok(false);
                }

                self.bit_reader.drop_bits(code_len);
                let dist_extra = self.bit_reader.read_bits(dist_extra_bits);
                let distance = DISTANCE_BASE[dist_idx] as usize + dist_extra as usize;

                if distance == 0 || distance > self.window_len {
                    return Err(PeDeflateError::InvalidDistance);
                }

                self.state = DecodeState::CopyMatch {
                    bfinal,
                    dist: distance,
                    remaining_len: length,
                };
                self.execute_copy_match()
            }
            DecodeState::CopyMatch { .. } => self.execute_copy_match(),
            DecodeState::ZlibChecksum {
                mut byte_idx,
                mut expected,
            } => {
                while byte_idx < 4 {
                    let b = if self.bit_reader.bits_in_buf >= 8 {
                        self.bit_reader.read_bits(8) as u8
                    } else if *in_idx < input.len() {
                        let byte = input[*in_idx];
                        *in_idx += 1;
                        byte
                    } else {
                        self.state = DecodeState::ZlibChecksum { byte_idx, expected };
                        return Ok(false);
                    };

                    expected = (expected << 8) | (b as u32);
                    byte_idx += 1;
                }

                let actual = self.adler.finish();
                if actual != expected {
                    return Err(PeDeflateError::ChecksumMismatch);
                }

                self.state = DecodeState::Done;
                Ok(true)
            }
            DecodeState::Done => Ok(false),
        }
    }

    fn execute_copy_match(&mut self) -> Result<bool, PeDeflateError> {
        let (bfinal, dist, mut remaining) = match self.state {
            DecodeState::CopyMatch {
                bfinal,
                dist,
                remaining_len,
            } => (bfinal, dist, remaining_len),
            _ => return Ok(false),
        };

        while remaining > 0 {
            if self.out_tail >= OUT_CAP {
                self.state = DecodeState::CopyMatch {
                    bfinal,
                    dist,
                    remaining_len: remaining,
                };
                return Ok(false);
            }

            // Window index backward reference
            let src_idx = if self.window_pos >= dist {
                self.window_pos - dist
            } else {
                WINDOW_SIZE + self.window_pos - dist
            };

            let b = self.window[src_idx];
            self.push_output_byte(b)?;
            remaining -= 1;
        }

        self.state = DecodeState::HuffmanStream { bfinal };
        Ok(true)
    }
}

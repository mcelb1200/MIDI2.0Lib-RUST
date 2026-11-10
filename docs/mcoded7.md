# Mcoded7 Encoding & Property Exchange Utilities

## Overview

In the MIDI 2.0 MIDI-CI (Capability Inquiry) specification and Property Exchange (PE), 8-bit binary data (such as compressed zlib/deflate streams or raw binary payloads) must often be transmitted across 7-bit MIDI byte streams. The **Mcoded7** encoding scheme packs 7 bytes of 8-bit data into 8 bytes of 7-bit data (MSB-first packaging).

## Algorithm

Every chunk of up to 7 bytes of 8-bit data is formatted into an 8-byte frame:
1. **Header Byte (Byte 0)**: Bit $i$ (for $i \in [0, 6]$) stores the Most Significant Bit (MSB, bit 7) of payload byte $i$.
2. **Data Bytes (Bytes 1..7)**: Store the lower 7 bits (`byte & 0x7F`) of each respective payload byte.

```text
Payload Bytes:   [B0, B1, B2, B3, B4, B5, B6]
Header Byte 0:   (MSB(B0)<<6) | (MSB(B1)<<5) | ... | MSB(B6)
Data Bytes 1..7: [B0 & 0x7F, B1 & 0x7F, ..., B6 & 0x7F]
```

## Rust Implementation

In `el_midi2`, Property Exchange utilizes zero-allocation streaming chunkers and reassemblers (`PeReplyChunker` and `PeDeflateDecoder` in `src/ci_pe.rs` and `src/ci_pe_deflate.rs`).

When dealing with 7-bit MUID and length packing:
- 28-bit MUIDs and 14-bit lengths are packed little-endian across 7-bit chunks (`val & 0x7F`, `(val >> 7) & 0x7F`).
- Broadcast MUID is `0x7F7F7F7F` (`0x0FFFFFFF`).

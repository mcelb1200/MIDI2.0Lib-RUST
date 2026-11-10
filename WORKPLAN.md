# Consolidated Master Workplan: Remaining Items & Agent Implementation Guide

## 1. Overview & Objective
This document outlines the master backlog of remaining items, feature expansions, interoperability bindings, and tool additions for the `el_midi2` workspace. It provides complete technical context, API designs, data structures, constraints, and test definitions so that an autonomous software engineering agent can execute and verify each task independently.

---

## 2. Core Architectural Principles & Invariants for Agents
When working on any task from this workplan, the agent **must** strictly enforce the following repository standards:
1. **Zero Dynamic Allocation in Hot Paths**:
   - The core protocol engines (`el_core`, `el_midi2`, and all `el_bsp_*` crates) must compile with `#![no_std]`.
   - Prefer passing byte/word slices (`&[u8]`, `&[u32]`) and bounded const-generic structures (e.g. `[T; CAP]`, `heapless::Vec`) instead of `alloc::vec::Vec` or `alloc::string::String`.
2. **UMP Endianness & Word Boundaries**:
   - Universal MIDI Packets (UMP) operate in 32-bit words (`[u32; 4]`). Wire transmissions (USB, Network, Serial) format words in Little-Endian byte order (`b0..b3`, where `b0` is LSB).
   - High nibbles of word 0 always encode Message Type (`MT = (w0 >> 28) & 0xF`) and Group (`Group = (w0 >> 24) & 0xF`).
3. **Panic-Free Safety**:
   - `#![forbid(unsafe_code)]` in all core logic and parsers. All failure conditions must return `Result<T, E>` or `Option<T>` without panicking or calling `.unwrap()` / `.expect()`.
   - Shifts must always guard against operand widths (`val < 32`).
4. **CI & Formatting Standards**:
   - Code must pass `cargo fmt -- --check`.
   - Code must pass `cargo clippy --all-targets --all-features -- -D warnings`.
   - C++ security scanner `python3 ./guardian.py` must pass with zero violations.

---

## 3. Workplan Task Matrix

| Task ID | Item Name | Category | Scope / Target Package | Priority |
|---|---|---|---|---|
| **WP-01** | Standard MIDI File 1.0 (SMF1) to MIDI Clip File (`.midiclip`) Converter Tool (`el_clip_conv`) | Tooling / Interop | `el_clip_conv` | High |
| **WP-02** | C/C++ FFI C-ABI Bindings (`el_midi2_c`) | Interoperability | `include/el_midi2.h`, `el_midi2_c/` | High |
| **WP-03** | Streaming Zero-Allocation Deflate / zlib Decoder for Compressed Property Exchange | Protocol Extension | `src/ci_pe_deflate.rs` (`el_midi2`) | Medium |
| **WP-04** | Native OS Virtual Endpoint Transports (Linux ALSA & macOS CoreMIDI / Windows MIDI Services) | Platform Transports | `el_bridge/src/native/` | Medium |
| **WP-05** | Real-Time Hardware Jitter Analyzer & Clock Offset Profiler CLI (`el_mon`) | Diagnostic Tooling | `el_mon` | ✅ Complete |

---

## 4. Detailed Implementation Blueprints for Agents

### 📌 Task WP-01: SMF1 (`.mid`) to MIDI Clip File (`.midiclip` / M2-116) Converter Tool (`el_clip_conv`)

#### 4.1 Objective
Build a standalone CLI tool `el_clip_conv` in the Cargo workspace that parses Standard MIDI Files (SMF Format 0 and Format 1, `.mid`) and translates them into MIDI 2.0 MIDI Clip Files (`.midiclip`, M2-116 standard) using `el_midi2::midiclip::MidiClipBuilder` and `el_core::utils::scale_up`.

#### 4.2 Specification Context & Mapping
- **SMF1 Input Format**:
  - 4-byte chunk ID `MThd` followed by 32-bit chunk length (6 bytes).
  - Format 0 (single multichannel track) or Format 1 (simultaneous tracks).
  - Division: Metric ticks per quarter note (`PPQN`).
  - Track chunks starting with 4-byte `MTrk`, followed by 32-bit length, containing delta-times (VLQ) and MIDI 1.0 status/data bytes or Meta-events (`0xFF`).
- **Translation Logic to UMP (M2-116 / MT=0x2 or MT=0x4)**:
  - Translate Channel Voice events (0x80..0xEF) into UMP MT=0x2 (or optionally up-convert to MT=0x4 with 16-bit velocity and 32-bit controllers via `--midi2`).
  - Convert Tempo meta-events (`0xFF 0x51 0x03 tt tt tt` microsec per quarter note) into Flex Data Common Tempo UMPs (MT=0xD, Bank 0x00, Status 0x00) with `tempo_10ns_pqn = microseconds * 100`.
  - Convert Track Name meta-events (`0xFF 0x03 len text...`) into Flex Data Track Metadata Name UMPs (MT=0xD, Bank 0x04, Status 0x00) using `FlexDataChunker`.
- **Output**: Write `.midiclip` binary file with magic header `SMF2` and sequential `MTrk` chunks.

#### 4.3 Proposed CLI Arguments (`el_clip_conv`)
```bash
el_clip_conv --input song.mid --output song.midiclip [--midi2] [--group 0]
```

#### 4.4 Test Verification
- Convert a sample SMF1 file into `.midiclip`.
- Read output back using `MultiTrackSequencer::from_clip_file`.
- Verify delta-times, tempo maps, and note values match precisely.

---

### 📌 Task WP-02: C/C++ FFI C-ABI Bindings (`el_midi2_c`)

#### 4.1 Objective
Expose core `el_midi2` and `el_core` Rust primitives and state machines via a stable C-ABI dynamic/static library (`libel_midi2.so` / `libel_midi2.a`) and accompanying C header (`include/el_midi2.h`) for integration with C++20 codebases and embedded RTOS firmware (FreeRTOS, Zephyr).

#### 4.2 Data Types & Functions to Expose
```c
#ifndef EL_MIDI2_H
#define EL_MIDI2_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t data[4];
} el_ump_t;

// Factory functions
el_ump_t el_ump_create_midi1_note_on(uint8_t group, uint8_t channel, uint8_t note, uint8_t velocity);
el_ump_t el_ump_create_midi2_note_on(uint8_t group, uint8_t channel, uint8_t note, uint8_t attribute_type, uint16_t velocity, uint16_t attribute_data);
el_ump_t el_ump_create_midi2_cc(uint8_t group, uint8_t channel, uint8_t index, uint32_t value);
el_ump_t el_ump_create_pitch_bend(uint8_t group, uint8_t channel, uint32_t value);

// Stream parser handle
typedef struct el_stream_parser_t el_stream_parser_t;
el_stream_parser_t* el_stream_parser_create(void);
bool el_stream_parser_feed_word(el_stream_parser_t* parser, uint32_t word, el_ump_t* out_ump);
void el_stream_parser_destroy(el_stream_parser_t* parser);

// Scaling utilities
uint32_t el_scale_up(uint32_t val, uint8_t src_bits, uint8_t dst_bits);
uint32_t el_scale_down(uint32_t val, uint8_t src_bits, uint8_t dst_bits);

#ifdef __cplusplus
}
#endif
#endif
```

#### 4.3 Implementation Strategy
- Create crate `el_midi2_c` with `crate-type = ["cdylib", "staticlib"]`.
- Annotate functions with `#[no_mangle] pub extern "C"`.
- Guard all pointer arguments against `NULL`.
- Catch any unwinding boundaries using `std::panic::catch_unwind`.
- Add test in `tests/test_c_bindings.cpp` linking with `g++` to verify C++ interoperability.

---

### 📌 Task WP-03: Streaming Zero-Allocation Deflate / zlib Decoder for Compressed Property Exchange

#### 4.1 Objective
Implement zero-allocation streaming decompression for Property Exchange JSON payloads transferred with `"mediaType": "application/json; charset=utf-8; compression=zlib"`.

#### 4.2 Specification Context & Constraints
- MIDI-CI Property Exchange allows compressing large JSON resource streams (such as `ProgramList` or `ControllerList` over 1KB) using RFC 1950 (zlib) or RFC 1951 (Deflate).
- Embedded constraint: Standard `flate2` / `miniz_oxide` requires substantial memory allocation.
- **Agent Blueprint**:
  - Implement a bounded, fixed-window streaming decompressor `PeDeflateDecoder<const WINDOW_SIZE: usize = 2048, const OUT_CAP: usize = 1024>`.
  - Process Huffman codes and LZ77 sliding window directly from incoming SysEx chunks.
  - Pipe decompressed output bytes into `PeJsonTokenizer` without assembling intermediate buffers in RAM.

#### 4.3 Test Verification
- Compress a 2KB JSON string with standard zlib.
- Feed chunks of 13 bytes (as SysEx8 / MDS packets) into `PeDeflateDecoder`.
- Assert decoded output matches original JSON string and parses properly with `PeSchemaValidator`.

---

### 📌 Task WP-04: Native OS Virtual Endpoint Transports (Linux ALSA & macOS CoreMIDI / Windows MIDI Services)

#### 4.1 Objective
Extend `el_bridge` with native operating system virtual MIDI 2.0 endpoint capabilities:
- **Linux**: ALSA Sequencer native UMP client (`snd_seq_open` with `SND_SEQ_OPEN_DUPLEX` and `SND_SEQ_USER_CLIENT`).
- **macOS**: CoreMIDI 2.0 `MIDIEndpointCreateWithBlock` declaring native UMP protocol (`kMIDIProtocol_2_0`).
- **Windows**: Windows 11 MIDI Services / UMP API bindings.

#### 4.2 Architecture
- Structure inside `el_bridge/src/native/`:
  - `alsa.rs`: Linux native ALSA sequencer UMP client.
  - `coremidi.rs`: macOS CoreMIDI 2.0 virtual endpoint.
  - `winmidi.rs`: Windows MIDI Services abstraction.
- Integrate with `el_bridge::translator::ToUmpTranslator` and `ToMidi1Translator`.

---

### 📌 Task WP-05: Real-Time Hardware Jitter Analyzer & Clock Offset Profiler CLI (`el_mon`)
* **Status in Codebase:** ✅ *Complete* (`el_mon/src/lib.rs:1-16`, `el_mon/src/jitter.rs:1-226`, `el_mon/src/network_clock.rs:1-196`, `el_mon/src/midi_clock.rs:1-140`, `el_mon/src/stats.rs:1-118`, `el_mon/src/main.rs:1-267`, `el_mon/tests/mon_tests.rs:1-160`).

#### 4.1 Objective
Audio-rate diagnostic CLI tool `el_mon` measuring clock drift, round-trip latency, and jitter across physical and network transports:
- Analyzes incoming Jitter Reduction timestamps (JR Clock / JR Timestamp) and Delta Clockstamps (DCT).
- For Network MIDI 2.0 / AppleMIDI, calculates round-trip latency from `CK0`, `CK1`, `CK2` timestamps:
  $$\text{Offset} = \frac{(t_1 + t_3)}{2} - t_2, \quad \text{RTT} = (t_3 - t_0) - (t_2 - t_1)$$
- Displays ANSI terminal live statistics: min, max, average jitter (in microseconds), dropped packets, and MIDI clock BPM drift.

---

## 5. Verification Checklist for Autonomous Agents
When picking up any item from this workplan, complete the following cycle:
1. Review this document and the referenced specifications in `MIDI_CORE_SPEC_MAPPING.md`.
2. Implement feature adhering to `#![no_std]` zero-allocation rules.
3. Write unit and integration tests under `tests/` or crate-specific test directories.
4. Verify workspace compilation: `cargo check --workspace` and `cargo test --workspace`.
5. Run linting and formatting: `cargo fmt -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
6. Run security guardian: `python3 ./guardian.py`.
7. Update `WORKPLAN.md` and `README.md` to mark the task as complete with exact line references.

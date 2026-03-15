# Phase A Implementation Plan: Foundation & Scaffolding

## 1. Objective
To scaffold the `am_midi2` workspace and develop the core foundation (`el_core`) for the `el_` suite of cross-platform MIDI 1.0 and 2.0 CLI tools. This phase also includes the implementation of the `el_dump` (Stream Analyzer) and `el_gen` (Message Generator) CLI utilities. The resulting code must be strictly `no_std` compatible, zero-allocation in hot paths, and suitable for stage-ready embedded hardware following MISRA 2023 equivalents.

## 2. Architecture & Workspace Scaffolding

The project will be structured as a Cargo Workspace containing multiple independent crates to enforce modularity and strict `no_std` boundaries.

### 2.1 Directory Structure
```text
/
├── Cargo.toml (Workspace definition)
├── el_core/
│   ├── Cargo.toml (Strictly #![no_std])
│   └── src/
│       ├── lib.rs
│       ├── ump.rs (UMP container & traits)
│       ├── parser.rs (Zero-allocation stream parser)
│       ├── builder.rs (Message generation factories)
│       ├── types.rs (Strongly-typed Enums/New Types)
│       └── utils.rs (Bit manipulation & scaling)
├── el_dump/
│   ├── Cargo.toml (CLI binary, uses std/clap)
│   └── src/
│       ├── main.rs
│       └── display.rs (Formatting logic)
└── el_gen/
    ├── Cargo.toml (CLI binary, uses std/clap)
    └── src/
        ├── main.rs
        └── commands.rs (CLI argument parsing)
```

### 2.2 Core Constraints (MISRA 2023 Equivalents in Rust)
The `el_core` crate will enforce the following top-level attributes in `lib.rs`:
```rust
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::alloc_instead_of_core)]
```
- **Zero Dynamic Allocation:** The `alloc` crate will *not* be used in `el_core`. All parsing and building will operate on statically sized arrays (e.g., `[u32; 4]`) or byte slices (`&[u8]`).
- **Error Handling:** Panic-free execution. Functions that can fail will return `Result<T, ErrorEnum>` or `Option<T>`.

## 3. Component Design: `el_core` (The Embedded Engine)

### 3.1 `ump.rs`: The Universal MIDI Packet
- **Design:** A struct wrapping a `[u32; 4]` representing the 128-bit UMP limit.
- **Strong Typing:** Use enums for `MessageType` (0x0 to 0xF).
- **Optimization:** Use inline constant arrays and bitmasks (e.g., `val & 0xF`) instead of branches for parsing message types and calculating word counts.

### 3.2 `parser.rs`: `UmpStreamParser`
- **Design:** An iterator-based parser that consumes a stream of `u32` words (`&[u32]` or iterator).
- **Safety:** Must explicitly check for truncated packets (e.g., returning `None` if a 128-bit message is missing words) without zero-padding, preventing data corruption.
- **Zero-Copy:** The parser will yield `Ump` structs containing the raw copied words. No heap allocation is required.

### 3.3 `builder.rs`: Message Generation
- **Design:** Factory methods for constructing valid MIDI 1.0 (MT=0x2) and MIDI 2.0 (MT=0x4) Channel Voice messages, System Common/Realtime messages, and Utility messages (MT=0x0).
- **Ergonomics:** All builder methods must be marked `#[must_use]`.

### 3.4 `utils.rs`: Bit Manipulation
- **Design:** Implement `scale_up` and `scale_down` functions for converting between 7-bit/8-bit/14-bit and 32-bit values.
- **Optimization:** Use unrolled bit-shifting and masking instead of loops for hot-path scaling operations.

## 4. Component Design: CLI Utilities

### 4.1 `el_dump` (Stream Analyzer)
- **Role:** Reads binary `.ump` files (or stdin) and decodes them into human-readable text.
- **Dependencies:** `el_core`, `clap` (for arguments), `crossterm` or `colored` (for CLI color coding).
- **Implementation:**
  - Read `u32` chunks from the input source.
  - Feed into `el_core::parser::UmpStreamParser`.
  - Format output showing Message Type, Group, Channel, Status, and Values.
  - Specifically handle edge cases like displaying NOOP messages (all zeros).

### 4.2 `el_gen` (Message Generator)
- **Role:** Generates precise binary UMP streams based on CLI arguments.
- **Dependencies:** `el_core`, `clap`.
- **Implementation:**
  - Subcommands for common message types: `note_on`, `cc`, `pitch_bend`.
  - Arguments for group, channel, value (with high-res support for MIDI 2.0).
  - Uses `el_core::builder` to construct the `Ump`.
  - Writes the raw `[u32]` data to stdout or a specified file.

## 5. Testing & Verification

### 5.1 Atomic Test Definitions (`el_core/tests/`)
1. **UMP Bounds & Length:** Verify `MessageType` enum correctly identifies 32, 64, 96, and 128-bit packet lengths.
2. **Parser Truncation:** Feed `UmpStreamParser` exactly 1 word of a 4-word message; assert it returns `None`.
3. **Scaling Accuracy:** Compare `utils::scale_up` against a known-good reference implementation for all 14-bit integer values to ensure bit-duplication is correct.
4. **Builder Bit-Packing:** Create a MIDI 2.0 Note On (velocity 0x8000); assert the underlying `u32` array exactly matches the MIDI 2.0 specification hex representation.

### 5.2 Integration Tests
- Run `el_gen note_on ... > test.ump`.
- Run `el_dump test.ump` and assert the stdout output matches the expected parsed representation.

## 6. Execution Steps
1. Create Cargo workspace and sub-crates.
2. Implement `el_core` constraints and primitive types.
3. Implement `el_core` utilities and tests.
4. Implement `el_core` builders and parsers.
5. Implement `el_dump` CLI.
6. Implement `el_gen` CLI.
7. Run complete test suite and verify no `unwrap/panic` in `el_core`.
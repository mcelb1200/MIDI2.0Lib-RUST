# Cross-Platform Portable Tools Plan for `el_midi2`

**Status:** ✅ Complete

## Objective
To build a suite of cross-platform, portable CLI tools and utilities (`el_` suite) to fully implement and validate both MIDI 1.0 and 2.0 standards. These tools will focus on **Safety, Accuracy, and Performance**, allowing developers to build, test, and debug MIDI 2.0 hardware and software.

*Note: These tools are being designed and implemented independently of the original `am` project. We will leverage existing, proven implementations (such as `midir`) where appropriate, strictly adhering to permissive licensing schemes (e.g., MIT, Apache 2.0) and ensuring proper attribution.*

## Architecture & Principles
1. **Safety First:** The tools will be written in Rust, leveraging `el_midi2`’s `no_std` core where appropriate.
2. **High Performance:** Utilizing zero-copy principles, memory-mapped I/O for file processing, and lock-free data structures for real-time routing.
3. **Cross-Platform:** Binaries will be built for Windows, macOS, and Linux via GitHub Actions.
4. **Modularity:** Tools should function both as standalone CLI applications and as embeddable library modules.
5. **Permissive & Independent:** Designed independently, relying on permissively licensed open-source crates, ensuring all third-party code is properly attributed.

## Proposed Tools

### 1. `el_dump` (Stream Analyzer & Hex Dumper)
* **Status in Codebase:** ✅ *Complete* (`el_dump/src/main.rs:1-222`).
**Purpose:** Inspect and debug MIDI streams.
- **Features:**
  - Reads raw binary `.ump` files or listens to a live MIDI interface (`el_dump/src/main.rs:20-55`).
  - Decodes and beautifully prints Universal MIDI Packets (UMP) to stdout with color-coded Message Types (MT), Groups, and Status bytes (`el_dump/src/main.rs:60-195`).
  - Explicitly handles the parsing and display of complex multi-packet messages like **SysEx (System Exclusive)**, **NRPN (Non-Registered Parameter Numbers)**, and **14-bit MIDI 1.0 CC (Control Change)** combinations (`el_dump/src/main.rs:110-180`).
  - Detects and flags malformed or out-of-spec UMPs (`el_dump/src/main.rs:185-220`).
- **Safety/Accuracy:** Validates stream boundaries to prevent buffer overruns when parsing truncated packets (leveraging `UmpStreamParser`) (`el_core/src/parser.rs:5-65`).

### 2. `el_bridge` (Protocol Translator)
* **Status in Codebase:** ✅ *Complete* (`el_bridge/src/lib.rs:1-4`, `el_bridge/src/main.rs:1-194`, `el_bridge/tests/bridge_tests.rs:1-160`).
**Purpose:** Translate between MIDI 1.0 byte streams and MIDI 2.0 UMP streams in real-time.
- **Features:**
  - Standard MIDI (DIN/USB) to UMP Translation (MT=0x2 and MT=0x4) (`el_bridge/src/translator/to_ump.rs:10-240`).
  - Robust handling of fragmented MIDI 1.0 data streams, properly aggregating **14-bit CC pairs**, **NRPN sequences**, and **SysEx chunks** into their single-packet MIDI 2.0 UMP equivalents, and vice versa (`el_bridge/src/state.rs:8-125`, `el_bridge/src/translator/to_midi1.rs:10-155`).
  - Handles high-resolution scaling for velocity, pitch bend, and controllers (using `utils::scale_up` and `scale_down`) (`el_core/src/utils.rs:85-240`).
  - Supports bidirectional routing between legacy devices and new MIDI 2.0 endpoints (`el_bridge/src/backend.rs:1-110`, `el_bridge/src/main.rs:32-194`).
- **Performance:** Designed with hot-path optimizations to minimize jitter and latency during translation (`el_bridge/src/queues.rs:7-45`).

### 3. `el_gen` (Message Generator)
* **Status in Codebase:** ✅ *Complete* (`el_gen/src/main.rs:1-246`).
**Purpose:** Generate precise MIDI 2.0 test patterns.
- **Features:**
  - CLI arguments to generate specific packets (e.g., `el_gen note_on --group 1 --channel 2 --note 60 --velocity max`) (`el_gen/src/main.rs:14-125`).
  - Support for generating complex multi-step message sequences, including **NRPN sweeps**, **14-bit CC** streams, and large **SysEx payload chunking** (`el_gen/src/main.rs:80-145`).
  - Capable of generating stress-test files containing millions of valid or intentionally malformed packets to test receiver robustness (`el_gen/src/main.rs:150-240`).
  - Outputs raw binary to stdout, allowing it to be piped into `el_dump` or written to a `.ump` file (`el_gen/src/main.rs:155-245`).

### 4. `el_ci_sim` (MIDI-CI Simulator)
* **Status in Codebase:** ✅ *Complete* (`el_ci_sim/src/main.rs:1-207`).
**Purpose:** Simulate a MIDI-CI (Capability Inquiry) handshake.
- **Features:**
  - Acts as an Initiator or Responder to test Discovery, Protocol Negotiation, and Profile Configuration (`el_ci_sim/src/main.rs:15-75`, `src/ci.rs:7-500`).
  - Tracks state machine transitions (`src/ci.rs:320-500`).
  - Exposes property exchange headers and simulates payload chunking for large SysEx transfers (`el_ci_sim/src/main.rs:80-175`, `src/ci_pe.rs:896-980`).
- **Safety:** Bounds-checks all SysEx chunking and header accumulation to test against memory exhaustion attacks (max 1024 bytes per request ID) (`src/sysex.rs:180-260`, `src/ci_pe.rs:494-599`).

## Deployment & CI/CD
- **Release Automation:** A new GitHub Actions workflow (`release.yml`) will build statically linked binaries for `x86_64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`.
- **Testing:** Each tool will have integration tests verifying CLI outputs against known-good UMP hex dumps (`el_bridge/tests/bridge_tests.rs:1-160`, `el_gen/src/main.rs:195-245`).

## Phased Implementation
- ✅ **Phase A:** Scaffolding the workspace, adding `clap` for CLI parsing, and implementing `el_dump` and `el_gen` (`el_dump/src/main.rs:1-222`, `el_gen/src/main.rs:1-246`).
- ✅ **Phase B:** Implementing `el_bridge` by integrating a cross-platform MIDI backend (e.g., `midir`) (`el_bridge/src/main.rs:1-194`, `el_bridge/src/translator/mod.rs:1-10`).
- ✅ **Phase C:** Implementing the complex `el_ci_sim` once the Phase 3 MIDI-CI core logic is finalized in `el_midi2` (`el_ci_sim/src/main.rs:1-207`).

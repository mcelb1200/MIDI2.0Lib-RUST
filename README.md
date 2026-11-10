# el_midi2

A Rust library for MIDI 2.0.

`el_midi2` is a general-purpose, `no_std` compatible library for building MIDI 2.0 Devices and Applications. It aims to work on everything from embedded devices to large-scale applications, providing the building blocks, processing, and translations needed for MIDI 2.0.

This library is a Rust port of an existing C++ MIDI 2.0 library, focusing on safety and ergonomics while maintaining a small footprint. It implements the core primitives, Universal MIDI Packets (UMP), stream parsing, and bit manipulation utilities according to the Phase 1 and Phase 2 roadmap goals.

## Workspace Architecture & State of the Codebase

`el_midi2` is structured as a modular Cargo workspace containing core libraries, device runtime engines, hardware Board Support Packages (BSPs), and portable CLI tooling:

### Workspace Crates

| Crate | Target / Environment | Description | Status |
|---|---|---|---|
| **`el_midi2`** (root) | `#![no_std]` (optional `alloc` / `std`) | High-level MIDI 2.0 device library, UMP factories, MIDI-CI engine, Property Exchange, and Clip sequencer. | ✅ Complete |
| **`el_core`** | Strict `#![no_std]` (zero-alloc) | Core embedded primitives: `Ump`, `MessageType`, `UmpStreamParser`, message builders (`VoiceBuilder`, `SystemBuilder`, `UtilityBuilder`, `FlexBuilder`), status byte types, and bit scaling. | ✅ Complete |
| **`el_bridge`** | CLI / `std` (`midir`, `ringbuf`, native ALSA / CoreMIDI / WinMIDI) | Real-time, thread-safe, lock-free bidirectional protocol translator between legacy MIDI 1.0 byte streams and MIDI 2.0 UMPs, with native OS virtual endpoint transports. | ✅ Complete |
| **`el_ci_sim`** | CLI / `std` | Cross-platform simulator and test harness for MIDI-CI handshakes, Property Exchange (Get, Set, Subscribe), and Process Inquiry reports. | ✅ Complete |
| **`el_dump`** | CLI / `std` | Stream analyzer and hex dumper for `.ump` streams and `.midiclip` files with color-coded packet inspection. | ✅ Complete |
| **`el_gen`** | CLI / `std` | Precise message pattern generator creating arbitrary MIDI 1.0/2.0 UMP streams, NRPN sweeps, and stress-test data files. | ✅ Complete |
| **`el_clip_conv`** | CLI / `std` | Standard MIDI File 1.0 (`.mid`) to MIDI 2.0 MIDI Clip File (`.midiclip`, M2-116) converter tool with MIDI 2.0 velocity & controller up-conversion. | ✅ Complete |
| **`el_bsp_esp32`** | `#![no_std]` (Xtensa & RISC-V) | Hardware peripheral drivers for ESP32, ESP32-S2/S3, ESP32-C3/C6/H2: UART FIFOs, USB-OTG MIDI 2.0 with GTB, and WiFi UDP Network MIDI. | ✅ Complete |
| **`el_bsp_rp2040`** | `#![no_std]` (ARM Cortex-M0+) | Raspberry Pi RP2040 drivers: PIO state-machine timed 31,250 baud MIDI 1.0 / serial UMP, and native USB-MIDI 2.0 with GTB. | ✅ Complete |
| **`el_bsp_stm32`** | `#![no_std]` (ARM Cortex-M) | STMicroelectronics STM32F4/G4/H7/F7 drivers: DMA circular buffer UART with baud calculations, and USB OTG_FS/OTG_HS device drivers. | ✅ Complete |
| **`el_bsp_nrf52`** | `#![no_std]` (ARM Cortex-M4F) | Nordic nRF52832/nRF52840/nRF52833 drivers: EasyDMA UARTE with baud calculation, USB USBD device driver, and BLE MIDI 2.0 GATT framing. | ✅ Complete |
| **`el_bsp_rpi3`** | `#![no_std]` (AArch64 bare metal) | Raspberry Pi 3B+ BCM2837B0 bare-metal drivers: 1 MHz System Timer (JR / AppleMIDI clock), Mailbox clock locking, PL011 UART0, DWC2 USB, and LAN7515 Ethernet. | ✅ Complete |

## Fully Implemented Features

### 1. Core Primitives & Streaming Parsers
- **Universal MIDI Packets (UMP)**: Full 32-bit word wrapper (`[u32; 4]`) supporting 32, 64, 96, and 128-bit packets with Little-Endian wire compliance.
- **`UmpStreamParser`**: Zero-allocation, iterator-based stream parser converting incoming `u32` words into validated UMPs with boundary truncation protection.
- **Bit Manipulation & Scaling**: Panic-free bit-duplication utilities (`scale_up` and `scale_down`) for 7-bit, 8-bit, 14-bit, 16-bit, and 32-bit values.

### 2. Message Factories & Protocol Types
- **MIDI 1.0 Channel Voice (MT=0x2)**: Note On/Off, Poly Pressure, Control Change, Program Change, Channel Pressure, Pitch Bend.
- **MIDI 2.0 Channel Voice (MT=0x4)**: High-resolution 16-bit velocity Note On/Off with attribute data, 32-bit Controller (CC), 32-bit Pitch Bend, RPN and NRPN parameters, Per-Note Pitch Bend, Per-Note Controllers, and Per-Note Management.
- **System Real-Time & System Common (MT=0x1)**: Timing Clock, Start, Continue, Stop, Active Sensing, Reset, MTC Quarter Frame, Song Position Pointer, Song Select, Tune Request.
- **Utility Messages (MT=0x0)**: NOOP, Jitter Reduction Clock, Jitter Reduction Timestamp, Delta Clockstamp Ticks (Status `0x3`), and Delta Clockstamp Since Last Event (Status `0x4`).
- **Flex Data Messages (MT=0xD)**: Common Tempo, Bar Marker, Beat Marker, Performance & Lyric text, Project Metadata (Project Name, Composition, Composer, Arranger, Copyright), and Track Metadata (Track/Channel/Clip Names, Marker Notes) with multi-packet chunking and reassembly.
- **Data 64 & Data 128 (MT=0x3 & MT=0x5)**: SysEx7 streaming chunker and reassembler, SysEx8 chunking, and Mixed Data Set (MDS) header/payload framing with multi-stream demultiplexing.

### 3. MIDI-CI (Capability Inquiry v1.2)
- **Universal SysEx Framing**: Standard 0x7E header serialization, 7-bit MUID packing/unpacking, and transaction tracking.
- **Discovery**: Inquiry and Reply generation, capability bitmasks, and initiator/responder state machines.
- **Protocol Negotiation**: Protocol Inquiry, Reply, and Stream Configuration switching state machine with timeout enforcement.
- **Profile Configuration**: Profile Inquiry, Profile Details Inquiry (`0x28`), Profile Details Reply (`0x29`), Profile Added (`0x26`), Profile Removed (`0x27`), Profile On/Off, and channel bitmask management.
- **Property Exchange (PE)**: Zero-allocation SAX-style JSON tokenizer, transaction manager (`PeTransactionManager`), subscription manager (`SubscriptionManager`), JSON schema validator (`PeSchemaValidator`), paginated querying (`offset`/`limit`), procedural resource writers (`DeviceInfo`, `ResourceList`, `ChannelList`, `Mode`, `ProgramList`, `LocalOn`, `DeviceState`, `ExternalSync`, `ControllerList`), and streaming zero-allocation Deflate / zlib decoder (`PeDeflateDecoder`) for compressed Property Exchange JSON streams.
- **Process Inquiry (PI)**: MIDI Message Report Inquiry and Reply generating 7-bit bitmaps for supported messages.

### 4. Turnkey Device Runtime & Profiles
- **`Midi2Device`**: Turnkey coordinator managing Endpoint info, up to `NUM_FB` Function Blocks, MUID, protocol negotiation, and transport polling.
- **Embedded Profile Handlers**: Standard embedded implementations for Default Profile, Drawbar Organ, Analog Synth, and General MIDI (GM1/GM2).

### 5. Multi-Track Sequencer & MIDI Clip Files (M2-116)
- **`.midiclip` File Format**: Parsing and serializing `SMF2` magic headers, track chunk structures (`MTrk`), and variable-length quantity (VLQ) delta times.
- **`MultiTrackSequencer`**: Concurrent multi-track scheduler merging independent tracks into a unified, chronological UMP stream with dynamic tempo map tracking from Flex Data Common Tempo messages.

### 6. Transports & Hardware Peripherals
- **USB-MIDI 2.0**: Standard and Class-Specific Group Terminal Block (GTB) descriptors and little-endian packet framing.
- **Serial DIN / TRS**: High-performance byte-to-UMP and UMP-to-byte conversion with running status tracking, real-time byte interleaving, and 14-bit CC / NRPN aggregation.
- **DMA Packet Queues**: Lock-free Single-Producer/Single-Consumer (SPSC) packet queues with zero-copy slice leasing.
- **Network Transports**: AppleMIDI session state machine (`IN`, `OK`, `NO`, `BY`), sample-accurate clock synchronization (`CK0`, `CK1`, `CK2`), and RTP-MIDI / Network MIDI 2.0 (M2-124) UDP packetization.
- **Microcontroller Board Support Crates**: Dedicated `#![no_std]` drivers for ESP32, RP2040, STM32, nRF52, and bare-metal Raspberry Pi 3B+.

### 7. Portable CLI Tool Suite
- **`el_dump`**: Hex dumper and stream inspector for `.ump` raw binary and `.midiclip` files.
- **`el_gen`**: Command-line generator for custom UMP messages and synthetic stress streams.
- **`el_bridge`**: Live bidirectional OS MIDI port routing, translation using `midir`, and native OS virtual MIDI 2.0 endpoints (Linux ALSA Sequencer UMP client, macOS CoreMIDI 2.0, and Windows 11 MIDI Services).
- **`el_clip_conv`**: Standard MIDI File (SMF 1.0 `.mid`) to MIDI 2.0 MIDI Clip File (`.midiclip`) converter tool.
- **`el_ci_sim`**: CLI simulator for testing Property Exchange requests and Capability Inquiry handshakes.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
el_midi2 = "0.1.0"
```

### Example: Creating Messages

```rust
use el_midi2::messages::UmpFactory;

fn main() {
    // Create a MIDI 2.0 Note On message
    // Group 0, Channel 0, Note 60, Attribute Type 0, Velocity 0x8000, Attribute Data 0
    let ump = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);

    println!("UMP Data: {:08X} {:08X}", ump.data[0], ump.data[1]);
}
```

### Example: Parsing a Stream

```rust
use el_midi2::buffer::UmpStreamParser;

fn main() {
    let raw_data = vec![0x20903C64, 0x40903C00, 0x80000000];
    let parser = UmpStreamParser::new(raw_data.into_iter());

    for ump in parser {
        println!("Received UMP: {:?}", ump);
    }
}
```

## Documentation

- **User Guide**: See [USER_GUIDE.md](USER_GUIDE.md) for detailed usage instructions across device runtime, transports, and CLI tools.
- **Consolidated Workplan**: Check out [WORKPLAN.md](WORKPLAN.md) for remaining future enhancements, FFI bindings, and agent implementation guides.
- **API Documentation**: Run `cargo doc --open` to view the generated API documentation across the workspace.
- **Development Roadmap**: Check out [MIDI2_ROADMAP.md](MIDI2_ROADMAP.md) for the complete implementation log and specification verification.
- **CLI Tools Plan**: Check out [TOOLS_PLAN.md](TOOLS_PLAN.md) for details on `el_dump`, `el_gen`, `el_bridge`, and `el_ci_sim`.

## Contributing

Contributions are welcome! If you find bugs, have feature requests, or want to improve documentation, please submit a Pull Request or Issue.

### License

This project is available under the MIT License.

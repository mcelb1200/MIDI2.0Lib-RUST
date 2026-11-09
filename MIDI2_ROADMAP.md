The following document is designed to be your Master Development Log. It serves two purposes:
1. **Project Management:** It breaks the massive MIDI 2.0 spec into atomic, sequential coding sessions.
2. **Context Injection:** It provides the technical constraints and definitions "Jules" (the AI) needs to maintain consistency across sessions.

**Note:** You should save this as MIDI2_ROADMAP.md in your repository root. At the start of each new AI session, copy the "Global Context" section and the specific "Task Definition" for that session.

# MIDI 2.0 Rust Implementation Pipeline

**Project Goal:** Build a fully compliant, no_std compatible, idiomatic Rust library for MIDI 2.0 (UMP, MIDI-CI, Transport Agnostic).

**Status:** 🔄 In Progress

## 🛠 Global Context (Paste at start of every session)

**Context for AI Session:**
We are building `el_midi2` (formerly midi2-rs), a comprehensive MIDI 2.0 library in Rust.

**Core Constraints:**
* **no_std Support:** The core library must support no_std (embedded targets). Use alloc feature flags only where necessary (e.g., large SysEx buffers).
* **Zero-Copy:** Prefer passing slices `&[u32]` and modifying Ump structs in place rather than heap allocation.
* **Strong Typing:** Use the "New Type" pattern or Enums to prevent invalid states (e.g., `struct Velocity(u16)`).
* **Endianness:** MIDI 2.0 UMP is always Little Endian on the wire (32-bit words). Ensure bit-shifting logic respects this.

**Architecture:**
* `ump.rs`: The raw container (`[u32; 4]`).
* `messages/`: Strongly typed builders (Voice, System, Utility).
* `ci/`: Capability Inquiry state machines and SysEx helpers.
* `transport/`: Traits for sending/receiving data.

## 📅 Phase 1: The Foundation (UMP & Primitive Types)
**Goal:** Establish the bit-level packet handling. No application logic yet.

### ✅ Session 1.1: The UMP Container
**Prompt:** "Implement the Ump struct wrapping `[u32; 4]`. Implement helper methods to get/set the Message Type (MT) and Group. Create the `MessageType` enum covering all 16 MIDI 2.0 types (Utility, System, MIDI 1.0, MIDI 2.0, Data, Stream, etc.). Ensure Debug and Display traits are useful for hex dumping."
**Definition of Done:** `Ump::new()`, `ump.message_type()`, and `ump.group()` work in unit tests.

### ✅ Session 1.2: Bit Manipulation Utilities
**Prompt:** "Create a utility module for bit packing. We need functions to extract/inject 7-bit, 14-bit, 16-bit, and 32-bit values into the UMP words. MIDI 2.0 relies heavily on splitting data across 32-bit boundaries. Add tests ensuring endianness is handled correctly."
**Definition of Done:** Helper functions `scale_up`, `scale_down`, etc., are tested and panic-free.

## 🎹 Phase 2: Protocol Logic (Voice Messages)
**Goal:** Ability to generate and parse musical data.

### ✅ Session 2.1: MIDI 2.0 Channel Voice (Note On/Off)
**Prompt:** "Implement the voice module. create builders/parsers for NoteOn and NoteOff. These are Message Type 0x4. Critical: Support 16-bit velocity and the Attribute fields (Attribute Type + Attribute Data). Use the builder pattern (e.g., `NoteOn::new(channel, note).velocity(v).build()`)."
**Definition of Done:** Unit tests verifying bit-perfect hex output against the spec examples.

### ✅ Session 2.2: Controllers & Pitch Bend
**Prompt:** "Extend the voice module. Implement ControlChange (Status 0xB), ProgramChange (Status 0xC), ChannelPressure (Status 0xD), and PitchBend (Status 0xE). Note: In MIDI 2.0, CC and Pitch Bend are 32-bit values. Ensure the API accepts u32 for these values."
**Definition of Done:** Ability to build a full suite of performance messages.

### ✅ Session 2.3: Jitter Reduction & Utility Messages
**Prompt:** "Implement Message Type 0x0 (Utility). Specifically, the JitterReductionClock and JitterReductionTimestamp messages. Implement the logic to prepend these timestamps to a Ump packet."
**Definition of Done:** Utility enum and logic to wrap a message in a JR timestamp.

## 🤝 Phase 3: MIDI-CI (Capability Inquiry)
**Goal:** The handshake. This is the hardest part involving multi-packet logic.

### ✅ Session 3.1: System Exclusive (SysEx) Manager
**Prompt:** "Implement the sysex module. MIDI 2.0 sends SysEx in 64-bit (MT 0x3) or 128-bit (MT 0x5) Data packets. We need a SysExBuilder that takes a `&[u8]` payload and returns an `Iterator<Item = Ump>` to chunk the data into packets (Start, Continue, End). Also need a SysExCollector state machine to reassemble incoming packets."
**Definition of Done:** A test that takes a long string, chunks it into 10 UMPs, and reassembles it back to the string.

### ✅ Session 3.2: Discovery & Protocol Negotiation
**Prompt:** "Implement the ci module basics. Define the Universal SysEx header for MIDI-CI (0x7E, 0x0D, ...). Create a struct CiDiscovery that generates the 'Discovery Inquiry' message. Create a state machine CiInitiator that transitions: Uninitialized -> DiscoverySent -> DiscoveryReceived -> Negotiating."
**Definition of Done:** A state machine that advances when fed simulated valid CI responses.

### ✅ Session 3.3: Profile Configuration
**Prompt:** "Implement MIDI-CI Profile Configuration messages. We need struct definitions for ProfileInquiry, ProfileOn, ProfileOff, and ProfileEnabledReport. These use the SysEx mechanism built in Session 3.1."
**Definition of Done:** Ability to generate the SysEx payload for turning on a specific Profile ID.

## 🔌 Phase 4: Stream & Transport
**Goal:** Discovery of endpoints and sending data.

### ✅ Session 4.1: UMP Stream Messages
**Prompt:** "Implement Message Type 0xF (Stream). This includes EndpointDiscovery, EndpointInfo, DeviceIdentity, and FunctionBlockInfo. These are crucial for the DAW to know what the device is (e.g., 'I am a synth on Group 1')."
**Definition of Done:** Structs that serialize to valid MT 0xF UMPs.

### ✅ Session 4.2: Function Blocks
**Prompt:** "Implement a FunctionBlock struct. This helps a device describe its internal topology. It needs to store: Block Direction (Input/Output), MI (MIDI 1/2), Group First, and Group Count."
**Definition of Done:** A helper that generates the Function Block Info Notification message.

### ✅ Session 4.3: Core Transport Traits & Mock Transport
**Prompt:** "Define a `Transport` trait and `AsyncTransport` trait suitable for both synchronous polling and interrupt/DMA driven embedded runtimes. Methods: `send(&[Ump]) -> Result<(), TransportError>` and `recv() -> Option<Ump>`. Implement a zero-allocation `MockTransport` with bounded internal ringbuffers for testing. Add integration test verifying endpoint discovery over `MockTransport`."
**Definition of Done:** `Transport` trait and `MockTransport` tested and working with zero allocations.

## 📦 Phase 5: MIDI-CI Property Exchange (PE) & Process Inquiry
**Goal:** Full bidirectional parameter exchange, discovery, and state synchronisation under strict embedded `no_std` constraints.

### ✅ Session 5.1: Zero-Allocation SAX/Streaming JSON Tokenizer
**Prompt:** "Implement a lightweight, no_std SAX-style JSON tokenizer in `ci::pe::json`. Capable of parsing Property Exchange headers and simple value bodies directly from byte slices without memory allocation (`alloc`). Support extracting string slices for `resource`, `resId`, `action`, `status`, and integer values."
**Definition of Done:** Tokenizer extracts keys and values from realistic PE JSON header payloads with zero heap allocation.

### ✅ Session 5.2: PE Transaction Manager & Request Pools
**Prompt:** "Implement `PeTransactionManager<const MAX_CONCURRENT: usize>` to manage asynchronous Request IDs without hash maps. Support matching Inquiries (Get, Set, Subscribe) to Replies and handling timeouts. Implement `PeHeader` and `PeMessage` view types."
**Definition of Done:** Unit test exercising concurrent Request ID allocation, matching, release, and pool exhaustion guards.

### ✅ Session 5.3: Property Chunking, Streaming & Subscriptions
**Prompt:** "Implement `PeReplyChunker` that streams arbitrary property bodies across multiple MT=0x3 (SysEx7) or MT=0x5 (SysEx8) UMP packets. Implement `SubscriptionManager<const MAX_SUBS: usize>` using resource string hashes (`u32`) for low-memory embedded subscriber tracking."
**Definition of Done:** A test that streams a simulated preset file across multiple UMP SysEx chunks and delivers notifications to registered subscribers.

### ✅ Session 5.4: MIDI-CI Process Inquiry (PI)
**Prompt:** "Implement MIDI-CI Process Inquiry messages: MIDI Message Report Inquiry/Reply (Sub-ID 0x40/0x41). This allows querying which MIDI 1.0 or MIDI 2.0 messages (Channel Voice, System Common, SysEx, etc.) are supported across groups and channels. Generate the 7-bit bitmap representation for supported messages."
**Definition of Done:** Serialization and parsing of MIDI Message Report Inquiry and Reply packets.

## 🔌 Phase 6: Embedded Transport Adapters & Hardware Interfaces
**Goal:** Direct hardware protocol bindings for physical microcontroller peripherals.

### ✅ Session 6.1: USB-MIDI 2.0 Class Specification & Group Terminal Blocks
**Prompt:** "Implement USB-MIDI 2.0 descriptor structures and packet framing in `transport::usb`. Define Group Terminal Block (GTB) descriptors (Standard and Class-Specific) describing group-to-terminal mappings, protocol capabilities (MIDI 1.0 / MIDI 2.0), and bidirectional association. Implement zero-copy USB packet unpackers/packers."
**Definition of Done:** Bit-accurate serialization of Group Terminal Block descriptors and packet framing tests against USB-MIDI 2.0 spec examples.

### ✅ Session 6.2: Serial UART / DIN / TRS Byte-Stream Transport & Bi-Directional Adapter
**Prompt:** "Implement the UART byte stream bridge (`transport::serial`). Provide `ByteStreamToUmp` and `UmpToByteStream` state machines adhering to `embedded-hal::serial` or raw byte slice buffers. Properly reconstruct running status, handle realtime byte interleaving, and aggregate 14-bit CCs, NRPNs, and SysEx."
**Definition of Done:** Bidirectional tests streaming legacy MIDI 1.0 serial bytes into UMPs and down-converting UMPs into compliant MIDI 1.0 byte streams without latency or heap allocation.

### ✅ Session 6.3: DMA-Friendly Ringbuffers & Interrupt-Safe Queues
**Prompt:** "Implement bounded, interrupt-safe lock-free SPSC queues (`transport::queue::PacketQueue<const CAP: usize>`) tailored for DMA transfer buffers. Provide zero-copy slice leasing for transmitting and receiving bursts of UMPs directly to/from hardware peripheral FIFOs."
**Definition of Done:** Stress tests verifying lock-free push/pop behavior across simulated concurrent producer and consumer contexts.

## 🎛 Phase 7: Embedded MIDI 2.0 Device Engine & High-Level Runtime
**Goal:** Turnkey embedded device runtime integrating Endpoint, Function Blocks, Profiles, and Transports into a unified system.

### ✅ Session 7.1: `Midi2Device` Coordinator
**Prompt:** "Implement a unified `Midi2Device<T: Transport, const NUM_FB: usize>` struct. Manages Endpoint info, up to `NUM_FB` Function Blocks, local MUID, CI state machine (Initiator + Responder), and active protocol. Provide a clean polling method `device.poll()` to process incoming UMPs and emit responses."
**Definition of Done:** High-level test simulating complete device startup: Endpoint Discovery -> Stream Config -> Function Block Discovery -> CI Discovery -> Protocol switch to MIDI 2.0.

### ✅ Session 7.2: Standard Embedded Profile Handlers
**Prompt:** "Implement standard profile managers for common embedded scenarios (e.g., Default Profile, Drawbar Organ, Analog Synth, General MIDI). Provide hooks for parameter updates and profile-specific data callbacks without heap allocations."
**Definition of Done:** Test enabling a standard profile on a Function Block and responding to Profile Inquiries and Profile On/Off messages.

### ✅ Session 7.3: Standard Resources & Device Identity Provider
**Prompt:** "Implement default Property Exchange resource providers: `DeviceInfo`, `ResourceList`, and `ChannelList`. Implement procedural string generators that write JSON chunks directly into SysEx UMP streams without assembling full JSON strings in RAM."
**Definition of Done:** End-to-end Property Exchange query for `DeviceInfo` yielding a valid, bit-perfect response stream parsed and verified against spec.

## ✅ Progress Log

- Initialized Rust port with Core primitives and utility methods, effectively completing Phase 1 and 2.
- Transitioned CI/CD workflows and developed `TOOLS_PLAN.md` for broader standard compliance and tooling support.
- Completed Phase 3 (SysEx7, SysEx8 / MDS, CI Discovery, Protocol Negotiation, and Profile Configuration).
- Completed Phase 4 (UMP Stream Messages, Endpoint Discovery/Notifications, Function Blocks, Function Block Discovery/Notifications, and Core Transport Traits & Mock Transport).
- Completed Phase 5 (MIDI-CI Property Exchange: Zero-allocation SAX JSON tokenizer, Transaction Manager, Subscription Manager, SysEx Chunker, and Process Inquiry).
- Completed Phase 6 (Embedded Transport Adapters & Hardware Interfaces: USB-MIDI 2.0 Group Terminal Blocks and packet framing, Serial UART/DIN/TRS bidirectional converters with running status and real-time interleaving, and DMA-friendly lock-free SPSC PacketQueue with zero-copy slice leasing).
- Completed Phase 7 (Embedded MIDI 2.0 Device Engine & High-Level Runtime: `Midi2Device` turnkey coordinator, standard embedded profile handlers for Drawbar Organ/Analog Synth/GM, and procedural zero-allocation Property Exchange resource providers for `DeviceInfo`, `ResourceList`, and `ChannelList`).
- Completed Phase 8 (Extended MIDI 2.0 Specifications & MIDI Clip File):
  - **M2-116**: MIDI Clip File Specification (`.midiclip`) header and event stream parser/builder (`src/midiclip.rs`).
  - **Comprehensive Protocol Types**: Exhaustive Status Byte enums for Channel Voice, System Common/Real-Time, Channel Mode CCs (120-127), and MIDI-CI Sub-ID 2 (`el_core::types`).
  - **Extended Property Exchange Resources**: Procedural zero-allocation JSON streaming for Mode (M2-106), ProgramList (M2-107), LocalOn (M2-109), DeviceState (M2-111), ExternalSync (M2-112), and ControllerList (M2-117).
  - **Hardware Transport Integration**: `EmbeddedTransport` trait for embedded-hal / embedded-io non-blocking peripherals.
  - **Tooling Support**: Extended `el_dump` and `el_gen` with `.midiclip` stream analysis and generation.

## 🔮 Future Roadmap & Extended Specifications Gap Analysis

The foundational and extended MIDI 2.0 specifications across the complete standard suite (M2-100 through M2-117, USB-MIDI 2.0, MIDI-CI v1.2, Property Exchange, and MIDI Clip Files) are fully implemented and verified in safe, idiomatic Rust. Future planned enhancements:

### ✅ Session 9.1: Network Transports (RTP MIDI / Network MIDI 2.0)
**Prompt:** "Implement RTP MIDI (RFC 6295) and Network MIDI 2.0 (M2-124) UDP UMP packetization and AppleMIDI session protocol state machines in `src/transport/network.rs`. Support session invitation sequences (`IN`, `OK`, `NO`, `BY`), clock synchronization (`CK0`, `CK1`, `CK2`), latency offset calculations, and zero-allocation packet packing/unpacking."
**Definition of Done:** Network transport handles session handshakes, clock synchronization offset calculation, and bit-accurate RTP UMP packet framing over UDP in unit tests.

### ✅ Session 9.2: Hardware Peripheral Drivers & Board Support Crates (ESP32 Xtensa & RISC-V)
**Prompt:** "Implement `el_bsp_esp32` board support crate covering Espressif ESP32 hardware peripherals for MIDI 2.0. Provide zero-allocation hardware UART FIFO drivers with running status and real-time interleaving for MIDI 1.0 (31,250 baud) and high-speed serial UMP, USB-OTG USB-MIDI 2.0 drivers with Group Terminal Blocks, and WiFi UDP Network MIDI 2.0 adapters for both Xtensa (ESP32, ESP32-S3) and RISC-V (ESP32-C3, ESP32-C6) architectures."
**Definition of Done:** `el_bsp_esp32` compiles in `no_std`, passes unit tests, and integrates seamlessly with `Transport` and `EmbeddedTransport`.

### ✅ Session 9.3: RP2040 Board Support Crate (`el_bsp_rp2040`)
**Prompt:** "Implement `el_bsp_rp2040` board support crate covering Raspberry Pi RP2040 hardware peripherals for MIDI 2.0. Provide direct PIO state-machine timed UART transmission and reception for 31,250 baud MIDI 1.0 and high-speed serial UMP with FIFOs, and USB-MIDI 2.0 native device drivers with Group Terminal Blocks."
**Definition of Done:** `el_bsp_rp2040` compiles in `no_std`, passes unit tests, and integrates seamlessly with `Transport` and `EmbeddedTransport`.

### ✅ Session 9.4: STM32 Board Support Crate (`el_bsp_stm32`)
**Prompt:** "Implement `el_bsp_stm32` board support crate covering STMicroelectronics STM32 hardware peripherals for MIDI 2.0 (ARM Cortex-M: STM32F4, STM32G4, STM32H7, STM32F7). Provide DMA-backed circular buffer UART drivers with running status and real-time interleaving for MIDI 1.0 (31,250 baud) and high-speed serial UMP with USART_BRR register calculations, and USB-MIDI 2.0 native device drivers for STM32 USB OTG_FS / OTG_HS cores with Group Terminal Blocks."
**Definition of Done:** `el_bsp_stm32` compiles in `no_std`, passes unit tests, and integrates seamlessly with `Transport` and `EmbeddedTransport`.

### ✅ Session 9.5: Nordic nRF52 Board Support Crate (`el_bsp_nrf52`)
**Prompt:** "Implement `el_bsp_nrf52` board support crate covering Nordic Semiconductor nRF52 microcontrollers (ARM Cortex-M4F: nRF52832, nRF52840, nRF52833). Provide EasyDMA UARTE drivers with running status and real-time interleaving for MIDI 1.0 (31,250 baud) and high-speed serial UMP with hardware BAUDRATE register calculations, USB-MIDI 2.0 native device drivers for nRF52 USBD cores with Group Terminal Blocks, and Bluetooth Low Energy (BLE) MIDI 2.0 packet framing adapters with 13-bit timestamp encapsulation."
**Definition of Done:** `el_bsp_nrf52` compiles in `no_std`, passes unit tests, and integrates seamlessly with `Transport` and `EmbeddedTransport`.

---

## 🔍 Detailed Specification Gap Analysis: Missing & Partial Implementations

An exhaustive examination of the codebase against the official MIDI Association MIDI 2.0 and MIDI-CI standard specifications reveals the following architectural areas where coverage is currently partial, foundational, or awaiting dedicated runtime bindings:

### 1. M2-104-UM: Mixed Data Set (MDS) Network & Large-Payload Transport (MT=0x5)
* **Status in Codebase:** ✅ *Complete*.
  - Dedicated zero-allocation struct representation `MdsHeader` (`MT=0x5, Status=0x8`) and payload helper `build_mds_payload_ump` (`Status=0x9`).
  - Streaming iterator `MdsChunker` emitting MDS Header and up to 14 payload bytes per packet.
  - Reassembly state machine `MdsReceiver<const CAP: usize>` and concurrent multi-stream demultiplexer `MdsDemuxReceiver<const CAP: usize, const NUM_STREAMS: usize>`.

### 2. M2-104-UM: Delta Clockstamps (DCT) & High-Resolution Jitter Reduction
* **Status in Codebase:** ✅ *Complete*.
  - Delta Clockstamp (DCT) messages (`MT=0x0, Status=0x3` ticks per quarter note, and `Status=0x4` ticks since last event) in `messages::UmpFactory` and `el_core::builder::UtilityBuilder`.
  - Zero-allocation `JitterReductionQueue<const CAP: usize>` and `ScheduledUmp` pairing JR timestamps and DCTs with Channel Voice and System messages for inbound and outbound scheduling.

### 3. M2-104-UM: Flex Data Messages Multi-Packet Reassembly & Full Metadata Banks (MT=0xD)
* **Status in Codebase:** ✅ *Complete*.
  - Zero-allocation multi-packet text chunker `FlexDataChunker` (Form 0=Complete, 1=Start, 2=Continue, 3=End) and reassembler `FlexDataReceiver<const CAP: usize>`.
  - Full Project & Track Metadata Status Banks: Status Bank `0x00` (Bar Marker, Beat Marker), Status Bank `0x03` (Project Name, Composition Name, Arranger, Composer, Copyright Notice), Status Bank `0x04` (Track / Channel Names, Clip Names, Marker Notes) in `el_core::types`, `el_core::builder::FlexBuilder`, and `messages::UmpFactory`.

### 4. M2-101-UM: MIDI-CI v1.2 Extended Profile Management
* **Status in Codebase:** *Complete*.
  - **Profile Details Inquiry (`0x28`) & Profile Details Reply (`0x29`)**: Negotiating per-profile capability bitmaps and configuration parameters.
  - **Profile Added (`0x26`) & Profile Removed (`0x27`) Notifications**: Dynamic profile advertisement during runtime.
  - Bounded multi-channel profile channel masking (`num_channels` bitmap allocation in CI v2 format and 16-channel bitmask management).

### 5. M2-104-UM: Stream Configuration & Endpoint Negotiation State Machine
* **Status in Codebase:** *Complete*.
  - Dynamic Stream Protocol Switch handshake engine (`StreamConfigStateMachine` in `src/stream.rs`): Active timeout handling when transitioning physical wire speeds or switching from MIDI 1.0 to MIDI 2.0 via Stream Configuration Request (`Status=0x005`) and verifying matching Notification (`Status=0x006`) prior to activating MT=0x4 message processing.

### 6. M2-103/105: Property Exchange (PE) Advanced Resource Handlers & Schema Validation
* **Status in Codebase:** *Complete*.
  - Zero-allocation JSON Schema validator (`PeSchemaValidator` in `src/ci_pe.rs`) for inbound `Set` requests rejecting malformed client parameter changes prior to hardware register commit.
  - Paginated property streaming for `ProgramList` and `ControllerList` querying using `offset` and `limit` JSON header properties without allocating memory.

### 7. M2-116: MIDI Clip File (`.midiclip`) Multi-Track Sequencer & Playback Engine
* **Status in Codebase:** ✅ *Complete*.
  - Clip header parsing/serialization (`MidiClipHeader`), variable-length quantity (VLQ) delta-time parser (`MidiClipTrackParser`), and builder (`MidiClipBuilder`) in `src/midiclip.rs`.
  - Simultaneous multi-track scheduler and playback engine (`MultiTrackSequencer`): Merging multiple concurrent `MTrk` chunks into a single chronological UMP stream with delta-time to microsecond/tick conversion and transport tempo changes (`FlexData` tempo map tracking, default 120 BPM, dynamic updates from MT=0xD Common Tempo messages).
  - High-precision metric ticks (`PPQN`) and SMPTE timecode frame calculations, full file parsing (`from_clip_file`), transport seeking (`seek_to_tick`), resetting (`reset`), and real-time playback ticking (`tick`).

---

## 🍓 Session 9.6: Complete Specification for Bare Metal Raspberry Pi 3B+ (`el_bsp_rpi3`)
* **Status in Codebase:** ✅ *Complete*.
  - Turnkey `#![no_std]` bare-metal crate `el_bsp_rpi3` targeting Broadcom BCM2837B0 SoC (ARM Cortex-A53 quad-core, AArch64 bare metal).
  - 1 MHz System Timer driver (`Rpi3SystemTimer`) converting ticks to AppleMIDI 100-microsecond network clock units and 16-bit Jitter Reduction timestamps.
  - VideoCore Mailbox 0 interface (`Rpi3Mailbox`) for 48 MHz UART reference clock locking and hardware MAC discovery.
  - GPIO controller driver (`Rpi3Gpio`) configuring GPIO 14/15 to `ALT0` and disabling pull-up/down resistors.
  - PL011 UART0 driver (`Rpi3UartDriver`) with exact fractional baud divisors for 31,250 baud (`96, 0`), 1 MBaud (`3, 0`), 3 MBaud (`1, 0`), and 115,200 baud (`26, 3`), zero-copy byte-to-UMP conversion, and running status reconstruction.
  - Synopsys DWC2 USB-MIDI 2.0 driver (`Rpi3UsbMidiDriver`) with 13-byte Group Terminal Block (GTB) descriptors and Little-Endian packet framing.
  - Microchip LAN7515 Ethernet Network MIDI 2.0 adapter (`Rpi3EthernetMidiAdapter`) with AppleMIDI session handling on UDP ports 5004/5005.

### 1. Architectural Overview & System Target
* **Target SoC:** Broadcom BCM2837B0 (same family as BCM2837, high-performance ARM Cortex-A53 quad-core 64-bit ARMv8-A).
* **Target Architecture:** `aarch64-unknown-none` (AArch64 bare metal, Exception Level 1 / EL1 or EL2).
* **System Clock / Oscillator:** 19.2 MHz crystal oscillator; GPU / VPU Core Clock dynamically governed or fixed at 250 MHz / 400 MHz via Mailbox; ARM Core clock up to 1.4 GHz.
* **Memory Management:** `#![no_std]` runtime with zero dynamic allocation; static MMIO registers and identity-mapped non-cacheable Device-nGnRnE memory regions for peripheral spaces.

### 2. Peripheral Base Addresses & Register Map (BCM2837B0)
On the Raspberry Pi 3B+ in 64-bit AArch64 mode, the physical MMIO base address is relocated to:
$$\text{MMIO\_BASE} = \mathtt{0x3F00\_0000}$$

| Peripheral | Base Address | Register Offset Details | Primary Function in MIDI 2.0 |
|---|---|---|---|
| **System Timer** | `0x3F00_3000` | `CS`: `+0x00`<br>`CLO`: `+0x04`<br>`CHI`: `+0x08`<br>`C0`-`C3`: `+0x0C`-`+0x18` | 1 MHz hardware counter (1 microsecond resolution) for UMP JR timestamps and AppleMIDI 100-microsecond clock sync. |
| **Mailbox 0 (ARM <- VC)** | `0x3F00_B880` | `READ`: `+0x00`<br>`STATUS`: `+0x18`<br>`WRITE`: `+0x20` | Firmware clock rate configuration, UART clock locking, MAC address acquisition for Network MIDI 2.0. |
| **ARM Interrupt Controller** | `0x3F00_B200` | `IRQ_BASIC_PEND`: `+0x00`<br>`ENABLE_IRQS1`: `+0x10`<br>`ENABLE_IRQS2`: `+0x14`<br>`DISABLE_IRQS1`: `+0x1C` | Hardware interrupt routing for UART RX/TX FIFOs and USB OTG events. |
| **GPIO Controller** | `0x3F20_0000` | `GPFSEL0`-`GPFSEL5`: `+0x00`-`+0x14`<br>`GPSET0`: `+0x1C`<br>`GPCLR0`: `+0x28`<br>`GPPUD`: `+0x94`<br>`GPPUDCLK0`: `+0x98` | Pin multiplexing for UART0 (GPIO 14 TX, GPIO 15 RX) and MIDI activity LEDs. |
| **PL011 UART0** | `0x3F20_1000` | `DR`: `+0x00`<br>`FR`: `+0x18`<br>`IBRD`: `+0x24`<br>`FBRD`: `+0x28`<br>`LCRH`: `+0x2C`<br>`CR`: `+0x30`<br>`IFLS`: `+0x34`<br>`IMSC`: `+0x38`<br>`MIS`: `+0x40`<br>`ICR`: `+0x44` | Primary high-precision MIDI 1.0 (31,250 baud) DIN/TRS transceiver and high-speed serial UMP link. |
| **Mini UART (AUX / UART1)** | `0x3F21_5000` | `AUX_ENABLES`: `+0x04`<br>`AUX_MU_IO`: `+0x40`<br>`AUX_MU_IER`: `+0x44`<br>`AUX_MU_IIR`: `+0x48`<br>`AUX_MU_LCR`: `+0x4C`<br>`AUX_MU_MCR`: `+0x50`<br>`AUX_MU_LSR`: `+0x54`<br>`AUX_MU_BAUD`: `+0x68` | Secondary serial MIDI port or diagnostic console output. |
| **Synopsys DWC2 USB OTG** | `0x3F98_0000` | `GAHBCFG`: `+0x08`<br>`GUSBCFG`: `+0x0C`<br>`GRSTCTL`: `+0x10`<br>`GINTSTS`: `+0x14`<br>`GINTMSK`: `+0x18`<br>`DAINT`: `+0x818`<br>`DIEPCTL0`: `+0x900`<br>`DOEPCTL0`: `+0xB00` | Native USB-MIDI 2.0 device/host controller with Group Terminal Blocks (GTB). |

---

### 3. PL011 UART0 Hardware Configuration & Exact Baud Divisor Formulas
The ARM PrimeCell PL011 UART provides fractional baud rate generation, 32-byte hardware FIFOs, and hardware parity/framing error detection, making it the ideal controller for jitter-free MIDI 1.0 (31,250 baud) and high-speed serial UMP.

#### 3.1 Mailbox Clock Locking
To prevent the GPU firmware from dynamically scaling the UART reference clock, `el_bsp_rpi3` issues a Mailbox property tag request (`TAG_SET_CLOCK_RATE = 0x00038002`, Clock ID `0x02` for UART) locking `UART_CLK` to a fixed **48,000,000 Hz** (48 MHz).

#### 3.2 Baud Rate Divisor Mathematics
The PL011 baud divisor is calculated as:
$$\text{Baud Divisor} = \frac{\text{UART\_CLK}}{16 \times \text{Target Baud}}$$
$$\text{IBRD} = \lfloor \text{Baud Divisor} \rfloor$$
$$\text{FBRD} = \lfloor (\text{Baud Divisor} - \text{IBRD}) \times 64 + 0.5 \rfloor$$

1. **Standard MIDI 1.0 (31,250 Baud):**
   $$\text{Baud Divisor} = \frac{48{,}000{,}000}{16 \times 31{,}250} = \frac{48{,}000{,}000}{500{,}000} = 96.0000$$
   - **Integer Part (`IBRD`):** `96` (`0x0060`)
   - **Fractional Part (`FBRD`):** `0` (`0x0000`)
   - **Baud Error:** **0.000%** (exact integer alignment, eliminating bit-rate drift and frame errors).

2. **High-Speed Serial UMP (1,000,000 Baud / 1 MBaud):**
   $$\text{Baud Divisor} = \frac{48{,}000{,}000}{16 \times 1{,}000{,}000} = 3.0000$$
   - **Integer Part (`IBRD`):** `3` (`0x0003`)
   - **Fractional Part (`FBRD`):** `0` (`0x0000`)
   - **Baud Error:** **0.000%**.

3. **High-Speed Serial UMP (3,000,000 Baud / 3 MBaud):**
   $$\text{Baud Divisor} = \frac{48{,}000{,}000}{16 \times 3{,}000{,}000} = 1.0000$$
   - **Integer Part (`IBRD`):** `1` (`0x0001`)
   - **Fractional Part (`FBRD`):** `0` (`0x0000`)
   - **Baud Error:** **0.000%**.

4. **Standard Diagnostic / Host Link (115,200 Baud):**
   $$\text{Baud Divisor} = \frac{48{,}000{,}000}{16 \times 115{,}200} \approx 26.041666$$
   - **Integer Part (`IBRD`):** `26` (`0x001A`)
   - **Fractional Part (`FBRD`):** $\lfloor 0.041666 \times 64 + 0.5 \rfloor = 3$ (`0x0003`)
   - **Calculated Baud:** $48{,}000{,}000 / (16 \times 26.046875) \approx 115{,}177.88$ Baud (**-0.019% error**, well within the 1.5% RS-232 tolerance).

#### 3.3 Pin Configuration (GPIO Alternate Functions)
* **GPIO 14 (Pin 8):** Set to `ALT0` (`TXD0`).
* **GPIO 15 (Pin 10):** Set to `ALT0` (`RXD0`).
* **Pull-up/down:** Set `GPPUD` to `0` (disable internal pull-up/down; external 5V/3.3V optocoupler circuit handles line termination).

---

### 4. Synopsys DWC2 USB Controller (USB-MIDI 2.0 Device/Host Driver)
The Raspberry Pi 3B+ routes its single OTG USB physical interface through a Microchip LAN7515 USB hub and Ethernet controller. In bare-metal device mode or when driving attached class-compliant USB-MIDI 2.0 synthesizers:

1. **Group Terminal Block (GTB) Class Descriptor:**
   - Standard 13-byte Class-Specific MS Interface Descriptor (Subtype `0x03`).
   - Declares bidirectional 16-group UMP pipe (`bLength = 13`, `bDescriptorType = 0x24`, `bDescriptorSubtype = 0x03`, `bGroupTrmBlkID = 0x01`, `bBlockType = 0x00`, `bGroupFirst = 0x00`, `bGroupLength = 16`, `bMidiProtocol = 0x02` for MIDI 2.0 with MIDI-CI).
2. **Endpoint Buffer Descriptor:**
   - Full-Speed / High-Speed bulk endpoints (Endpoint 1 IN / OUT) with 64-byte or 512-byte max packet sizes.
   - Zero-copy DMA packetization packing native Little-Endian 32-bit UMP words into USB packet buffers.

---

### 5. LAN7515 Ethernet Controller & Network MIDI 2.0 (AppleMIDI / RTP-MIDI)
The on-board Microchip LAN7515 provides 10/100/1000 Mbps Ethernet connected over USB:
1. **Network Stack:** `#![no_std]` zero-allocation UDP/IP stack (`smoltcp` integration or raw Ethernet framing).
2. **Ports:** UDP Port 5004 (Control Port) and 5005 (Data Port).
3. **Session Protocol:** Integrated with `src/transport/network.rs` (`AppleMidiSession`, `ClockSync`, and `RtpMidiHeader`), utilizing the 1 MHz BCM2837 System Timer (`CLO` register) to populate 64-bit 100-microsecond timestamps for `CK0`, `CK1`, and `CK2` clock synchronization packets.

---

### 6. Bare Metal Driver Architecture (`el_bsp_rpi3`)

The bare-metal Raspberry Pi 3B+ crate (`el_bsp_rpi3`) will be structured as follows:

```text
el_bsp_rpi3/
├── Cargo.toml (Strictly #![no_std], aarch64-unknown-none target)
└── src/
    ├── lib.rs (Top-level exports, BCM2837 peripheral base constants)
    ├── timer.rs (1 MHz hardware System Timer driver for JR timestamps)
    ├── mailbox.rs (Mailbox property interface for clock locking & board info)
    ├── gpio.rs (GPIO pinmux and function select)
    ├── uart.rs (PL011 UART0 driver with running status, real-time interleaving, and baud math)
    ├── usb.rs (DWC2 USB-MIDI 2.0 Group Terminal Block and packet framing driver)
    └── network.rs (Ethernet LAN7515 UDP Network MIDI 2.0 adapter)
```

#### 6.1 `Rpi3UartDriver` Implementation Blueprint
```rust
pub struct Rpi3UartDriver<const RX_CAP: usize, const TX_CAP: usize> {
    base_addr: usize,
    default_group: u8,
    baud_rate: Rpi3BaudRate,
    byte_to_ump: ByteStreamToUmp,
    ump_to_byte: UmpToByteStream,
    rx_queue: PacketQueue<RX_CAP>,
    tx_queue: PacketQueue<TX_CAP>,
}
```
- Implements `Transport` (`send(&[Ump])`, `recv() -> Option<Ump>`).
- Implements `EmbeddedTransport` (`read_ump()`, `write_ump()`, `flush()`).
- Provides direct hardware FIFO accessors: `read_dr()`, `write_dr()`, `is_tx_fifo_full()`, `is_rx_fifo_empty()`.

---

### 7. Definition of Done & Test Suite for `el_bsp_rpi3`
* **Test 1: Baud Divisor Precision (`test_rpi3_baud_divisors`)**: Assert integer (`IBRD`) and fractional (`FBRD`) registers evaluate to exact theoretical values for 31,250 baud (`96, 0`), 1 MBaud (`3, 0`), 3 MBaud (`1, 0`), and 115,200 baud (`26, 3`) under 48 MHz clock.
* **Test 2: System Timer Timestamp Precision (`test_rpi3_system_timer_math`)**: Verify conversion of 1 MHz hardware counter ticks to 100-microsecond AppleMIDI network clock units and 16-bit Jitter Reduction timestamps.
* **Test 3: UART0 Loopback Simulation (`test_rpi3_uart_loopback`)**: Stream MIDI 1.0 Note On/Off bytes and high-speed UMP frames through the PL011 driver FIFO buffer, verifying zero-copy byte-to-UMP translation and running status reconstruction.
* **Test 4: USB-MIDI 2.0 GTB Descriptor Serialization (`test_rpi3_usb_gtb_descriptors`)**: Verify byte-accurate USB-MIDI 2.0 Group Terminal Block descriptors and Little-Endian packet framing.
* **Test 5: Zero-Allocation Interface Compliance**: Validate `#![no_std]` compilation and zero heap allocation across all drivers.

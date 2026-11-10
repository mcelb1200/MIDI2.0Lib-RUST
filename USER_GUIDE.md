# User Guide for el_midi2

## Introduction

`el_midi2` is a lightweight, `no_std` compatible Rust library for working with MIDI 2.0. It provides the building blocks for creating MIDI 2.0 devices and applications, handling Universal MIDI Packets (UMP), message creation, and stream parsing.

This library is designed to be efficient and portable, suitable for embedded systems as well as desktop applications.

## Getting Started

### Installation

Add `el_midi2` to your `Cargo.toml`:

```toml
[dependencies]
el_midi2 = { path = "." } # Or the crates.io version when available
```

### Basic Concepts

The core of MIDI 2.0 is the Universal MIDI Packet (UMP), which is a 32-bit word based packet structure. UMPs can vary in size from 32 bits (1 word) to 128 bits (4 words).

- **UMP**: The fundamental data structure, holding up to 128 bits.
- **MessageType**: Determines the format and length of the UMP (e.g., MIDI 1.0 Channel Voice, MIDI 2.0 Channel Voice, SysEx).
- **Group**: A 4-bit value allowing up to 16 separate MIDI streams on a single connection.

## Usage

### Creating Messages

The `messages::UmpFactory` provides helper methods to create common MIDI messages, ranging from MIDI 1.0 and 2.0 channel voice messages to System and Utility messages.

#### MIDI 1.0 Channel Voice

```rust
use el_midi2::messages::UmpFactory;

// Create a Note On message on Group 0, Channel 0, Note 60 (Middle C), Velocity 100
let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
```

#### MIDI 2.0 Channel Voice

```rust
use el_midi2::messages::UmpFactory;

// Create a MIDI 2.0 Note On message with high-resolution velocity
// Group 0, Channel 0, Note 60, Attribute Type 0, Velocity 0x8000 (mid), Attribute Data 0
let m2_note_on = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);

// Create a MIDI 2.0 Pitch Bend message
// Group 0, Channel 0, 32-bit Pitch Bend Value
let pitch_bend = UmpFactory::midi2_pitch_bend(0, 0, 0x80000000);
```

#### System Common & Utility Messages

```rust
use el_midi2::messages::UmpFactory;

// Create a MIDI Timing Clock message on Group 0
let clock = UmpFactory::timing_clock(0);

// Create a Jitter Reduction Timestamp message
let jr_timestamp = UmpFactory::jr_timestamp(12345);
```

### Parsing Streams

The `buffer::UmpStreamParser` allows you to parse a stream of 32-bit integers into `Ump` structs. This is useful when reading data from a hardware interface or network stream.

```rust
use el_midi2::buffer::UmpStreamParser;
use el_midi2::ump::MessageType;

let raw_data = vec![
    0x20903C64, // MIDI 1.0 Note On
    0x40903C00, 0x80000000 // MIDI 2.0 Note On (2 words)
];

let parser = UmpStreamParser::new(raw_data.into_iter());

for ump in parser {
    match ump.message_type() {
        MessageType::Midi1ChannelVoice => {
            println!("Received MIDI 1.0 Message: {:?}", ump);
        }
        MessageType::Midi2ChannelVoice => {
            println!("Received MIDI 2.0 Message: {:?}", ump);
        }
        _ => {}
    }
}
```

### Utility Functions

`el_midi2` provides utility functions for bit scaling, which is crucial for converting between MIDI 1.0 (7-bit) and MIDI 2.0 (16-bit or 32-bit) values. The library includes optimized fast paths for common parameters in rust hot paths.

```rust
use el_midi2::utils::{scale_up, scale_down};

let val_7bit = 0x7F;
let val_32bit = scale_up(val_7bit, 7, 32); // Scales to 0xFFFFFFFF
let back_to_7bit = scale_down(val_32bit, 32, 7); // Scales back to 0x7F
```

## Advanced Usage

### Custom UMP Manipulation

You can manually inspect and modify UMP data using the `Ump` struct methods.

```rust
use el_midi2::ump::{Ump, MessageType};

let mut ump = Ump::new();
ump.set_message_type(MessageType::Midi1ChannelVoice);
ump.set_group(1);
// Manually set data bytes if needed, though Factory methods are preferred.
```

### Turnkey Embedded Runtime: `Midi2Device`

`el_midi2::device::Midi2Device` provides a complete, turnkey coordinator integrating Endpoint Discovery, Stream Configuration protocol switching, Function Block negotiation, and MIDI-CI responder / initiator roles over any `Transport`.

```rust
use el_midi2::device::Midi2Device;
use el_midi2::transport::MockTransport;

// Initialize mock or hardware transport
let transport = MockTransport::<64>::new();

// Create a device with up to 2 Function Blocks and a unique 28-bit MUID
let mut device = Midi2Device::new(transport, 0x1234567);

// Poll device periodically in an event loop or timer ISR
device.poll();
```

### MIDI-CI Capability Inquiry, Profiles, and Property Exchange

The library provides zero-allocation state machines and data structures for MIDI-CI v1.2:

```rust
use el_midi2::ci::{CiDiscovery, CiInitiator, CiProfile};
use el_midi2::ci_pe::{PeTransactionManager, SubscriptionManager};

// 1. Transaction Pool for asynchronous Property Exchange (up to 8 concurrent requests)
let mut pe_mgr = PeTransactionManager::<8>::new();
if let Some(req_id) = pe_mgr.start_request(0x2345678) {
    // Dispatch PE Get or Set Inquiry
    pe_mgr.complete_request(req_id).ok();
}

// 2. Subscription Manager tracking resource subscriber MUIDs
let mut subs = SubscriptionManager::<8>::new();
let dev_info_hash = el_midi2::ci_pe::hash_resource("DeviceInfo");
subs.add_subscription(0x2345678, dev_info_hash).ok();
```

### Multi-Track MIDI Clip File Sequencing (`.midiclip`)

The `midiclip` module supports parsing, building, and scheduling multi-track MIDI Clip Files (M2-116 standard) with dynamic tempo tracking:

```rust
use el_midi2::midiclip::{ClipTimeFormat, MidiClipBuilder, MultiTrackSequencer};
use el_midi2::messages::UmpFactory;

// Create track events
let note = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);

// Build a multi-track clip
let mut builder = MidiClipBuilder::new(ClipTimeFormat::TicksPerQuarterNote(480));
builder.add_event(0, 0, note).unwrap(); // Track 0, delta tick 0
let clip_data = builder.build().unwrap();

// Play back tracks concurrently using MultiTrackSequencer
let mut seq = MultiTrackSequencer::<4>::new(ClipTimeFormat::TicksPerQuarterNote(480));
seq.add_track(b"\x00\x40\x90\x3C\x00\x80\x00\x00\x00").unwrap();

while let Some(event) = seq.tick(1) {
    println!("Scheduled UMP: {:?} at tick {}", event.ump, event.tick);
}
```

### Microcontroller Board Support Packages (BSPs)

When developing for microcontrollers, import the appropriate BSP crate for turnkey zero-allocation drivers:

- **`el_bsp_esp32`**: ESP32 (Xtensa & RISC-V) UART FIFOs, USB-OTG with Group Terminal Blocks, and WiFi UDP Network MIDI.
- **`el_bsp_rp2040`**: Raspberry Pi RP2040 PIO state-machine UART and native USB-MIDI 2.0 with GTB.
- **`el_bsp_stm32`**: STM32 DMA circular buffer UART and USB OTG_FS/OTG_HS drivers.
- **`el_bsp_nrf52`**: Nordic nRF52 EasyDMA UARTE, native USBD, and BLE MIDI 2.0 GATT framing.
- **`el_bsp_rpi3`**: Raspberry Pi 3B+ BCM2837B0 bare-metal PL011 UART0, 1 MHz System Timer, and LAN7515 Ethernet.

### Portable CLI Tools

The workspace includes four cross-platform CLI utilities:

1. **`el_dump`**: Stream analyzer & hex dumper.
   ```bash
   cargo run -p el_dump -- --file capture.ump
   ```
2. **`el_gen`**: Message pattern generator.
   ```bash
   cargo run -p el_gen -- note-on --group 0 --channel 0 --note 60 --velocity 32768
   ```
3. **`el_bridge`**: Live bidirectional MIDI 1.0 <-> MIDI 2.0 protocol translator.
   ```bash
   cargo run -p el_bridge -- --in-port 0 --out-port 1
   ```
4. **`el_ci_sim`**: MIDI-CI Property Exchange and Process Inquiry simulator.
   ```bash
   cargo run -p el_ci_sim -- pe get --resource "DeviceInfo"
   ```

## Support

If you encounter any issues or have questions, please check the repository documentation or submit an issue.

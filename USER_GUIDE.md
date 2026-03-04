# User Guide for am_midi2

## Introduction

`am_midi2` is a lightweight, `no_std` compatible Rust library for working with MIDI 2.0. It provides the building blocks for creating MIDI 2.0 devices and applications, handling Universal MIDI Packets (UMP), message creation, and stream parsing.

This library is designed to be efficient and portable, suitable for embedded systems as well as desktop applications.

## Getting Started

### Installation

Add `am_midi2` to your `Cargo.toml`:

```toml
[dependencies]
am_midi2 = { path = "." } # Or the crates.io version when available
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
use am_midi2::messages::UmpFactory;

// Create a Note On message on Group 0, Channel 0, Note 60 (Middle C), Velocity 100
let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
```

#### MIDI 2.0 Channel Voice

```rust
use am_midi2::messages::UmpFactory;

// Create a MIDI 2.0 Note On message with high-resolution velocity
// Group 0, Channel 0, Note 60, Attribute Type 0, Velocity 0x8000 (mid), Attribute Data 0
let m2_note_on = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);

// Create a MIDI 2.0 Pitch Bend message
// Group 0, Channel 0, 32-bit Pitch Bend Value
let pitch_bend = UmpFactory::midi2_pitch_bend(0, 0, 0x80000000);
```

#### System Common & Utility Messages

```rust
use am_midi2::messages::UmpFactory;

// Create a MIDI Timing Clock message on Group 0
let clock = UmpFactory::timing_clock(0);

// Create a Jitter Reduction Timestamp message
let jr_timestamp = UmpFactory::jr_timestamp(12345);
```

### Parsing Streams

The `buffer::UmpStreamParser` allows you to parse a stream of 32-bit integers into `Ump` structs. This is useful when reading data from a hardware interface or network stream.

```rust
use am_midi2::buffer::UmpStreamParser;
use am_midi2::ump::MessageType;

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

`am_midi2` provides utility functions for bit scaling, which is crucial for converting between MIDI 1.0 (7-bit) and MIDI 2.0 (16-bit or 32-bit) values. The library includes optimized fast paths for common parameters in rust hot paths.

```rust
use am_midi2::utils::{scale_up, scale_down};

let val_7bit = 0x7F;
let val_32bit = scale_up(val_7bit, 7, 32); // Scales to 0xFFFFFFFF
let back_to_7bit = scale_down(val_32bit, 32, 7); // Scales back to 0x7F
```

## Advanced Usage

### Custom UMP Manipulation

You can manually inspect and modify UMP data using the `Ump` struct methods.

```rust
use am_midi2::ump::{Ump, MessageType};

let mut ump = Ump::new();
ump.set_message_type(MessageType::Midi1ChannelVoice);
ump.set_group(1);
// Manually set data bytes if needed, though Factory methods are preferred.
```

## Support

If you encounter any issues or have questions, please check the repository documentation or submit an issue.

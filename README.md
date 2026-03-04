# am_midi2

A Rust library for MIDI 2.0.

`am_midi2` is a general-purpose, `no_std` compatible library for building MIDI 2.0 Devices and Applications. It aims to work on everything from embedded devices to large-scale applications, providing the building blocks, processing, and translations needed for MIDI 2.0.

This library is a Rust port of an existing C++ MIDI 2.0 library, focusing on safety and ergonomics while maintaining a small footprint. It implements the core primitives, Universal MIDI Packets (UMP), stream parsing, and bit manipulation utilities according to the Phase 1 and Phase 2 roadmap goals.

## State of the Port

Currently, `am_midi2` has completed:
- **Phase 1 (The Foundation)**: UMP containers, core bit-manipulation, and utility primitives.
- **Phase 2 (Protocol Logic)**: Creation of MIDI 1.0 and 2.0 Channel Voice messages, System Common/Realtime messages, Utility messages, and parameter messages (CC, RPN, NRPN, Pitch Bend).

Future phases will address **Phase 3 (MIDI-CI)** and **Phase 4 (Stream & Transport)** to fully implement the specification.

## Features

- **UMP (Universal MIDI Packet) Support**: Full support for all defined UMP message types (32, 64, 96, 128-bit).
- **Message Factory**: Easy-to-use factory methods for creating MIDI 1.0 and MIDI 2.0 Channel Voice messages, System messages, and more.
- **Stream Parsing**: Efficient iterator-based parser for converting streams of `u32` words into valid UMPs.
- **Utilities**: Helper functions for optimized bit scaling (up/down) and constant definitions for MIDI status bytes.
- **`no_std` Compatible**: Designed for embedded use cases with no dependency on the standard library (though `alloc` may be used for specific features in the future).

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
am_midi2 = "0.1.0"
```

### Example: Creating Messages

```rust
use am_midi2::messages::UmpFactory;

fn main() {
    // Create a MIDI 2.0 Note On message
    // Group 0, Channel 0, Note 60, Attribute Type 0, Velocity 0x8000, Attribute Data 0
    let ump = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);
    
    println!("UMP Data: {:08X} {:08X}", ump.data[0], ump.data[1]);
}
```

### Example: Parsing a Stream

```rust
use am_midi2::buffer::UmpStreamParser;

fn main() {
    let raw_data = vec![0x20903C64, 0x40903C00, 0x80000000];
    let parser = UmpStreamParser::new(raw_data.into_iter());

    for ump in parser {
        println!("Received UMP: {:?}", ump);
    }
}
```

## Documentation

- **User Guide**: See [USER_GUIDE.md](USER_GUIDE.md) for detailed usage instructions.
- **API Documentation**: Run `cargo doc --open` to view the generated API documentation.
- **Development Roadmap**: Check out [MIDI2_ROADMAP.md](MIDI2_ROADMAP.md) to see where the project is heading.

## Contributing

Contributions are welcome! If you find bugs, have feature requests, or want to improve documentation, please submit a Pull Request or Issue.

### License

This project is available under the MIT License.

# Developer Guide: Building with and Contributing to `el_midi2`

Welcome to the **`el_midi2` Developer Guide**. Whether you are extending the core protocol engine, adding support for a new microcontroller or audio transport, integrating MIDI 2.0 into an existing C/C++ engine, or contributing bug fixes, this guide covers architecture, design patterns, testing standards, and best practices.

---

## 1. Architectural Philosophy & Core Constraints

`el_midi2` is engineered from the ground up for mission-critical embedded firmware and high-throughput desktop audio software. Every crate adheres to four core design principles:

### 1.1 Strict `#![no_std]` and Zero Dynamic Allocation
- Core crates (`el_core`, `el_midi2`, and all Board Support Packages `el_bsp_*`) run under `#![no_std]`.
- No heap allocation (`alloc`, `Box`, `Vec`, `String`) is used on data paths.
- All buffering utilizes const-generic fixed arrays (e.g., `PacketQueue<const CAP: usize>`, `PeTransactionManager<const MAX_CONCURRENT: usize>`).
- Const-generic ringbuffers statically enforce capacity constraints at compile time (`assert!(CAP > 0)`).

### 1.2 Zero-Copy Slicing and Word-Aligned Packaging
- Universal MIDI Packets (UMP) are 32-bit word-aligned structures (`[u32; 4]`).
- On the wire, UMP is transmitted in Little-Endian byte order within each 32-bit word (Byte 0 is LSB, Byte 3 is MSB).
- APIs pass immutable/mutable slices (`&[u32]` or `&[u8]`) and modify state in place, avoiding buffer copies.

### 1.3 State-Machine Streaming & Reassembly
- Payloads exceeding single-packet boundaries (MIDI 1.0 SysEx7, MIDI 2.0 SysEx8, Mixed Data Sets, Flex Data text, and Property Exchange JSON) are handled via streaming chunkers (`Iterator<Item = Ump>`) and bounded stateful accumulators (`SysEx7Receiver`, `FlexDataReceiver`, `PeDeflateDecoder`).
- Streaming parsers are resilient to truncated streams, interleaving of real-time bytes, and packet sequence anomalies.

### 1.4 Predictable, Branchless Optimization
- Hot paths (such as UMP bit-packing and bit-scaling) utilize branchless arithmetic and bitwise masks (`((center.wrapping_sub(val) as i32) >> 31) as u32`) to prevent CPU branch mispredictions.
- `match` statements on hot paths group mathematically bounded integers using explicit piped values (e.g., `0x3 | 0x4`) to ensure LLVM generates fast jump tables rather than conditional branches.

---

## 2. Workspace Layout & Crate Taxonomy

The workspace is organized into a modular hierarchy:

```text
el_midi2/
├── el_core/              # Zero-alloc foundational primitives: Ump, MessageType, Builders, Scaling
├── src/                  # Main library crate (`el_midi2`): Device coordinator, MIDI-CI, PE, Midiclip
├── el_midi2_c/           # Stable C-ABI static and dynamic library bindings (`libel_midi2.so/.a`)
├── include/              # C and C++20 header files (`el_midi2.h`, legacy C++ headers)
├── el_bridge/            # Bidirectional MIDI 1.0 <-> 2.0 translator with native OS endpoints
├── el_ci_sim/            # MIDI-CI Property Exchange and Process Inquiry simulator CLI
├── el_dump/              # UMP and .midiclip binary stream inspector & hex dumper CLI
├── el_gen/               # Precise synthetic message pattern generator CLI
├── el_mon/               # Real-time hardware jitter analyzer and clock offset profiler CLI
├── el_clip_conv/         # SMF Format 0/1 to MIDI Clip File (.midiclip) converter CLI
└── el_bsp_*/             # Hardware Board Support Packages (ESP32, RP2040, STM32, nRF52, RPi3)
```

### 2.1 Crate Dependency Relationships

```text
         +---------------------------------------+
         |               el_core                 |  (Strict #![no_std], zero alloc)
         +---------------------------------------+
              ^                      ^         ^
              |                      |         +-----------------------+
              |                      |                                 |
         +----+-----------+     +----+------------+              +-----+-----+
         |    el_midi2    |     |   el_midi2_c    |              | el_bsp_*  |
         +----------------+     +-----------------+              +-----------+
           ^     ^     ^
           |     |     |
    +------+     |     +-------+
    |            |             |
+---+-----+  +---+----+   +----+-----+
| el_dump |  | el_gen |   |  el_mon  |
+---------+  +--------+   +----------+
    |            |
+---+-----+  +---+-------+
| el_ci_  |  | el_bridge |
|  sim    |  +-----------+
+---------+
```

---

## 3. Developing and Extending Core Protocols

### 3.1 Adding Message Builders (`el_core::builder` & `src/messages.rs`)
When adding new UMP message types or status codes:
1. Define the status constants in `el_core/src/types.rs`.
2. Implement message constructors in `el_core/src/builder.rs` returning `Ump`.
3. In `src/messages.rs`, expose ergonomic factory methods on `UmpFactory`.
4. Ensure all bit masks are panic-free and properly shifted. Example:
   ```rust
   let combined = ((group as u32) << 24)
       | ((((status & 0xF0) | (channel & 0xF)) as u32) << 16)
       | (((val1 & 0x7F) as u32) << 8)
       | ((val2 & 0x7F) as u32);
   Ump { data: [0x4000_0000 | (combined & 0x0FFF_7F7F), val32, 0, 0] }
   ```

### 3.2 Implementing Bounded Streaming Receivers
When receiving multi-packet data (e.g. SysEx, Flex Data text):
1. Use const-generic bounds: `struct MyReceiver<const CAP: usize>`.
2. Prohibit zero capacity at compile time:
   ```rust
   const { assert!(CAP > 0, "Capacity must be greater than zero") };
   ```
3. Copy payload slices directly into fixed buffers using `copy_from_slice`.
4. Return descriptive errors (`BufferOverflow`, `UnexpectedFormat`, `GroupMismatch`) instead of panicking.

### 3.3 Adding Property Exchange Resources
Standard resources (such as `DeviceInfo`, `ResourceList`, `ChannelList`) format JSON responses dynamically into slice buffers without heap allocation:
1. In `src/resource.rs`, implement a procedural writer function `write_<resource>(buf: &mut [u8], ...) -> Result<usize, PeResourceError>`.
2. Calculate exact string offsets and write key-value pairs using byte slice copying.
3. Validate payload schema against `PeSchemaValidator` in `src/ci_pe.rs`.

---

## 4. Developing Board Support Packages (BSPs)

Each microcontroller family has a dedicated `#![no_std]` BSP crate (`el_bsp_esp32`, `el_bsp_rp2040`, `el_bsp_stm32`, `el_bsp_nrf52`, `el_bsp_rpi3`).

### 4.1 Required Implementations for New BSPs
When adding support for a new MCU architecture:
1. **Serial UART Driver (`*UartDriver`)**:
   - Provide hardware FIFO or DMA circular buffer integration.
   - Calculate exact baud rate dividers for 31,250 baud (MIDI 1.0) and high-speed UMP (1 MBaud, 3 MBaud).
   - Integrate `ByteStreamToUmp` and `UmpToByteStream` state machines to guarantee running status tracking and real-time byte interleaving.
2. **USB-MIDI 2.0 Driver (`*UsbMidiDriver`)**:
   - Format 13-byte standard Group Terminal Block (GTB) descriptors (`bDescriptorSubtype = 0x03`).
   - Implement little-endian 32-bit packet packaging into USB bulk endpoints.
3. **Hardware Clock & Timers**:
   - Map a hardware microsecond counter (1 MHz) for Jitter Reduction timestamps (31,250 Hz / 32 µs units) and AppleMIDI network clock (100 µs units).
4. **Trait Implementations**:
   - Implement `el_midi2::transport::Transport` (`send(&[Ump])`, `recv() -> Option<Ump>`).
   - Implement `el_midi2::transport::EmbeddedTransport` (`read_ump()`, `write_ump()`, `flush()`).

---

## 5. C/C++ FFI Interoperability (`el_midi2_c`)

The `el_midi2_c` crate produces `libel_midi2.so` and `libel_midi2.a` and exposes C-ABI headers in `include/el_midi2.h`.

### 5.1 FFI Safety Guidelines
- Every `pub unsafe extern "C"` function must include an explicit `# Safety` section in its doc comments to satisfy Clippy (`-D warnings`).
- Wrap all internal logic with `std::panic::catch_unwind` to prevent panics from unwinding across the C FFI boundary (which is undefined behavior).
- Check all incoming raw pointers for null before dereferencing:
  ```rust
  /// # Safety
  ///
  /// `out_ump` must point to valid, writable `el_ump_t` memory.
  #[no_mangle]
  pub unsafe extern "C" fn el_ump_create_midi2_note_on(
      group: u8,
      channel: u8,
      note: u8,
      attribute_type: u8,
      velocity: u16,
      attribute_data: u16,
      out_ump: *mut el_ump_t,
  ) -> i32 {
      if out_ump.is_null() {
          return -1;
      }
      let res = catch_unwind(|| {
          let ump = VoiceBuilder::midi2_note_on(group, channel, note, attribute_type, velocity, attribute_data);
          *out_ump = el_ump_t::from(ump);
          0
      });
      res.unwrap_or(-2)
  }
  ```

---

## 6. Contributor Workflow & Verification

Before submitting pull requests, run the following verification sequence:

### 6.1 Building and Testing
```bash
# 1. Run all unit and integration tests across the workspace
cargo test --workspace

# 2. Verify all targets compile cleanly
cargo check --workspace --all-targets

# 3. Build workspace documentation with zero warnings
cargo doc --workspace --no-deps
```

### 6.2 Code Style and Linting
```bash
# 4. Enforce canonical Rust formatting
cargo fmt -- --check

# 5. Run Clippy across all targets with warnings treated as errors
cargo clippy --all-targets --all-features -- -D warnings
```

### 6.3 Security Policy Audit
```bash
# 6. Execute repository security policy audit
python3 ./guardian.py
```

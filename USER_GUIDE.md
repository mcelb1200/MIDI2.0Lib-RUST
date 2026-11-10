# The Tinkerer's Practical Guide to MIDI 2.0 with `el_midi2`

Welcome to `el_midi2`! This guide is crafted specifically for hardware makers, embedded engineers, audio software creators, and synthesizer tinkerers who want to build real, working MIDI 2.0 hardware and software.

---

## Table of Contents

1. [Quick Mental Model: MIDI 1.0 vs. MIDI 2.0](#1-quick-mental-model-midi-10-vs-midi-20)
2. [Hardware Recipes & Physical Wiring](#2-hardware-recipes--physical-wiring)
   - [5-Pin DIN & 3.5mm TRS Optoisolated Input](#5-pin-din--35mm-trs-optoisolated-input)
   - [Hardware UART Baud Rates: The 31,250 vs 1MBaud Reality](#hardware-uart-baud-rates-the-31250-vs-1mbaud-reality)
3. [Tinkerer Project 1: High-Resolution 16-Bit Rotary Knob Controller (RP2040)](#3-tinkerer-project-1-high-resolution-16-bit-rotary-knob-controller-rp2040)
4. [Tinkerer Project 2: DIN MIDI 1.0 to USB-MIDI 2.0 Bridge with JR Timestamps](#4-tinkerer-project-2-din-midi-10-to-usb-midi-20-bridge-with-jr-timestamps)
5. [Tinkerer Project 3: Turnkey MIDI 2.0 Device with MIDI-CI Property Exchange](#5-tinkerer-project-3-turnkey-midi-20-device-with-midi-ci-property-exchange)
6. [Tinkerer Project 4: Playing Multi-Track `.midiclip` Files](#6-tinkerer-project-4-playing-multi-track-midiclip-files)
7. [The CLI Diagnostic Toolkit: How to Debug Your Hardware](#7-the-cli-diagnostic-toolkit-how-to-debug-your-hardware)
   - [Sniffing and Hex Dumping (`el_dump`)](#sniffing-and-hex-dumping-el_dump)
   - [Measuring Microsecond Jitter & Clock Drift (`el_mon`)](#measuring-microsecond-jitter--clock-drift-el_mon)
   - [Generating Synthetic Stress Streams (`el_gen`)](#generating-synthetic-stress-streams-el_gen)
   - [Simulating Property Exchange Handshakes (`el_ci_sim`)](#simulating-property-exchange-handshakes-el_ci_sim)
   - [Bridging Hardware to Virtual DAW Ports (`el_bridge`)](#bridging-hardware-to-virtual-daw-ports-el_bridge)
8. [Gotchas, Edge Cases & FAQ](#8-gotchas-edge-cases--faq)

---

## 1. Quick Mental Model: MIDI 1.0 vs. MIDI 2.0

| Concept | Legacy MIDI 1.0 | Modern MIDI 2.0 (UMP) |
|---|---|---|
| **Data Format** | Variable-length byte stream (1-3 bytes or SysEx) | Universal MIDI Packets (32, 64, 96, or 128-bit words) |
| **Resolution** | 7-bit velocity (0-127), 7-bit CC, 14-bit pitch bend | 16-bit velocity (0-65535), 32-bit CC, 32-bit pitch bend |
| **Channels & Streams**| 16 channels total per cable | 16 Groups $\times$ 16 Channels = 256 virtual channels per wire |
| **Per-Note Control** | Polyphonic key pressure only | Per-Note Pitch Bend, Per-Note CCs, Per-Note RPN/NRPN |
| **Configuration** | Static hardware assumptions | Dynamic MIDI-CI negotiation (Profiles, Property Exchange) |
| **Timing Accuracy** | Unstamped serial intervals, prone to buffer jitter | Jitter Reduction Timestamps (32 µs resolution) & Delta Clockstamps |

---

## 2. Hardware Recipes & Physical Wiring

### 5-Pin DIN & 3.5mm TRS Optoisolated Input

When building hardware synths or eurorack modules, physical opto-isolation prevents ground loops, 60 Hz hum, and damaging overvoltages.

```text
MIDI IN (5-Pin DIN Pin 4 or TRS Tip) ----[ 220 Ω ]----+
                                                       |
                                                    +--+---+
                                                    | 2  VCC | (Pin 8) ---- 3.3V / 5V
                                          1N4148 ---|>|  VO  | (Pin 6) ---- MCU UART RX
                                          Diode     | 3  GND | (Pin 5) ---- MCU GND
                                                    +------+
                                                       | (6N138 / H11L1 Optocoupler)
MIDI IN (5-Pin DIN Pin 5 or TRS Ring) -----------------+

* Note: Add a 4.7 kΩ pull-up resistor from Pin 6 (VO) to VCC.
```

### Hardware UART Baud Rates: The 31,250 vs 1MBaud Reality

1. **Legacy MIDI 1.0 DIN/TRS**: Fixed at **31,250 baud** (1 stop bit, 8 data bits, no parity).
   - In `el_midi2`, feed raw serial bytes into `ByteStreamToUmp` to reconstruct running status and handle interleaved real-time clock bytes without latency.
2. **High-Speed Serial UMP (Serial MIDI 2.0)**:
   - Microcontrollers communicate point-to-point (e.g. between RP2040 and ESP32) using **1,000,000 baud (1 MBaud)** or **3,000,000 baud (3 MBaud)**.
   - At 1 MBaud, each 32-bit UMP word takes only 32 microseconds to transmit, virtually eliminating MIDI latency!

---

## 3. Tinkerer Project 1: High-Resolution 16-Bit Rotary Knob Controller (RP2040)

Building a MIDI 2.0 expression controller using a potentiometer or rotary encoder on an RP2040 (or any ADC-equipped MCU):

```rust
#![no_std]
#![no_main]

use el_core::builder::VoiceBuilder;
use el_core::ump::Ump;

pub struct RotaryController {
    group: u8,
    channel: u8,
    last_value: u32,
}

impl RotaryController {
    pub const fn new(group: u8, channel: u8) -> Self {
        Self {
            group,
            channel,
            last_value: 0,
        }
    }

    /// Read an ADC value (12-bit, 0..4095) and emit a 32-bit MIDI 2.0 Control Change UMP
    pub fn update_adc(&mut self, adc_reading: u16, controller_index: u8) -> Option<Ump> {
        // Upscale 12-bit ADC reading (0..4095) to full 32-bit MIDI 2.0 range (0..0xFFFFFFFF)
        let scaled_32 = el_core::utils::scale_up(adc_reading as u32, 12, 32);

        // Apply basic hysteresis noise filtering (ignore changes < ~0.05%)
        if scaled_32.abs_diff(self.last_value) < 0x0020_0000 {
            return None;
        }

        self.last_value = scaled_32;

        // Build 64-bit MIDI 2.0 Control Change UMP (MT=0x4, Status=0xB)
        Some(VoiceBuilder::midi2_control_change(
            self.group,
            self.channel,
            controller_index,
            scaled_32,
        ))
    }
}
```

---

## 4. Tinkerer Project 2: DIN MIDI 1.0 to USB-MIDI 2.0 Bridge with JR Timestamps

Bridge legacy serial DIN hardware into a modern DAW with jitter-reduction timestamps:

```rust
use el_midi2::messages::{JitterReductionQueue, UmpFactory};
use el_midi2::transport::serial::ByteStreamToUmp;

pub struct DinToUsbBridge<const QUEUE_CAP: usize> {
    byte_to_ump: ByteStreamToUmp,
    jr_queue: JitterReductionQueue<QUEUE_CAP>,
}

impl<const QUEUE_CAP: usize> DinToUsbBridge<QUEUE_CAP> {
    pub fn new(default_group: u8) -> Self {
        Self {
            byte_to_ump: ByteStreamToUmp::new(default_group),
            jr_queue: JitterReductionQueue::new(),
        }
    }

    /// Ingests a raw byte received from UART ISR at the given microsecond timestamp
    pub fn on_uart_byte(&mut self, byte: u8, hw_microsecond_clock: u64) {
        if let Some(ump) = self.byte_to_ump.process_byte(byte) {
            // JR Timestamp resolution is 32 microseconds (31,250 Hz)
            let jr_timestamp_16 = ((hw_microsecond_clock / 32) & 0xFFFF) as u16;

            // Ingest and pair with JR Timestamp
            self.jr_queue
                .push_outbound(&ump, Some(jr_timestamp_16), None)
                .ok();
        }
    }

    /// Read next packet ready to send across USB bulk endpoint
    pub fn poll_usb_out(&mut self) -> Option<el_core::ump::Ump> {
        self.jr_queue.pop().map(|item| item.ump)
    }
}
```

---

## 5. Tinkerer Project 3: Turnkey MIDI 2.0 Device with MIDI-CI Property Exchange

Build a fully compliant synthesizer engine that negotiates protocols and serves JSON parameters:

```rust
use el_midi2::device::Midi2Device;
use el_midi2::transport::MockTransport;

fn main() {
    // 1. Initialize your hardware transport (UART, USB, or Network)
    let transport = MockTransport::<128>::new();

    // 2. Instantiate Midi2Device with a unique 28-bit MUID
    let mut synth = Midi2Device::new(transport, 0x0123456);

    // 3. Configure Function Blocks (e.g. Block 0: Synth Engine on Group 0)
    synth.set_function_block(
        0,
        "Analog PolySynth",
        0, // First Group: 0
        1, // Group Count: 1
        el_midi2::stream::FunctionBlockDirection::Bidirectional,
    );

    // 4. Main Event Loop
    loop {
        // Polling processes inbound UMPs, answers Endpoint Inquiries,
        // handles Stream Configuration protocol switching, and serves
        // standard Property Exchange queries (DeviceInfo, ResourceList)
        synth.poll();

        // Break loop for non-embedded environments
        break;
    }
}
```

---

## 6. Tinkerer Project 4: Playing Multi-Track `.midiclip` Files

Schedule and play multi-track MIDI 2.0 Clip files with high-precision tempo maps:

```rust
use el_midi2::midiclip::{ClipTimeFormat, MultiTrackSequencer};

fn main() {
    // Standard metric resolution (e.g. 480 PPQN)
    let time_format = ClipTimeFormat::TicksPerQuarterNote(480);
    let mut sequencer = MultiTrackSequencer::<8>::new(time_format);

    // Add raw MTrk track bytes (VLQ delta times + UMP packets)
    let track_data = b"\x00\x40\x90\x3C\x00\x80\x00\x00\x00"; // Delta 0, Note On
    sequencer.add_track(track_data).unwrap();

    // In your high-resolution timer interrupt / audio block callback:
    let delta_ticks = 1;
    while let Some(event) = sequencer.tick(delta_ticks) {
        println!("Track {} scheduled UMP: {:08X}", event.track_index, event.ump.data[0]);
    }
}
```

---

## 7. The CLI Diagnostic Toolkit: How to Debug Your Hardware

The `el_midi2` workspace provides 5 standalone CLI tools to analyze, test, and debug your builds:

### Sniffing and Hex Dumping (`el_dump`)
Inspect `.ump` captures and `.midiclip` files with colorized hex output:
```bash
cargo run -p el_dump -- --file session_capture.ump
```

### Measuring Microsecond Jitter & Clock Drift (`el_mon`)
Measure hardware clock drift (24 PPQN MIDI clocks) and jitter reduction timestamps:
```bash
# Monitor raw UMP file or live stream for jitter and BPM drift
cargo run -p el_mon -- --file session_capture.ump --format table
```

### Generating Synthetic Stress Streams (`el_gen`)
Generate precise test streams to stress-test your synth parser:
```bash
# Generate high-resolution pitch bend sweep
cargo run -p el_gen -- pitch-bend --group 0 --channel 0 --value 4294967295
```

### Simulating Property Exchange Handshakes (`el_ci_sim`)
Test your device's MIDI-CI response over terminal:
```bash
cargo run -p el_ci_sim -- pe get --resource "DeviceInfo"
```

### Bridging Hardware to Virtual DAW Ports (`el_bridge`)
Route physical USB or DIN MIDI devices into your DAW via native OS endpoints (ALSA, CoreMIDI, WinMIDI):
```bash
cargo run -p el_bridge -- --native-virtual --native-name "My DIY Synth"
```

---

## 8. Gotchas, Edge Cases & FAQ

### Q1: Why does my DAW ignore my MIDI 2.0 (MT=0x4) notes?
**Answer:** MIDI 2.0 mandates protocol negotiation. Devices start in MIDI 1.0 mode. A DAW will send a **Stream Configuration Request** (`MT=0xF, Status=0x005`). Your device must respond with a **Stream Configuration Notification** (`Status=0x006`) confirming `protocol = 0x02`. `Midi2Device` manages this handshake automatically.

### Q2: How does UMP endianness work on 8-bit or 16-bit MCUs?
**Answer:** UMP is word-based (32 bits). Within each 32-bit word, bytes are transmitted Little-Endian over USB. Always serialize UMP words as:
```rust
let word: u32 = ump.data[0];
let bytes = word.to_le_bytes(); // [b0, b1, b2, b3]
```

### Q3: What happens when two devices share the same MUID?
**Answer:** MIDI-CI uses 28-bit MUIDs (randomly generated, avoiding `0x0000000` and `0x0FFFFFFF`). If an incoming Discovery message has the same MUID as your local device, a collision has occurred. Immediately regenerate your MUID using a hardware entropy source and re-announce with Discovery Inquiry.

### Q4: Can I run this on microcontrollers with tiny RAM (like 16 KB)?
**Answer:** Yes! `el_core` requires **0 bytes of dynamic RAM**. All buffers are const-generically sized. You can configure queues with small capacities (e.g. `PacketQueue<16>`), fitting comfortably within a few hundred bytes of static RAM.

# Utility Functions & Bit-Scaling Architecture

## Overview

MIDI 2.0 introduces high-resolution parameters (16-bit note velocities, 32-bit Control Changes, 32-bit Pitch Bend, and 32-bit RPN/NRPN values) while maintaining backward compatibility with legacy MIDI 1.0 7-bit and 14-bit data.

Converting between these resolutions requires precise mathematical scaling rather than simple zero-padding or bit truncation. Simple bit-shifting introduces asymmetric ranges (e.g. 7-bit `127` shifted left by 25 becomes `0xFE000000`, failing to reach the maximum 32-bit value `0xFFFFFFFF`).

## Bit Replication / Duplication Algorithm

To ensure full dynamic range coverage:
- **Scaling Up (`scale_up`)**: The source bits are shifted to the MSB and duplicated repeatedly down to the LSB.
  - 7-bit `0x7F` (`127`) scales to 32-bit `0xFFFFFFFF` (`4294967295`).
  - 7-bit `0x40` (`64`, center/neutral) scales to `0x80000000` (or `0x81020408` depending on spec midpoint centering rules).
- **Scaling Down (`scale_down`)**: The most significant bits are extracted (`val >> (from_bits - to_bits)`).

## Branchless Optimization

In `el_core::utils` and `src/utils.rs`, bit-scaling hot paths utilize branchless sign-extended masks to eliminate CPU branch mispredictions:
```rust
let mask = ((center_u32.wrapping_sub(val) as i32) >> 31) as u32;
```
This guarantees constant-time execution suitable for hard real-time audio thread processing and interrupt service routines.

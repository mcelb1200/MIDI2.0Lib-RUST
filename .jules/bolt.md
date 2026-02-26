## 2024-05-23 - [Optimized MessageType Lookups]
**Learning:** Replacing `match` statements with `const` array lookups for small enums (like 4-bit Message Types) significantly reduces branching instructions in hot paths.
**Action:** When working with `#[repr(u8)]` enums that cover a small, contiguous range, prefer array lookups over `match` statements for property accessors or conversion from integers.
## 2025-05-23 - [Optimized Bit Scaling]
**Learning:** MIDI 2.0 conversion often involves scaling 7-bit or 14-bit values to 32-bit. A generic loop-based implementation handles arbitrary bit depths but is slow. Replacing it with specialized bitwise operations for common cases (7->32, 14->32) yields a ~2x performance improvement.
**Action:** When implementing generic algorithms, always check for common "hot" input parameters and provide optimized fast paths for them.

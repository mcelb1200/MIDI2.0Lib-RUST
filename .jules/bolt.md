## 2024-05-23 - [Optimized MessageType Lookups]
**Learning:** Replacing `match` statements with `const` array lookups for small enums (like 4-bit Message Types) significantly reduces branching instructions in hot paths.
**Action:** When working with `#[repr(u8)]` enums that cover a small, contiguous range, prefer array lookups over `match` statements for property accessors or conversion from integers.
## 2025-05-23 - [Optimized Bit Scaling]
**Learning:** MIDI 2.0 conversion often involves scaling 7-bit or 14-bit values to 32-bit. A generic loop-based implementation handles arbitrary bit depths but is slow. Replacing it with specialized bitwise operations for common cases (7->32, 14->32) yields a ~2x performance improvement.
**Action:** When implementing generic algorithms, always check for common "hot" input parameters and provide optimized fast paths for them.
## 2025-10-24 - [Optimized Branchless Small Enum Lookups]
**Learning:** For properties of small `#[repr(u8)]` enums with bounded sizes (e.g. 16 values), explicit bounds checking (`if val > 0xF`) can prevent optimal branchless compilation. By using a bitmask (`val & 0xF`) instead, the compiler emits a simple direct memory load without conditional branches. Additionally, storing small constants (like lengths from 1 to 4) as `u8` instead of `usize` in lookup tables shrinks the array footprint (e.g., from 128 to 16 bytes), improving cache efficiency and potentially leading to better memory alignment and instruction selection.
**Action:** When implementing property lookups for small bounded types, favor bitmasking over logical bounds checking to guarantee branchless array lookups. Also, pack static lookup tables tightly (e.g., `[u8; N]`) to save data segment space and cache lines.

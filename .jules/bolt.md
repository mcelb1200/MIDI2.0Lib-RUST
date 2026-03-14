## 2024-05-23 - [Optimized MessageType Lookups]
**Learning:** Replacing `match` statements with `const` array lookups for small enums (like 4-bit Message Types) significantly reduces branching instructions in hot paths.
**Action:** When working with `#[repr(u8)]` enums that cover a small, contiguous range, prefer array lookups over `match` statements for property accessors or conversion from integers.
## 2025-05-23 - [Optimized Bit Scaling]
**Learning:** MIDI 2.0 conversion often involves scaling 7-bit or 14-bit values to 32-bit. A generic loop-based implementation handles arbitrary bit depths but is slow. Replacing it with specialized bitwise operations for common cases (7->32, 14->32) yields a ~2x performance improvement.
**Action:** When implementing generic algorithms, always check for common "hot" input parameters and provide optimized fast paths for them.
## 2025-10-24 - [Optimized Branchless Small Enum Lookups]
**Learning:** For properties of small `#[repr(u8)]` enums with bounded sizes (e.g. 16 values), explicit bounds checking (`if val > 0xF`) can prevent optimal branchless compilation. By using a bitmask (`val & 0xF`) instead, the compiler emits a simple direct memory load without conditional branches. Additionally, storing small constants (like lengths from 1 to 4) as `u8` instead of `usize` in lookup tables shrinks the array footprint (e.g., from 128 to 16 bytes), improving cache efficiency and potentially leading to better memory alignment and instruction selection.
**Action:** When implementing property lookups for small bounded types, favor bitmasking over logical bounds checking to guarantee branchless array lookups. Also, pack static lookup tables tightly (e.g., `[u8; N]`) to save data segment space and cache lines.
## 2025-10-24 - [Optimized String Accumulation for PE Headers]
**Learning:** During MIDI-CI parsing of Property Exchange (PE) headers, characters are appended one by one over multiple bytes into a `std::string` mapped by request index (`peHeaderStr`). Without pre-allocating memory, this triggers repeated memory allocations under the hood, negatively impacting performance especially for larger SysEx headers near the 1024-byte limit. Additionally, the original code checked for `midici.numChunk == 1` at the start of header payload processing (byte 16), but `midici.numChunk` only gets updated after headers are fully consumed, meaning `numChunk` is 0 during the entire header parsing of the first chunk.
**Action:** When accumulating data chunks of known length into a standard library string or vector, always extract the length field early and call `.reserve(length)` to allocate memory once, preventing expensive reallocations in the parsing loop. Ensure state variables like chunk counters are evaluated at their true value during the lifecycle of the parsing state machine.

## 2024-05-24 - [Avoid Duplicated Conditional Checks in Hot Paths]
**Learning:** The `scale_up` utility function had two separate `if dst_bits == 32` blocks with duplicated logic checking `src_bits`. This redundancy resulted in unnecessary branching, variable assignment overhead, and an overall slower execution (e.g., ~57ms for 10M iterations). Merging these into a single fast path using concise bitwise operators reduced the instruction count and yielded an ~8x performance improvement (down to ~7ms for 10M iterations).
**Action:** When creating explicit fast paths for common parameters, consolidate the condition checks into single branch blocks and simplify intermediate variables into direct bitwise return statements to assist compiler optimization in hot paths.

## 2024-05-24 - Unroll UmpStreamParser Iterator
**Learning:** In hot parser loops reading from an iterator (like `UmpStreamParser::next`), using a `for` loop to conditionally populate an array introduces branching and mutable state overhead that limits compiler optimizations.
**Action:** Replace small bounded `for` loops with explicitly unrolled `match` statements and direct left-to-right array initialization (`[w1, iter.next()?, ...]`). This maintains correct evaluation order while saving ~10% execution time by removing loop counters and branch mispredictions.

## 2025-10-24 - [Avoid Naive Bit Scaling Assumptions]
**Learning:** When writing explicit fast paths for MIDI 2.0 bit scaling (e.g., 8-bit to 32-bit), the algorithm is not a standard repeating pattern (like `(val << 24) | (val << 16)...`). The specification dictates that values less than or equal to the source center (e.g., 128 for 8-bit) are merely left-shifted, and only values above the center fill the lower bits with a repeated pattern of their remaining value. Using a generic standard formula introduces severe logic errors that break existing tests.
**Action:** When unrolling the `scale_up` loop for a specific bit depth, always write a companion benchmarking script (`verify.rs`) to test every possible input for that bit depth against the original generic loop (`scale_up_reference`) to guarantee 100% correctness before committing.
## 2025-10-24 - [Avoid Duplicated Bounds Checking in scale_down]
**Learning:** The `scale_down` function previously performed bounds checking twice: first validating `src_bits > 32 || dst_bits > 32` and then calculating `scale_bits` to check if `scale_bits >= 32`. By calculating `scale_bits` early using `saturating_sub` and merging all limits into a single `if` statement, we reduce unnecessary variable assignment jumps and duplicate branching in hot paths. This optimized simple right shift scales much faster.
**Action:** When implementing mathematical utilities or bit shifts, attempt to compute any preliminary saturating operations first so that out-of-bounds error handling can be consolidated into a single early-return condition block.

## 2023-10-27 - [C++ scaleUp Optimization]
**Learning:** Explicitly unrolling generic bit-shifting loops for common MIDI conversion paths (7-bit, 8-bit, 14-bit to 32-bit) in C++ (`M2Utils::scaleUp`) can significantly speed up the hot path (approx. 3x improvement). This matches the optimization strategy used in the Rust implementation.
**Action:** When working on similar conversion functions, consider replacing generalized loop logic with direct bitwise operations for common, known sizes.

## 2025-05-23 - [Optimized MessageType Extraction]
**Learning:** `Ump::message_type()` uses a large match statement that can be optimized into a direct array lookup using the bitmasked value. Because the types map exactly to 4-bits (`0x0` to `0xF`), masking the shifted value allows us to safely index into a constant array of `MessageType` instances. This replaces branching instructions with a direct data load, yielding measurable performance improvements.
**Action:** When extracting bounded enum types from a raw word in hot paths (like `Ump::message_type`), prefer a bitmasked direct lookup table over a large `match` statement.
## 2025-10-24 - [Optimized UmpStreamParser Branching and Lookup]
**Learning:** In highly iterated `next()` methods mapping an integer type directly to expected read lengths (like `UmpStreamParser` consuming chunks from a stream), looking up the chunk count in a static `const` array first, and then using a secondary `match` on the resulting count adds extra memory lookup overhead and instructions.
**Action:** Use a direct `match` statement grouped logically by bitmasked values (e.g. `match (w1 >> 28) & 0xF { 0x0 | 0x1 => ... }`). The compiler can optimize this unified block more effectively than a separate array load and subsequent state match, reducing operations and yielding up to 20% faster execution in tight parsing loops.

## 2024-05-24 - [Avoid Double Memory Lookups for Enum Properties]
**Learning:** In hot paths, calling an enum property method that relies on a static array lookup (like `self.message_type().word_count()`) can result in a double memory lookup: one to convert the raw bit value to the enum via a static array, and a second to get the property via another static array.
**Action:** When a property maps directly from a raw bit value (e.g., extracting word count directly from the Message Type nibble), implement a direct array lookup from the raw value to bypass the intermediate enum conversion overhead.

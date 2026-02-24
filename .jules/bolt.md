## 2024-05-23 - [Optimized MessageType Lookups]
**Learning:** Replacing `match` statements with `const` array lookups for small enums (like 4-bit Message Types) significantly reduces branching instructions in hot paths.
**Action:** When working with `#[repr(u8)]` enums that cover a small, contiguous range, prefer array lookups over `match` statements for property accessors or conversion from integers.

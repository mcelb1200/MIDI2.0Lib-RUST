## 2024-05-18 - Trusting SysEx length fields for fixed internal buffers
**Vulnerability:** Out-of-bounds Read
**Learning:** In C++ specifically, passing a pointer to an internal buffer, and an unsanitized length variable into a user callback provides an opportunity for malicious devices to dictate the buffer length being read by the client application. In this case, `intTemp[1]` which dictated the length could exceed the internal bounds of `buffer` (which is size 256), leading an out-of-bounds read vulnerability.
**Prevention:** Bound `intTemp[X]` parameters to the length of the actual array provided to the user callback (`buffer`) since we already bounds check inserting data into `buffer`.

## 2024-03-02 - Unsafe sprintf buffer overflow risk
**Vulnerability:** The `hirezRepresentation` function in `include/utils.h` used `sprintf` to format high-resolution output strings. Since `sprintf` does not know the size of the target buffer, this could potentially lead to a buffer overflow if the generated string exceeds the buffer size.
**Learning:** Legacy C++ code often uses unsafe string functions like `sprintf`, `strcpy`, and `strcat` which are prone to buffer overflows. These should always be replaced with safer, bounds-checked equivalents.
**Prevention:** Replace `sprintf` with `snprintf` by modifying the function signature to accept a buffer length parameter (`size_t outputLen`). Always pass the correct buffer size to `snprintf` to ensure the buffer is never overrun.

## 2026-03-05 - Buffer overwrite by overlapping variables
**Vulnerability:** In C++, fixed-size arrays were used to store both fixed header components and variable-length payload components, but overlapping index limits allowed payload data to overwrite header data, leading to a buffer overwrite.
**Learning:** Even if the bounds of the array as a whole are checked to prevent out-of-bounds writes, the logic inside a single array may overwrite other variables stored in the array if the indices used for variable-length payload storage are not correctly offset from the header field indices.
**Prevention:** Explicitly subtract the header storage size from the maximum allowed payload write length when bounds-checking, and explicitly offset the write indices by the header storage size.

## 2024-05-24 - Edge cases omitted by early length byte accumulation returns
**Vulnerability:** Out-of-bounds Read / Data integrity
**Learning:** When writing loops that parse stateful, streaming buffers (like SysEx processing where lengths span multiple bytes) returning early after processing intermediate length variables skips over the end-of-packet/completion conditions. If the payload length evaluates to 0, returning early forces the parser to expect more data that will never arrive, causing callbacks to fire late, out-of-bounds, or never at all.
**Prevention:** Do not early `return;` when accumulating length metadata (e.g., `intTemp`). Process the byte and allow the completion check at the end of the block to execute in the same iteration, adjusting the check if necessary to account for `length == 0` configurations.
## 2024-05-30 - Information Leak via Integer Underflow in 0-length payload handling
**Vulnerability:** Information Leak
**Learning:** In C++ processing loops (like `src/midiCIProcessor.cpp`), calculating array offsets by subtracting an initial index from the current position (`sysexPos - initPos`) without checking if the payload length is zero can cause the calculated offset modulo the buffer length to result in a positive, non-zero value. Passing this offset to a user callback as a payload length (while passing a pointer to the internal buffer) exposes stale or uninitialized memory to the application, resulting in an information leak.
**Prevention:** Always explicitly check for zero-length payload edge cases (`dataLength == 0` or `bodyLength == 0`) before using offset calculations as length parameters for callbacks, and explicitly pass `0` to prevent leaking internal buffer state.
## 2024-05-22 - [Fix silent skips for zero-length SysEx payloads]
**Vulnerability:** The MIDI-CI SysEx parsing logic contained early `return;` statements in blocks that accumulate length fields for Property Exchange (PE) requests and Profile Specific Data. This caused the parser to skip the end-of-payload completion checks if the payload length was exactly zero, silently ignoring valid zero-length payloads and potentially causing the system to hang waiting for an expected callback.
**Learning:** In streaming parsers where the end of a message is detected by matching a running index against a calculated length, an early `return;` during length accumulation can skip the same-iteration check for zero-length payloads.
**Prevention:** Avoid early `return;` statements in parser state machines when reading length fields; instead, let execution fall through to the payload length checks to correctly process zero-length payloads on the same iteration.

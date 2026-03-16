## 2024-03-04 - [Defense in Depth: Zeroing Internal Buffers]
**Vulnerability:** The internal parsing `buffer[256]` in `midiCIProcessor.cpp` was only partially cleared (`buffer[0] = '\0'`) during `startSysex7()`. Previously parsed payload bytes from sensitive Property Exchange requests, Endpoint Info, Profile details, etc., could persist in memory across parsing sessions.
**Learning:** Even though index bounds checking prevented direct overflow exploits, leftover buffer data poses a secondary risk of memory disclosure or logic errors if uninitialized parts of the buffer are later copied or reused. Security is defense in depth.
**Prevention:** Explicitly `memset` fixed-size internal buffers to zero at the start and end of processing lifecycles (e.g., `startSysex7` and `endSysex7`).

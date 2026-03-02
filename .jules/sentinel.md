## 2024-05-18 - Trusting SysEx length fields for fixed internal buffers
**Vulnerability:** Out-of-bounds Read
**Learning:** In C++ specifically, passing a pointer to an internal buffer, and an unsanitized length variable into a user callback provides an opportunity for malicious devices to dictate the buffer length being read by the client application. In this case, `intTemp[1]` which dictated the length could exceed the internal bounds of `buffer` (which is size 256), leading an out-of-bounds read vulnerability.
**Prevention:** Bound `intTemp[X]` parameters to the length of the actual array provided to the user callback (`buffer`) since we already bounds check inserting data into `buffer`.## 2024-03-02 - Unsafe sprintf buffer overflow risk

**Vulnerability:** The `hirezRepresentation` function in `include/utils.h` used `sprintf` to format high-resolution output strings. Since `sprintf` does not know the size of the target buffer, this could potentially lead to a buffer overflow if the generated string exceeds the buffer size.
**Learning:** Legacy C++ code often uses unsafe string functions like `sprintf`, `strcpy`, and `strcat` which are prone to buffer overflows. These should always be replaced with safer, bounds-checked equivalents.
**Prevention:** Replace `sprintf` with `snprintf` by modifying the function signature to accept a buffer length parameter (`size_t outputLen`). Always pass the correct buffer size to `snprintf` to ensure the buffer is never overrun.

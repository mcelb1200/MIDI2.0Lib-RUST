#include "include/utils.h"
#include <chrono>
#include <iostream>
#include <vector>
#include <cstring>

void benchmark(size_t n, int iterations) {
    std::vector<uint8_t> buffer(n);
    uint8_t* ptr = buffer.data();
    auto start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < iterations; ++i) {
        M2Utils::clear(ptr, (uint8_t)(i & 0xFF), n);
    }
    auto end = std::chrono::high_resolution_clock::now();
    std::chrono::duration<double> diff = end - start;
    std::cout << "Buffer size: " << n << ", Iterations: " << iterations << ", Time: " << diff.count() << "s" << std::endl;
}

int main() {
    benchmark(64, 100000000);
    benchmark(256, 100000000);
    benchmark(1024, 10000000);
    benchmark(1024 * 1024, 10000);
    return 0;
}

#!/bin/bash
cppcheck --enable=all --inconclusive --std=c++17 -Iinclude src/midiCIProcessor.cpp src/umpProcessor.cpp

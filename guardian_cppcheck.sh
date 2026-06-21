#!/bin/bash
cppcheck --enable=all --inconclusive --std=c++17 -Iinclude src/midiCIProcessor.cpp src/umpProcessor.cpp --suppress=uninitMemberVar --suppress=unusedFunction --suppress=unusedPrivateFunction --suppress=knownConditionTrueFalse --error-exitcode=1

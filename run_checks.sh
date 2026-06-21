#!/bin/bash
cppcheck --enable=warning,style,performance,portability --inconclusive --std=c++17 -Iinclude src/midiCIProcessor.cpp src/umpProcessor.cpp --suppress=uninitMemberVar --suppress=unusedFunction --suppress=unusedPrivateFunction --suppress=knownConditionTrueFalse > cppcheck_output.txt 2>&1
cat cppcheck_output.txt

#include "el_midi2.h"
#include <cassert>
#include <cstring>
#include <iostream>

int main() {
  std::cout << "Running C/C++ FFI C-ABI integration tests...\n";

  // 1. Factory functions tests
  {
    // MIDI 1.0 Note On: group 1, channel 2, note 60 (0x3C), velocity 100 (0x64)
    // Word 0: 0x20000000 | (1 << 24) | (0x90 << 16) | (2 << 16) ... ->
    // 0x21923C64
    el_ump_t m1_note = el_ump_create_midi1_note_on(1, 2, 60, 100);
    assert(m1_note.data[0] == 0x21923C64);
    assert(m1_note.data[1] == 0);
    assert(m1_note.data[2] == 0);
    assert(m1_note.data[3] == 0);

    // MIDI 2.0 Note On: group 0, channel 5, note 72 (0x48), attr 0, vel 0xC000,
    // attr_data 0
    el_ump_t m2_note = el_ump_create_midi2_note_on(0, 5, 72, 0, 0xC000, 0);
    assert(m2_note.data[0] == 0x40954800);
    assert(m2_note.data[1] == 0xC0000000);
    assert(m2_note.data[2] == 0);
    assert(m2_note.data[3] == 0);

    // MIDI 2.0 CC: group 2, channel 3, index 7, value 0x12345678
    el_ump_t m2_cc = el_ump_create_midi2_cc(2, 3, 7, 0x12345678);
    assert(m2_cc.data[0] == 0x42B30700);
    assert(m2_cc.data[1] == 0x12345678);

    // MIDI 2.0 Pitch Bend: group 3, channel 4, value 0x80000000
    el_ump_t pb = el_ump_create_pitch_bend(3, 4, 0x80000000);
    assert(pb.data[0] == 0x43E40000);
    assert(pb.data[1] == 0x80000000);

    // MIDI 2.0 RPN: group 1, channel 2, bank 0x12, index 0x34, value 0x55667788
    el_ump_t rpn = el_ump_create_midi2_rpn(1, 2, 0x12, 0x34, 0x55667788);
    assert(rpn.data[0] == 0x41221234);
    assert(rpn.data[1] == 0x55667788);

    // MIDI 2.0 NRPN: group 2, channel 3, bank 0x0A, index 0x0B, value
    // 0x11223344
    el_ump_t nrpn = el_ump_create_midi2_nrpn(2, 3, 0x0A, 0x0B, 0x11223344);
    assert(nrpn.data[0] == 0x42330A0B);
    assert(nrpn.data[1] == 0x11223344);

    // MIDI 2.0 Relative RPN: group 0, channel 1, bank 0x01, index 0x02, value
    // -100
    el_ump_t rel_rpn = el_ump_create_midi2_relative_rpn(0, 1, 0x01, 0x02, -100);
    assert(rel_rpn.data[0] == 0x40410102);
    assert(rel_rpn.data[1] == static_cast<uint32_t>(-100));

    // MIDI 2.0 Relative NRPN: group 3, channel 4, bank 0x05, index 0x06, value
    // 250
    el_ump_t rel_nrpn =
        el_ump_create_midi2_relative_nrpn(3, 4, 0x05, 0x06, 250);
    assert(rel_nrpn.data[0] == 0x43540506);
    assert(rel_nrpn.data[1] == 250);
  }

  // 2. Scaling utilities tests
  {
    uint32_t scaled_up_32 = el_scale_up(64, 7, 32);
    assert(scaled_up_32 == 0x80000000);

    uint32_t scaled_down_7 = el_scale_down(0x80000000, 32, 7);
    assert(scaled_down_7 == 64);

    uint32_t scaled_up_16 = el_scale_up(127, 7, 16);
    assert(scaled_up_16 == 0xFFFF);

    uint32_t scaled_down_from_16 = el_scale_down(0xFFFF, 16, 7);
    assert(scaled_down_from_16 == 127);
  }

  // 3. Stream Parser tests
  {
    el_stream_parser_t *parser = el_stream_parser_create();
    assert(parser != nullptr);

    el_ump_t out;
    std::memset(&out, 0, sizeof(out));

    // 1-word packet: MT=0x2
    bool ready = el_stream_parser_feed_word(parser, 0x20903C64, &out);
    assert(ready);
    assert(out.data[0] == 0x20903C64);
    assert(out.data[1] == 0);

    // 2-word packet: MT=0x4
    ready = el_stream_parser_feed_word(parser, 0x40B00700, &out);
    assert(!ready);
    ready = el_stream_parser_feed_word(parser, 0xAABBCCDD, &out);
    assert(ready);
    assert(out.data[0] == 0x40B00700);
    assert(out.data[1] == 0xAABBCCDD);

    // 4-word packet: MT=0xF
    assert(!el_stream_parser_feed_word(parser, 0xF0000000, &out));
    assert(!el_stream_parser_feed_word(parser, 0x11111111, &out));
    assert(!el_stream_parser_feed_word(parser, 0x22222222, &out));
    assert(el_stream_parser_feed_word(parser, 0x33333333, &out));
    assert(out.data[0] == 0xF0000000);
    assert(out.data[1] == 0x11111111);
    assert(out.data[2] == 0x22222222);
    assert(out.data[3] == 0x33333333);

    // NULL safety checks
    assert(!el_stream_parser_feed_word(nullptr, 0x1234, &out));
    assert(!el_stream_parser_feed_word(parser, 0x1234, nullptr));

    // Clean up parser
    el_stream_parser_destroy(parser);
    el_stream_parser_destroy(nullptr); // Safe no-op
  }

  std::cout << "All C/C++ FFI C-ABI integration tests passed successfully!\n";
  return 0;
}

#ifndef EL_MIDI2_H
#define EL_MIDI2_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  uint32_t data[4];
} el_ump_t;

// Factory functions
el_ump_t el_ump_create_midi1_note_on(uint8_t group, uint8_t channel,
                                     uint8_t note, uint8_t velocity);
el_ump_t el_ump_create_midi2_note_on(uint8_t group, uint8_t channel,
                                     uint8_t note, uint8_t attribute_type,
                                     uint16_t velocity,
                                     uint16_t attribute_data);
el_ump_t el_ump_create_midi2_cc(uint8_t group, uint8_t channel, uint8_t index,
                                uint32_t value);
el_ump_t el_ump_create_pitch_bend(uint8_t group, uint8_t channel,
                                  uint32_t value);

// Stream parser handle
typedef struct el_stream_parser_t el_stream_parser_t;
el_stream_parser_t *el_stream_parser_create(void);
bool el_stream_parser_feed_word(el_stream_parser_t *parser, uint32_t word,
                                el_ump_t *out_ump);
void el_stream_parser_destroy(el_stream_parser_t *parser);

// Scaling utilities
uint32_t el_scale_up(uint32_t val, uint8_t src_bits, uint8_t dst_bits);
uint32_t el_scale_down(uint32_t val, uint8_t src_bits, uint8_t dst_bits);

#ifdef __cplusplus
}
#endif

#endif // EL_MIDI2_H

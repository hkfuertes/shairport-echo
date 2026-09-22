#ifndef ECHO_ALSA_H
#define ECHO_ALSA_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct echo_alsa_handle echo_alsa_t;

typedef struct {
  uint32_t sample_rate;
  uint32_t channels;
  uint32_t period_frames;
  uint32_t periods;
} echo_alsa_config_t;

typedef struct {
  uint64_t raw_measurement_time_ns;
  uint64_t corrected_measurement_time_ns;
  uint64_t frames_sent_to_dac;
  int32_t delay_frames;
  uint32_t reserved;
} echo_alsa_stats_t;

/* All failures are negative errno values. A handle is owned by its caller. */
int echo_alsa_open(echo_alsa_t **out);
int echo_alsa_close(echo_alsa_t *handle);
int echo_alsa_get_config(echo_alsa_t *handle, echo_alsa_config_t *out);
int echo_alsa_start(echo_alsa_t *handle);
int echo_alsa_stop(echo_alsa_t *handle);
int echo_alsa_flush(echo_alsa_t *handle);
int echo_alsa_write_s16le(echo_alsa_t *handle, const int16_t *samples,
                          uint32_t frames, uint32_t *frames_written);
int echo_alsa_delay_frames(echo_alsa_t *handle, int32_t *out);
/* A negative return may still provide stats after an output discontinuity. */
int echo_alsa_stats(echo_alsa_t *handle, echo_alsa_stats_t *out);
int echo_alsa_set_volume_db(echo_alsa_t *handle, double volume_db);
int echo_alsa_set_mute(echo_alsa_t *handle, int muted);

#ifdef __cplusplus
}
#endif

#endif

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
/* Must be called before start. With external controls, this handle only owns PCM and
 * route/XRUN recovery; another process owns volume, mute and speaker amplification. */
int echo_alsa_set_external_controls(echo_alsa_t *handle, int enabled);
int echo_alsa_start(echo_alsa_t *handle);
int echo_alsa_stop(echo_alsa_t *handle);
int echo_alsa_flush(echo_alsa_t *handle);
int echo_alsa_write_s16le(echo_alsa_t *handle, const int16_t *samples,
                          uint32_t frames, uint32_t *frames_written);
int echo_alsa_delay_frames(echo_alsa_t *handle, int32_t *out);
/* A negative return may still provide stats after an output discontinuity. */
int echo_alsa_stats(echo_alsa_t *handle, echo_alsa_stats_t *out);
int echo_alsa_set_volume_db(echo_alsa_t *handle, double volume_db);
/* Reads the current system mixer level; muted hardware reports -144 dB. */
int echo_alsa_get_volume_db(echo_alsa_t *handle, double *out);
/* Reads only the system mixer; it does not open PCM or change routing/amplification. */
int echo_alsa_read_system_volume_db(double *out);
/* Sets only the system mixer, including fractional dB, and returns actual readback.
 * No PCM, routing or amplifier changes. Rejects NULL out and non-finite volume. */
int echo_alsa_set_system_volume_db(double volume_db, double *out);
/* Adjusts this handle's mixer in 1 dB steps within -30..0 dB; the bottom step mutes. */
int echo_alsa_adjust_volume_db(echo_alsa_t *handle, int steps, double *out);
/* Adjusts only the system mixer in 1 dB steps; it does not open PCM or change routing/amplification. */
int echo_alsa_adjust_system_volume_db(int steps, double *out);
int echo_alsa_set_mute(echo_alsa_t *handle, int muted);

#ifdef __cplusplus
}
#endif

#endif

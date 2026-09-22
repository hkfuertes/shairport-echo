#ifndef ECHO_CONTROLS_H
#define ECHO_CONTROLS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define ECHO_CONTROLS_RING_SEGMENTS 12u
#define ECHO_CONTROLS_RING_CHANNELS (ECHO_CONTROLS_RING_SEGMENTS * 3u)

typedef struct echo_controls_ring echo_controls_ring_t;
typedef struct echo_controls_buttons echo_controls_buttons_t;

enum echo_controls_button {
  ECHO_CONTROLS_BUTTON_MUTE = 1,
  ECHO_CONTROLS_BUTTON_VOLUME_DOWN = 2,
  ECHO_CONTROLS_BUTTON_VOLUME_UP = 3,
  ECHO_CONTROLS_BUTTON_ACTION = 4,
};

enum echo_controls_button_state {
  ECHO_CONTROLS_BUTTON_PRESSED = 1,
  ECHO_CONTROLS_BUTTON_RELEASED = 2,
  ECHO_CONTROLS_BUTTON_REPEAT = 3,
};

typedef struct {
  uint32_t button;
  uint32_t state;
} echo_controls_button_event_t;

/*
 * All failures are negative errno values. Ring open and close never write LEDs.
 * rgb is 12 consecutive red/green/blue triplets, one for each ring segment.
 */
int echo_controls_ring_open_default(echo_controls_ring_t **out);
int echo_controls_ring_open(const char *path, echo_controls_ring_t **out);
int echo_controls_ring_close(echo_controls_ring_t *handle);
int echo_controls_ring_set_frame(echo_controls_ring_t *handle, const uint8_t *rgb,
                                 size_t bytes);
int echo_controls_ring_set_all(echo_controls_ring_t *handle, uint8_t red,
                               uint8_t green, uint8_t blue);
int echo_controls_ring_off(echo_controls_ring_t *handle);
int echo_controls_ring_set_current(echo_controls_ring_t *handle, uint8_t current);
int echo_controls_ring_set_boot_animation(echo_controls_ring_t *handle, int enabled);

/*
 * nonblocking != 0 makes buttons_next return -EAGAIN when no complete event is ready.
 * Otherwise buttons_next blocks. Do not close a handle concurrently with buttons_next.
 */
int echo_controls_buttons_open(const char *path, int nonblocking,
                               echo_controls_buttons_t **out);
int echo_controls_buttons_close(echo_controls_buttons_t *handle);
int echo_controls_buttons_next(echo_controls_buttons_t *handle,
                               echo_controls_button_event_t *out);

#ifdef __cplusplus
}
#endif

#endif

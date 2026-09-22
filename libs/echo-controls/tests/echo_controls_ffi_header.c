#include "../include/echo_controls.h"

#include <stddef.h>

_Static_assert(ECHO_CONTROLS_RING_SEGMENTS == 12u, "ring segment count");
_Static_assert(ECHO_CONTROLS_RING_CHANNELS == 36u, "ring channel count");
_Static_assert(sizeof(echo_controls_button_event_t) == 8u, "button event layout");
_Static_assert(offsetof(echo_controls_button_event_t, state) == 4u, "button state offset");

static void check_signatures(void) {
  int (*ring_open)(const char *, echo_controls_ring_t **) = echo_controls_ring_open;
  int (*ring_frame)(echo_controls_ring_t *, const uint8_t *, size_t) =
      echo_controls_ring_set_frame;
  int (*button_open)(const char *, int, echo_controls_buttons_t **) =
      echo_controls_buttons_open;
  int (*button_next)(echo_controls_buttons_t *, echo_controls_button_event_t *) =
      echo_controls_buttons_next;
  (void)ring_open;
  (void)ring_frame;
  (void)button_open;
  (void)button_next;
}

int main(void) {
  check_signatures();
  return 0;
}

#include <stddef.h>

#include "echo_alsa.h"

_Static_assert(sizeof(echo_alsa_config_t) == 16, "echo_alsa_config_t");
_Static_assert(sizeof(echo_alsa_stats_t) == 32, "echo_alsa_stats_t");
_Static_assert(offsetof(echo_alsa_stats_t, delay_frames) == 24, "delay_frames offset");

static int (*const open_fn)(echo_alsa_t **) = echo_alsa_open;
static int (*const write_fn)(echo_alsa_t *, const int16_t *, uint32_t, uint32_t *) =
    echo_alsa_write_s16le;
static int (*const stats_fn)(echo_alsa_t *, echo_alsa_stats_t *) = echo_alsa_stats;

int main(void) {
  return open_fn == 0 || write_fn == 0 || stats_fn == 0;
}

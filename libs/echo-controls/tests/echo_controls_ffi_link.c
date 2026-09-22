#include "echo_controls.h"

int main(void) {
  echo_controls_ring_t *ring = 0;
  return echo_controls_ring_open_default(&ring);
}

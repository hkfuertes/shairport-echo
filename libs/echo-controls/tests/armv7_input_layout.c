#include <linux/input.h>
#include <stddef.h>

_Static_assert(sizeof(struct input_event) == 16, "ARMv7 input_event size");
_Static_assert(offsetof(struct input_event, type) == 8, "input_event type offset");
_Static_assert(offsetof(struct input_event, code) == 10, "input_event code offset");
_Static_assert(offsetof(struct input_event, value) == 12, "input_event value offset");

_Static_assert(EV_KEY == 0x01, "EV_KEY value");
_Static_assert(KEY_MUTE == 113, "mute key code");
_Static_assert(KEY_VOLUMEDOWN == 114, "volume down key code");
_Static_assert(KEY_VOLUMEUP == 115, "volume up key code");
_Static_assert(KEY_HELP == 138, "action key code");

int main(void) { return 0; }

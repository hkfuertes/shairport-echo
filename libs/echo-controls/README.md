# echo-controls

LED-ring and physical-input support for ARMv7 Echo Dot Minimal Base devices running Android 7.1/API 25.

## Provides

- 12-segment RGB LED ring frames through the Echo sysfs endpoint.
- Frame deduplication and no-write-on-open behavior.
- Evdev button reading with runtime-correct `input_event` layout.
- `controls-preflight`, `led_probe`, and the ARMv7 input-layout C probe.

## Receiver integration

This crate stays separate from Shairport Sync; it is not linked into `audio_echo`. The packaged `echo-volume-control` service links it directly for ring and input access.

It also builds `libecho_controls.a` and ships `include/echo_controls.h`. The C ABI uses caller-owned opaque handles, returns negative errno values, writes no LED state on ring open/close, and supports blocking or `-EAGAIN` nonblocking button reads. It deliberately supplies hardware access only: a companion service owns policy and must not create a second volume state.

The shipped service uses `--no-volume-buttons`; local `+/-` are deliberately inactive until device-to-sender synchronization has a supported AP2 control path.

## Target safety

`controls-preflight` is non-writing. `led_probe` writes visible LED frames, and button readers open input devices. Run target binaries only with explicit hardware-test approval.

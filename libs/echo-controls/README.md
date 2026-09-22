# echo-controls

LED-ring and physical-input support for ARMv7 Echo Dot Minimal Base devices running Android 7.1/API 25.

## Provides

- 12-segment RGB LED ring frames through the Echo sysfs endpoint.
- Frame deduplication and no-write-on-open behavior.
- Evdev button reading with runtime-correct `input_event` layout.
- `controls-preflight`, `led_probe`, and the ARMv7 input-layout C probe.

## Shairport port status

This crate stays separate from Shairport Sync, but the `audio_echo` controls bridge links its static archive.

It also builds `libecho_controls.a` and ships `include/echo_controls.h`. The C ABI uses caller-owned opaque handles, returns negative errno values, writes no LED state on ring open/close, and supports blocking or `-EAGAIN` nonblocking button reads. It deliberately supplies hardware access only: a companion C program owns button/LED policy and must not create a second volume state.

The bridge owns button/LED policy and must keep volume ownership with the selected system output path rather than creating a second gain state.

## Target safety

`controls-preflight` is non-writing. `led_probe` writes visible LED frames, and button readers open input devices. Run target binaries only with explicit hardware-test approval.

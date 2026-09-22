# echo-alsa

Raw ALSA output and speaker-route support for ARMv7 Echo Dot Minimal Base devices running Android 7.1/API 25.

## Provides

- PCM card 0, device 23 at 48 kHz, stereo, S16_LE.
- ARMv7/Bionic ALSA ioctl layouts and compile-time C ABI probe.
- Echo mixer lifecycle: PCM master volume, amplifier, GPIO mute, and `Right Channel Only` handling.
- `preflight` and `pcm_probe` diagnostics.
- The existing Rust `EchoAlsaSink`/`EchoAudioHandler` adapter for `shairplay` PCM callbacks.

## Shairport port status

This is imported hardware code, **not** a C library yet. `EchoAudioHandler` is coupled to Rust `shairplay`; Shairport Sync cannot link it directly.

The intended next step is a small, panic-safe C ABI over the hardware core, then a Shairport `audio_output` backend. Preserve real ALSA delay/flush behavior: an extra blind queue would undermine AP2 timing and multi-room synchronization.

See [`../../PORTING_HANDOFF.md`](../../PORTING_HANDOFF.md).

## Target safety

`preflight` configures PCM but does not play audio. `pcm_probe` and normal sink use can change mixer state and write audio/silence. Run target binaries only with explicit hardware-test approval.

# echo-alsa

Raw ALSA output and speaker-route support for ARMv7 Echo Dot Minimal Base devices running Android 7.1/API 25.

## Provides

- PCM card 0, device 23 at 48 kHz, stereo, S16_LE.
- ARMv7 Linux ALSA ioctl layouts and a compile-time C ABI probe.
- Echo output-route and MFP-GPIO recovery plus narrow system-mixer operations for external policy.
- `preflight` and `pcm_probe` diagnostics.
- A panic-safe C ABI and static archive (`libecho_alsa.a`) for non-Rust consumers.
- The legacy Rust `EchoAlsaSink`/`EchoAudioHandler` adapter when built with `--features shairplay`.

## C ABI

The crate stays independent of Shairport Sync. Build it for the target as a static archive:

```sh
cargo build --release --target armv7-unknown-linux-musleabihf
# target/armv7-unknown-linux-musleabihf/release/libecho_alsa.a
```

Its C surface is [`include/echo_alsa.h`](include/echo_alsa.h): fixed-format S16_LE writes,
real PCM delay/stats, volume, mute, flush, and lifecycle. The API has no packet queue and no
Shairport types. Compile the header contract with:

```sh
tests/check-ffi-header.sh
```

Shairport's `audio_echo` backend links this archive directly. In the packaged receiver it owns only PCM, route setup, and XRUN GPIO recovery; `echo-volume-control` owns user volume, mute, and the privacy-controlled amplifier. Preserve real ALSA delay/flush behavior: an extra blind queue would undermine AP2 timing and multi-room synchronization.

## Target safety

`preflight` configures PCM but does not play audio. `pcm_probe` and normal sink use can change mixer state and write audio/silence. Run target binaries only with explicit hardware-test approval.

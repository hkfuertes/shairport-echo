# echo-alsa

Raw ALSA output and speaker-route support for ARMv7 Echo Dot Minimal Base devices running Android 7.1/API 25.

## Provides

- PCM card 0, device 23 at 48 kHz, stereo, S16_LE.
- ARMv7/Bionic ALSA ioctl layouts and compile-time C ABI probe.
- Echo mixer lifecycle: PCM master volume, amplifier, GPIO mute, and `Right Channel Only` handling.
- `preflight` and `pcm_probe` diagnostics.
- A panic-safe C ABI and static archive (`libecho_alsa.a`) for non-Rust consumers.
- The legacy Rust `EchoAlsaSink`/`EchoAudioHandler` adapter when built with `--features shairplay`.

## C ABI

The crate stays independent of Shairport Sync. Build it for the target as a static archive:

```sh
cargo build --release --target armv7-linux-androideabi
# target/armv7-linux-androideabi/release/libecho_alsa.a
```

Its C surface is [`include/echo_alsa.h`](include/echo_alsa.h): fixed-format S16_LE writes,
real PCM delay/stats, volume, mute, flush, and lifecycle. The API has no packet queue and no
Shairport types. Compile the header contract with:

```sh
tests/check-ffi-header.sh
```

A future Shairport `audio_output` backend will link this archive directly. Preserve real ALSA
delay/flush behavior: an extra blind queue would undermine AP2 timing and multi-room synchronization.

See [`../../PORTING_HANDOFF.md`](../../PORTING_HANDOFF.md).

## Target safety

`preflight` configures PCM but does not play audio. `pcm_probe` and normal sink use can change mixer state and write audio/silence. Run target binaries only with explicit hardware-test approval.

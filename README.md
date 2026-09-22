# shairport-echo

Android ARMv7 port of Shairport Sync for the Echo Dot Biscuit.

- `third_party/shairport-sync` and `third_party/nqptp` are pristine pinned upstream snapshots.
- Shairport will use its upstream ALSA backend with `config/echo-alsa.conf`, which maps `echo` to PCM card 0, device 23.
- `Dockerfile` builds a reproducible Android API 24 `alsa-open-probe` against static upstream `alsa-lib` 1.2.14. The probe opens, configures and closes PCM without writing frames.
- `libs/echo-alsa` remains an independent Rust hardware library and fallback; it is not in the first Shairport audio path.
- `libs/echo-controls` is reserved for v2.

The silent target probe passed on the Biscuit: `echo` negotiated S16_LE, 48 kHz, stereo, 1,024-frame periods and a 4,096-frame buffer. The existing `ledcontroller` service owns that PCM normally, so production startup must coordinate its lifecycle before Shairport opens it.

See [PORTING_HANDOFF.md](PORTING_HANDOFF.md) and [third_party/README.md](third_party/README.md).

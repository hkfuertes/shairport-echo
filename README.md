# shairport-echo

Android ARMv7 port of Shairport Sync for the Echo Dot Biscuit.

- `third_party/shairport-sync` and `third_party/nqptp` are pristine pinned upstream snapshots.
- Shairport will use its upstream ALSA backend with `config/echo-alsa.conf`, which maps `echo` to PCM card 0, device 23.
- `Dockerfile` builds a reproducible Android API 24 `alsa-open-probe` against static upstream `alsa-lib` 1.2.14. The probe opens, configures and closes PCM without writing frames.
- Docker target `nqptp-artifact` builds NQPTP for Android API 24 using external Bionic compatibility patches; it uses `/dev/shm` for the NQPTP–Shairport shared-memory interface. `scripts/nqptp-smoke.sh` validates that interface and the PTP ports without opening PCM.
- Docker target `uuid-artifact` builds the static Android `libuuid` required for AirPlay device identifiers.
- Docker target `shairport-artifact` builds Android API 24 Shairport Sync with AirPlay 2, upstream ALSA and TinySVCmDNS. Its external Bionic and TinySVC patches remain under `patches/shairport-sync/`.
- `config/echo-shairport-sync.conf` and `scripts/echo-airplay.sh` start the NQPTP/Shairport control plane. The launcher never enables the speaker route. `scripts/echo-route.sh on` is reserved for an attended playback test; `off` restores the safe idle route.
- `libs/echo-alsa` remains an independent Rust hardware library and fallback; it is not in the first Shairport audio path.
- `libs/echo-controls` is reserved for v2.

The silent target probe passed on the Biscuit: `echo` negotiated S16_LE, 48 kHz, stereo, 1,024-frame periods and a 4,096-frame buffer. The Android Shairport binary also executed on Biscuit and its active control plane advertises both `_airplay._tcp` and `_raop._tcp` as `Echo Shairport`. The existing `ledcontroller` service owns that PCM normally, so production startup must coordinate its lifecycle before Shairport opens it.

See [PORTING_HANDOFF.md](PORTING_HANDOFF.md) and [third_party/README.md](third_party/README.md).

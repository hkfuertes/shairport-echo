# Vendored Echo artifacts

These ARMv7-musl artifacts are built from [echo-libs](https://github.com/hkfuertes/echo-libs) (currently private). `libecho_alsa.a` and `echo_alsa.h` came from revision `8f76eba24d9d5a271e7f90bdf969e473d6371140`; `speakerd` came from `8bf4285e022e9c6e5cc565f87ce678b3591288be`.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `libecho_alsa.a` | Raw Echo PCM/mixer C ABI linked by `audio_echo` | `dfb91dca039fb3df8ca7804b0af67c4f04591594d0eec97cdf79ed2ba4216de5` |
| `echo_alsa.h` | Header for `libecho_alsa.a` | `e56cbbbe76ba5939c6d9e57d47dcbcd1d3676099ba7bda018bebaf8ecc0f646b` |
| `speakerd` | Static mode, LED, metadata, privacy-amp, and service supervisor | `6468e733d3a1924141f4683e9a5d2faf4d3b8bb60459febf1d64c9d8e93650af` |

`entropy-seed/entropy-seed.c` is intentionally source: it is this project's small boot-time helper and Docker compiles it for the target.

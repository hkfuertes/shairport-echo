# Vendored Echo artifacts

These ARMv7-musl artifacts are built from [echo-libs](https://github.com/hkfuertes/echo-libs) (currently private). `libecho_alsa.a` and `echo_alsa.h` came from revision `8f76eba24d9d5a271e7f90bdf969e473d6371140`; `speakerd` came from `438a3bf90048924ed780d57b12eb23cb4da4c50d`.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `libecho_alsa.a` | Raw Echo PCM/mixer C ABI linked by `audio_echo` | `dfb91dca039fb3df8ca7804b0af67c4f04591594d0eec97cdf79ed2ba4216de5` |
| `echo_alsa.h` | Header for `libecho_alsa.a` | `e56cbbbe76ba5939c6d9e57d47dcbcd1d3676099ba7bda018bebaf8ecc0f646b` |
| `speakerd` | Static mode, LED, metadata, privacy-amp, and service supervisor | `f4cea189e6fdb6286db2fd463edffd08e54fb90dea95bd1369acfa7547696119` |

`entropy-seed/entropy-seed.c` is intentionally source: it is this project's small boot-time helper and Docker compiles it for the target.

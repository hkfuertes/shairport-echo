# Vendored Echo artifacts

These ARMv7-musl artifacts are built from [echo-libs](https://github.com/hkfuertes/echo-libs) (currently private), revision `8f76eba24d9d5a271e7f90bdf969e473d6371140`.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `libecho_alsa.a` | Raw Echo PCM/mixer C ABI linked by `audio_echo` | `dfb91dca039fb3df8ca7804b0af67c4f04591594d0eec97cdf79ed2ba4216de5` |
| `echo_alsa.h` | Header for `libecho_alsa.a` | `e56cbbbe76ba5939c6d9e57d47dcbcd1d3676099ba7bda018bebaf8ecc0f646b` |
| `echo-volume-control` | Static LED, metadata, and privacy-amp service | `8185e3e0414c3370b118a9484db6adca229c92541aa0822e7f3b4662f8ef8eca` |

`entropy-seed/entropy-seed.c` is intentionally source: it is this project's small boot-time helper and Docker compiles it for the target.

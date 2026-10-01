# Vendored Echo artifacts

`speakerd` is a prebuilt ARMv7 musl binary from the pinned echo-libs submodule (`third_party/echo-libs`, `main` at `7e7acfc`), never source. Refresh it with `make update-speakerd`, then update this table. The build compiles echo-alsa's C source from that same submodule into Shairport.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `speakerd` | Static mode, LED, metadata, privacy-amp, and service supervisor | `8348b4b6fcd378bf27ef266cc0b2d19d0b5ab393c84ddd586c841f685bfd94bd` |

`entropy-seed/entropy-seed.c` is intentionally source: it is this project's small boot-time helper and Docker compiles it for the target.

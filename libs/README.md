# Vendored Echo artifacts

`speakerd` is a prebuilt ARMv7 musl binary from the pinned echo-libs submodule (`third_party/echo-libs`, `main` at `355545f`), never source. Refresh it with `make update-speakerd`, then update this table. The build compiles echo-alsa's C source from that same submodule into Shairport.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `speakerd` | Static mode, LED, metadata, privacy-amp, and service supervisor | `935441dfa17e4b784f8141382664d300be524d9d3552d98b69fdcc2086fae3c5` |

`entropy-seed/entropy-seed.c` is intentionally source: it is this project's small boot-time helper and Docker compiles it for the target.

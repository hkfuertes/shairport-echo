# Handoff: port Shairport Sync with Echo hardware libraries

## Current state

- New local repository: `/home/hkfuertes/projects/shairport-echo`, branch `main`, no remote.
- `libs/echo-alsa` was moved from `../shairplay-echo-alsa/crates/shairplay-echo-alsa`.
- `libs/echo-controls` was moved from `../shairplay-echo-alsa/crates/shairplay-echo-controls`.
- The associated ARMv7 C layout probes moved with them into each crate's `tests/` directory.
- Both crates passed isolated Rust tests, Android API 24 ARMv7 release builds, and compile-time C ABI layout probes after the move. Generated `Cargo.lock` files are intentionally retained; `target/` is ignored.
- No C ABI, no Shairport backend, no target deployment, and no audio test has been added.

Read [`README.md`](README.md) first. The old source worktree is on `feat/android-jni` and intentionally has uncommitted deletions from this physical move; its `main` history still contains the original working application. Do not try to resume development from that dirty worktree.

## Why this split

Shairport Sync should own AirPlay/AP2, timing, multi-room and stereo-pair protocol behavior. These crates should own Echo-specific hardware behavior only: PCM 0:23, mixer/amp/GPIO lifecycle, LED/input access, and ABI checks.

The current `echo-alsa` crate is not yet a C library. Its public `EchoAlsaSink`/`EchoAudioHandler` path is coupled to Rust `shairplay::AudioHandler`; it cannot be dropped directly into a C process.

## Source to integrate

- C source: `/home/hkfuertes/projects/shairport-sync`
- Current local branch/HEAD inspected: `agent/static-android-airplay2` at `ef34e937`.
- Its documented backend seam is `audio.h`'s `audio_output`: `init`, `prepare`, `get_configuration`, `configure`, `start`, `play`, `stop`, `flush`, `delay`, `stats`, `volume`, and `mute`.

Use that seam; do not copy Shairport protocol code into Rust.

## Recommended next slices

1. **Decouple the hardware core.** Split the raw PCM/mixer/lifecycle portion from `sink.rs` so a C-facing build does not need the `shairplay` trait implementation or pull its LGPL dependency into a static archive unnecessarily. Preserve all existing Rust tests and ARMv7 layout probes.
2. **Add a narrow C ABI.** Prefer `staticlib` first and one checked-in C header with opaque handles and primitive types only. Minimum functions: open/configure, S16_LE frame write, flush, actual delay/stats, dB volume, mute, shutdown. Catch panics/errors at the ABI boundary; never expose Rust types or unwind into C.
3. **Implement `audio_echo.c` in Shairport Sync.** Register one `audio_output` backend. Negotiate the actual Echo format deliberately (currently 48 kHz, stereo, S16_LE) so Shairport performs any resample/mix. Verify the exact units/layout supplied to `play()` before forwarding frames.
4. **Preserve sync.** Do not blindly put Shairport's `play()` behind the current extra packet queue. AP2 synchronization depends on backend timing: `delay()` and `stats()` need meaningful hardware data, and flush/start/mute lifecycle must be deterministic. A double buffer with guessed latency can defeat the multi-room benefit.
5. **Port/build Shairport Sync for Android separately.** Audit its dependencies and Android/Bionic portability before touching hardware. Use the existing API-24 ARMv7 NDK setup as a baseline, but pin/checksum every new build input.
6. **Validate in order.** Host/unit + C ABI tests; silent target preflight; Shairport AP1 output; AP2 one receiver; only then two-device timing/multi-room. Do not claim stereo pair or multi-room from a successful compile.

## Controls

`libs/echo-controls` moved for ownership consistency. Do not integrate buttons or LEDs into the first Shairport output backend unless explicitly requested.

## Safety and provenance

- Do not access target hardware, restart services, open PCM, modify mixer/LED state, or play audio without explicit user approval.
- Keep the ARMv7 Android ABI assumptions and C probes; target binaries must not be executed on the x86 build host.
- Review Shairport Sync and Rust dependency licensing before linking/distributing a combined artifact. Preserve the existing no-copy/provenance rules for EchoLocal and unrelated reference code.

## Suggested skills

- `context-mode` for builds and large compiler output.
- `diagnose` for C/Rust ABI, timing, or Android linker failures.
- `handoff` before changing sessions.

# Handoff: Shairport Sync on Echo Biscuit

## Current audio A/B (`fix/audio-quality`)

- Commit `6baa539` adds the optional `echo` Shairport backend, linked to the separate `libecho_alsa.a` through `echo_alsa.h`; default `alsa` remains available.
- The first direct arbitrary-frame bridge failed audibly with `EPIPE`. The current bridge accumulates 1,024-frame periods, retries the same period after an XRUN, and includes its pending frames in Shairport delay/statistics accounting.
- An attended Biscuit test validated clean audible playback and iPhone AirPlay volume changes. The log recorded no `EPIPE`/period-write failures and hardware volume requests from -17.10 dB through 0 dB.
- `echo-airplay crate` selects `echo-shairport-sync-echo.conf`, which limits hardware volume to 30 dB so normal AirPlay slider positions remain audible.
- Physical Echo volume/buttons to iPhone are not implemented. The earlier report that audio continued after the sender app was killed also needs a dedicated teardown test; do not claim it is fixed.

## Current state

- Repository: `/home/hkfuertes/projects/shairport-echo`, branch `main`, private origin `https://github.com/hkfuertes/shairport-echo.git`.
- Upstream sources are pristine snapshots in `third_party/`: Shairport Sync 5.5.2 (`7bad231c18368dbd26f298577f6210e36e4b0797`) and NQPTP 1.2.8 (`c925f27c1fd12e4033ac477e5a405969b0b0260b`). Keep project changes outside those trees.
- `libs/echo-alsa` and `libs/echo-controls` remain independent Rust packages. `echo-alsa` now has a C-safe static-library ABI, but it is a fallback, not the first Shairport audio path. Controls are deferred to v2.
- `Dockerfile` builds `alsa-open-probe`: an ARMv7 Android API 24 PIE linked to Bionic and static upstream ALSA 1.2.14. ALSA's unsupported SysV-SHM components are excluded using upstream configure options and `ac_cv_header_sys_shm_h=no`.
- The silent probe was built and ran on the attached Biscuit. With `ALSA_CONFIG_PATH` set to `config/echo-alsa.conf`, it opened and configured `echo` as S16_LE, 48,000 Hz, 2 channels, 1,024-frame periods and a 4,096-frame buffer, then closed without writing a frame.
- Docker target `nqptp-artifact` now builds an Android ARMv7 NQPTP binary. Its external patches replace Linux-only `-lpthread`/`-lrt` checks, provide the API-24 shared-memory compatibility layer using `/dev/shm`, avoid unsupported pthread cancellation and avoid `MAP_LOCKED` on Bionic.
- `scripts/nqptp-smoke.sh` passed on the Biscuit: it started NQPTP, observed `/dev/shm/nqptp` and UDP 319/320, then stopped it and removed its temporary `/dev/shm` mount. It never opens PCM.
- Docker target `uuid-artifact` builds static Android `libuuid` 2.40.4, needed by Shairport's AirPlay identifiers. The input archive is SHA-256 pinned.
- Docker target `shairport-artifact` builds Shairport Sync 5.5.1 for Android API 24 with AirPlay 2, upstream ALSA, TinySVCmDNS and static third-party dependencies. It needs only Bionic `libc`, `libdl` and `libm` at runtime; the binary executed successfully with `-V` on Biscuit.
- The active Biscuit control plane runs NQPTP plus Shairport from `/data/local/tmp/shairport-echo`. It owns UDP 319/320 and TCP 7000, creates `/dev/shm/nqptp`, and answers multicast queries for both `_airplay._tcp` and `_raop._tcp` as `Echo Shairport`. A silent AirPlay 2 stream, song change, switch back to the sender and clean session teardown all passed with the speaker route off.
- Android lacks pthread cancellation. `0006-cooperatively-cancel-ap2-receivers-on-android.patch` makes the two AP2 `recv` loops check cancellation after `EINTR`; it fixed leaked AP2 threads and a sender hang on song changes. `echo-airplay verbose` starts Shairport with `-vv` for a reproducible diagnostic capture.

## Target state and safety

- Target: rooted Echo Dot Minimal Base (`biscuit`), Android 7.1.2/API 25, ARMv7, permissive SELinux.
- The physical speaker amplifier is currently confirmed `Off`; audio is intentionally deferred until the user validates it in person.
- `/system/bin/ledcontroller` (currently a symlink to `airplayd`) owns `pcmC0D23p` normally and also conflicted with the vendor mDNS announcement. It is currently stopped through init so Shairport is the only advertised receiver; restore it with `setprop ctl.start ledcontroller` if the Shairport control plane is stopped or abandoned.
- Future production startup must deliberately coordinate that service, rather than racing it for the exclusive PCM device.

## Architecture decision

Use Shairport Sync's upstream `audio_alsa.c`, statically linking ALSA for Android. Configure `alsa.output_device = "echo"` and ship `config/echo-alsa.conf` to map that alias to hardware card 0/device 23.

Do not add `audio_echo.c` or link the Rust static library into the first receiver. Retain `echo-alsa` for standalone diagnostics and a fallback only if upstream ALSA proves inadequate.

## Build and validation

```sh
docker build --target shairport-artifact -t shairport-echo-shairport:local .
docker build --target nqptp-artifact -t shairport-echo-nqptp:local .
```

The Shairport artifact contains `/shairport-sync`, `/echo-alsa.conf`, `/echo-shairport-sync.conf`, `/echo-airplay` and `/echo-route`. `echo-route on` is intentionally untested and must only be run for an attended playback test; it is already deployed but not invoked on Biscuit.

Next slices:

1. Enable the known-good Echo route only for an attended playback test, then verify actual ALSA format/delay and audible output.
2. Exercise full-daemon stop/restart behavior; the current Android pthread-cancellation shim still requires the launcher's SIGKILL fallback for shutdown.
3. Validate two-device AirPlay 2 timing/multi-room before claiming it works.
4. If testing an advertised HomePod-mini icon, keep it a reversible mDNS/GetInfo metadata override; it must not be represented as HomeKit or native HomePod stereo support.
5. Package only after license/source-compliance review; do not add controls or LEDs in v1.

## Constraints

- Do not edit vendored upstream trees in place.
- Target binaries must be Android ARMv7/API 24-compatible; do not run them on the x86 host.
- Do not enable the speaker route or amplifier until an attended playback test; the user has granted device authority but asked to validate audible output in person.
- NQPTP is GPL-licensed; review distribution obligations before a release ZIP.

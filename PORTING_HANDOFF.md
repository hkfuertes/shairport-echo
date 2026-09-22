# Handoff: Shairport Sync on Echo Biscuit

## Current state

- Repository: `/home/hkfuertes/projects/shairport-echo`, branch `main`, private origin `https://github.com/hkfuertes/shairport-echo.git`.
- Upstream sources are pristine snapshots in `third_party/`: Shairport Sync 5.5.2 (`7bad231c18368dbd26f298577f6210e36e4b0797`) and NQPTP 1.2.8 (`c925f27c1fd12e4033ac477e5a405969b0b0260b`). Keep project changes outside those trees.
- `libs/echo-alsa` and `libs/echo-controls` remain independent Rust packages. `echo-alsa` now has a C-safe static-library ABI, but it is a fallback, not the first Shairport audio path. Controls are deferred to v2.
- `Dockerfile` builds `alsa-open-probe`: an ARMv7 Android API 24 PIE linked to Bionic and static upstream ALSA 1.2.14. ALSA's unsupported SysV-SHM components are excluded using upstream configure options and `ac_cv_header_sys_shm_h=no`.
- The silent probe was built and ran on the attached Biscuit. With `ALSA_CONFIG_PATH` set to `config/echo-alsa.conf`, it opened and configured `echo` as S16_LE, 48,000 Hz, 2 channels, 1,024-frame periods and a 4,096-frame buffer, then closed without writing a frame.
- Docker target `nqptp-artifact` now builds an Android ARMv7 NQPTP binary. Its external patches replace Linux-only `-lpthread`/`-lrt` checks, provide the API-24 shared-memory compatibility layer using `/dev/shm`, avoid unsupported pthread cancellation and avoid `MAP_LOCKED` on Bionic.
- `scripts/nqptp-smoke.sh` passed on the Biscuit: it started NQPTP, observed `/dev/shm/nqptp` and UDP 319/320, then stopped it and removed its temporary `/dev/shm` mount. It never opens PCM.

## Target state and safety

- Target: rooted Echo Dot Minimal Base (`biscuit`), Android 7.1.2/API 25, ARMv7, permissive SELinux.
- The physical speaker amplifier was confirmed `Off` before and after the probe. Do not enable it or write PCM until the user explicitly permits audio.
- `/system/bin/ledcontroller` (currently a symlink to `airplayd`) owns `pcmC0D23p` normally. It is an init-supervised service. The silent probe used `ctl.stop ledcontroller`, opened PCM, then restored it with `ctl.start ledcontroller`; afterwards the service again owned PCM and the amplifier remained off.
- Future Shairport startup must deliberately coordinate that service, rather than racing it for the exclusive PCM device.

## Architecture decision

Use Shairport Sync's upstream `audio_alsa.c`, statically linking ALSA for Android. Configure `alsa.output_device = "echo"` and ship `config/echo-alsa.conf` to map that alias to hardware card 0/device 23.

Do not add `audio_echo.c` or link the Rust static library into the first receiver. Retain `echo-alsa` for standalone diagnostics and a fallback only if upstream ALSA proves inadequate.

## Build and validation

```sh
docker build --target artifact -t shairport-echo-alsa-probe:local .
```

The image contains `/alsa-open-probe` and `/echo-alsa.conf`. The probe never calls `snd_pcm_prepare`, `snd_pcm_start`, or a write operation.

Next slices:

1. Extend the Android Docker build with Shairport Sync's AirPlay 2 dependencies, keeping all source versions and checksums pinned.
2. Build Shairport with `--with-airplay-2 --with-alsa --with-tinysvcmdns` and the upstream ALSA backend.
3. Run NQPTP silently on target with a temporary `/dev/shm` mount, then validate its control/shared-memory interface.
4. Perform a silent Shairport execution/configuration test while `ledcontroller` is safely coordinated.
5. Validate discovery and AirPlay 2 control-plane behaviour before enabling any speaker route or audio playback.
6. Package only after license/source-compliance review; do not add controls or LEDs in v1.

## Constraints

- Do not edit vendored upstream trees in place.
- Target binaries must be Android ARMv7/API 24-compatible; do not run them on the x86 host.
- No noise: no PCM writes, no route enable, no amplifier enable without fresh user permission.
- NQPTP is GPL-licensed; review distribution obligations before a release ZIP.

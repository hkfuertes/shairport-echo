# shairport-echo

Experimental AirPlay 2 receiver for rooted Android ARMv7 Echo Dot Minimal Base devices. It runs static [Shairport Sync](https://github.com/mikebrady/shairport-sync) and [NQPTP](https://github.com/mikebrady/nqptp) through the existing `ledcontroller` init-service contract—no APK, boot image, or ramdisk change required.

> **Device-test software, not a release.** It has been exercised on Biscuit and Radar/Radar Puffin, but distribution licensing, multi-room timing, and HomePod-style stereo-pair support are not complete.

## What it does

- Receives AirPlay 2 on the Echo's Android 7.1.2 ARMv7 system with TinySVCmDNS discovery and NQPTP timing. Shairport and NQPTP are fully static musl binaries (real `pthread_cancel`, no Bionic dependency or compatibility patches).
- Uses vendored static `libs/libecho_alsa.a` through Shairport's `audio_echo` backend: 48 kHz stereo S16_LE, 1,024-frame periods, four-period PCM buffer, mono speaker mixdown, and retry of the same period after an XRUN.
- Runs a separate static `echo-volume-control` daemon for the LED ring, ALSA control events, and mixer/privacy policy. Shairport emits upstream metadata over localhost UDP; it no longer contains Echo button, LED, or AP2 reverse-control logic. The shipped `--no-volume-buttons` deliberately leaves local `+/-` inactive. With `--sync-with-mic-mute`, the physical privacy state is the speaker-amp kill switch; AirPlay playback never controls it. The audio backend retains PCM, routing and XRUN recovery.
- Builds a reversible TWRP ZIP for `biscuit`, `radar`, and `radar_puffin` only. It preserves the original regular `/system/bin/ledcontroller`, replaces it with a regular shell entrypoint, and restores the original on uninstall. The `shairport-sync` binary itself is generic: all Echo-specific setup lives in that shell.
- Turns off the inherited LED boot animation. `/data/shairport-sync.conf` is seeded on first boot (name from `ro.product.name`) and is authoritative thereafter: any Shairport Sync option can be set there. `general.model` (patch) picks the sender icon, e.g. `AirPort10,115`, `AudioAccessory5,1`, or `AudioAccessory1,1`. Before mDNS starts, `ledcontroller` sets a MAC-derived hostname (`shairport-<wlan0 MAC>`) so multiple Echos never collide as `localhost.local`. `/data/shairport-echo.seed` restores kernel entropy on later boots, avoiding musl's `getrandom()` startup wait.
- Shairport patches live in `patches/shairport-sync/`: the three-patch series (`--with-echo-alsa` with external controls, `general.model`, TinySVCmDNS AP2 registration) applies alone to pristine upstream, e.g. from Buildroot.

## Build and test

```sh
# Requires Docker. The base image is digest-pinned and the musl toolchain is checksum-verified.
make       # build the ARMv7 artifact and install/uninstall ZIPs in out/
make test  # also run the host-side ZIP/install/upgrade/uninstall regression
```

Docker downloads SHA-256-pinned Shairport Sync 5.5.1 and NQPTP 1.2.8 source archives, applies the tracked patch series, and uses checked-in Echo artifacts from the revision recorded in [`libs/README.md`](libs/README.md). The digest-pinned Debian base and checksum-verified musl.cc `armv7l-linux-musleabihf` toolchain keep builds repeatable from a clean machine.

Install the generated ZIP from TWRP on an explicitly selected supported device.

## Validation status

- `make test` verifies ZIP structure, device gating, fresh install, upgrade, uninstall, config seeding, MAC-derived hostname generation, and `/data/shairport-sync.conf` preservation.
- Biscuit exercised AP2 realtime and buffered playback, metadata-driven volume/ring updates, and the physical privacy `0/1` speaker-amp gate.
- Radar/Radar Puffin exercised a fresh TWRP install, AirPlay discovery and connection from iPhone and Mac alongside Biscuit, plus the `AudioAccessory1,1` advertisement. The dual-device test exposed and then verified the `localhost.local` collision fix.
- This does **not** establish multi-room timing, pairing persistence across service restarts, or native HomePod stereo pairing.

## Limits and safety

- NQPTP is GPLv2; the combined Shairport/NQPTP/Rust distribution-license review remains open.
- AirPlay 2 pairing persistence across service restarts has not been designed or validated.
- Do not claim native HomePod-style stereo-pair support. Validate sender behavior and two-device timing before claiming multi-room support.
- The installer never writes boot, recovery, cache, persist, or `/data`. At runtime, `ledcontroller` owns `/data/shairport-sync.conf`, `/data/shairport-echo.seed`, and `/data/shairport-echo.log`.

## Credits

- [Shairport Sync](https://github.com/mikebrady/shairport-sync) by Mike Brady and contributors provides the AirPlay receiver implementation.
- [NQPTP](https://github.com/mikebrady/nqptp) by Mike Brady and contributors provides the PTP timing service.
- [EchoLocal](https://github.com/ygelfand/echolocal) is the upstream source from which this repository's Echo hardware crates were derived; it also informed recovery-based deployment.

## AI-assisted development

This project was developed with substantial assistance from AI coding agents. Maintainers remain responsible for review, hardware safety, security, licensing, and release decisions.

## Repository layout

- `Dockerfile`: downloads pinned Shairport Sync and NQPTP archives during the build.
- [`patches/shairport-sync/`](patches/shairport-sync/): vendorable mDNS, model and Echo-audio patches.
- [`libs/`](libs/README.md): prebuilt Echo ALSA/header/control-service artifacts; only `entropy-seed` keeps local source.
- [`twrp/`](twrp/): reversible installer and package build tooling.

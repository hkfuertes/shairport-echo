# shairport-echo

Experimental AirPlay 2 receiver for rooted Android ARMv7 Echo Dot Minimal Base devices. It runs static [Shairport Sync](https://github.com/mikebrady/shairport-sync) and [NQPTP](https://github.com/mikebrady/nqptp) through `speakerd`, reached by the existing `ledcontroller` init-service name—no APK, boot image, or ramdisk change required.

> **Device-test software, not a release.** It has been exercised on Biscuit and Radar/Radar Puffin, but distribution licensing, multi-room timing, and HomePod-style stereo-pair support are not complete.

## What it does

- Receives AirPlay 2 on the Echo's Android 7.1.2 ARMv7 system with TinySVCmDNS discovery and NQPTP timing. Shairport and NQPTP are fully static musl binaries (real `pthread_cancel`, no Bionic dependency or compatibility patches).
- Uses vendored static `libs/libecho_alsa.a` through Shairport's `audio_echo` backend: 48 kHz stereo S16_LE, 1,024-frame periods, four-period PCM buffer, mono speaker mixdown, and retry of the same period after an XRUN.
- Vendors static `speakerd` for the LED ring, ALSA control events, mixer/privacy policy, and service supervision. The installer makes `/system/bin/ledcontroller -> speakerd`; NQPTP is its always-on service and Shairport its only mode `init`. Shairport sends volume metadata over localhost UDP, while `speakerd` keeps local `+/-` off and makes privacy the speaker-amp kill switch.
- Builds a reversible TWRP ZIP for `biscuit`, `radar`, and `radar_puffin` only. It preserves the original regular `/system/bin/ledcontroller`, installs `/system/bin/speakerd`, links `ledcontroller` to it, and restores the original on uninstall.
- The installer seeds `/data/shairport-sync.conf` and `/data/speakerd.ini` only when absent; both are authoritative thereafter. `speakerd` turns off the inherited LED boot animation. Its NQPTP service (`nqptp-service`) restores `/data/shairport-echo.seed`, waits for Wi-Fi, sets a MAC-derived hostname (`shairport-<wlan0 MAC>`) so multiple Echos never collide as `localhost.local`, and mounts `/dev/shm` if missing. `general.model` (patch) picks the sender icon, e.g. `AirPort10,115`, `AudioAccessory5,1`, or `AudioAccessory1,1`.
- Shairport patches live in `patches/shairport-sync/`: the three-patch series (`--with-echo-alsa` with external controls, `general.model`, TinySVCmDNS AP2 registration) applies alone to pristine upstream, e.g. from Buildroot.

## Build and test

```sh
# Requires Docker. The base image is digest-pinned and the musl toolchain is checksum-verified.
make       # build the ARMv7 artifact and install/uninstall ZIPs in out/
make test  # also run the host-side ZIP/install/upgrade/uninstall regression
```

Docker downloads SHA-256-pinned Shairport Sync 5.5.1 and NQPTP 1.2.8 source archives, applies the tracked patch series, and uses checked-in Echo artifacts with provenance and checksums recorded in [`libs/README.md`](libs/README.md). The digest-pinned Debian base and checksum-verified musl.cc `armv7l-linux-musleabihf` toolchain keep builds repeatable from a clean machine.

Install the generated ZIP from TWRP on an explicitly selected supported device (add `-s <serial>` when several are attached):

```sh
adb reboot recovery
adb push out/shairport-echo-0.2.0-armv7.zip /tmp/
adb shell twrp install /tmp/shairport-echo-0.2.0-armv7.zip
adb reboot
```

The AirPlay name defaults to `ro.product.name`; change `general.name` in `/data/shairport-sync.conf` and reboot.

## Validation status

- `make test` verifies ZIP structure, device gating, fresh install, upgrade, uninstall, both config seeds and preservation, plus the `ledcontroller -> speakerd` link.
- Biscuit ran the `speakerd` package end to end: fresh TWRP install, cold boot with `speakerd` supervising NQPTP (via `nqptp-service`: entropy, hostname, `/dev/shm`) and Shairport, and AirPlay discovery under a custom name.
- With the previous `echo-volume-control` package, Biscuit exercised AP2 realtime and buffered playback, metadata-driven volume/ring updates, and the physical privacy `0/1` speaker-amp gate.
- With the previous package, Radar/Radar Puffin exercised a fresh TWRP install, AirPlay discovery and connection from iPhone and Mac alongside Biscuit, plus the `AudioAccessory1,1` advertisement. The dual-device test exposed and then verified the `localhost.local` collision fix.
- This does **not** establish multi-room timing, pairing persistence across service restarts, or native HomePod stereo pairing.

## Limits and safety

- NQPTP is GPLv2; the combined Shairport/NQPTP/Rust distribution-license review remains open.
- AirPlay 2 pairing persistence across service restarts has not been designed or validated.
- Do not claim native HomePod-style stereo-pair support. Validate sender behavior and two-device timing before claiming multi-room support.
- The installer never writes boot, recovery, cache, or persist. It seeds `/data/shairport-sync.conf` and `/data/speakerd.ini` only when absent, and preserves them on update and uninstall; `speakerd` later owns `/data/speakerd.mode` and `/data/shairport-echo.seed`.

## Credits

- [Shairport Sync](https://github.com/mikebrady/shairport-sync) by Mike Brady and contributors provides the AirPlay receiver implementation.
- [NQPTP](https://github.com/mikebrady/nqptp) by Mike Brady and contributors provides the PTP timing service.
- [EchoLocal](https://github.com/ygelfand/echolocal) is the upstream source from which this repository's Echo hardware crates were derived; it also informed recovery-based deployment.

## AI-assisted development

This project was developed with substantial assistance from AI coding agents. Maintainers remain responsible for review, hardware safety, security, licensing, and release decisions.

## Repository layout

- `Dockerfile`: downloads pinned Shairport Sync and NQPTP archives during the build.
- [`patches/shairport-sync/`](patches/shairport-sync/): vendorable mDNS, model and Echo-audio patches.
- [`libs/`](libs/README.md): prebuilt Echo ALSA/header/`speakerd` artifacts; only `entropy-seed` keeps local source.
- [`twrp/`](twrp/): reversible installer and package build tooling.

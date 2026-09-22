# shairport-echo

Experimental AirPlay 2 receiver for rooted Android ARMv7 Echo Dot Minimal Base devices. It runs static [Shairport Sync](https://github.com/mikebrady/shairport-sync) and [NQPTP](https://github.com/mikebrady/nqptp) through the existing `ledcontroller` init-service contract—no APK, boot image, or ramdisk change required.

> **Device-test software, not a release.** It has been exercised on Biscuit and Radar/Radar Puffin, but distribution licensing, multi-room timing, and HomePod-style stereo-pair support are not complete.

## What it does

- Receives AirPlay 2 on Android 7.1.2/API 25 ARMv7 with TinySVCmDNS discovery and NQPTP timing.
- Uses the separate Rust `libecho_alsa.a` hardware library through Shairport's `audio_echo` backend: 48 kHz stereo S16_LE, 1,024-frame periods, four-period PCM buffer, mono speaker mixdown, and retry of the same period after an XRUN.
- Bridges physical Echo volume, Action buttons, and the LED ring to the ALSA mixer and AirPlay 2 event channel. ALSA remains the volume source of truth.
- Builds a reversible TWRP ZIP for `biscuit`, `radar`, and `radar_puffin` only. It preserves the original regular `/system/bin/ledcontroller`, replaces it with the relative `ledcontroller -> airplayd` symlink, verifies payload hashes, and restores the original on uninstall.
- Turns off the inherited LED boot animation. `/data/AIRPLAY_NAME` is created from `ro.product.name` on first boot and is authoritative thereafter.

## Build and test

```sh
# Requires Docker and the local android-armv7-r27c-research:latest image.
make       # build the ARMv7 artifact and install/uninstall ZIPs in out/
make test  # also run the host-side ZIP/install/upgrade/uninstall regression
```

The tracked sources, patches, checksums, and Rust lockfile make builds repeatable **once that local image exists**. The base image is not yet built or digest-pinned here, so a clean-machine bootstrap is not yet reproducible.

Install only from TWRP on an explicitly selected supported device. Full prerequisites, installation, upgrade, uninstall, and safe manual-development instructions are in [build_and_install.md](build_and_install.md).

## Validation status

- `make test` verifies ZIP structure, payload hashes, device gating, fresh install, upgrade, legacy-package migration, uninstall, and `/data/AIRPLAY_NAME` preservation.
- The ZIP has been installed and hash-verified through TWRP on Biscuit and Radar. Both start NQPTP and Shairport automatically after Wi-Fi is available; the ring is black at boot and PCM remains closed until playback.
- Attended AirPlay playback, iPhone volume, physical controls, and LED behavior have been exercised on hardware. This does **not** establish multi-room synchronization or native HomePod stereo pairing.

## Limits and safety

- NQPTP is GPLv2; the combined Shairport/NQPTP/Rust distribution-license review remains open.
- AirPlay 2 pairing persistence across service restarts has not been designed or validated.
- Do not claim native HomePod-style stereo-pair support. Validate sender behavior and two-device timing before claiming multi-room support.
- The installer never writes boot, recovery, cache, or persist, and preserves `/data/AIRPLAY_NAME` on upgrades and uninstall.

## Repository layout

- [`third_party/`](third_party/README.md): pristine, checksum-pinned upstream snapshots.
- [`patches/shairport-sync/`](patches/shairport-sync/): vendorable Android, audio, and controls patches.
- [`libs/echo-alsa/`](libs/echo-alsa): reusable Echo ALSA C ABI.
- [`libs/echo-controls/`](libs/echo-controls): reusable Echo controls C ABI.
- [`twrp/`](twrp/): reversible installer and package build tooling.

# Build, TWRP, and manual development install

This covers the recoverable TWRP ZIP for rooted Biscuit and Radar devices, plus the older manual development path.

The TWRP lifecycle has been validated on Biscuit and Radar/Radar Puffin. Attended audio quality is validated on Biscuit and basic AirPlay playback has been observed on Radar; before changing a device's hardware configuration, run a read-only preflight and verify its PCM node, ALSA controls, and `ledcontroller` ownership.

## Prerequisites

- Docker.
- The local Android build image `android-armv7-r27c-research:latest`.
- `adb` access to the rooted Biscuit or Radar Android ARMv7 target.
- Permission to stop `ledcontroller` and make attended audio tests.

The source snapshots, patches, Rust lockfile, dependency versions, and source checksums are tracked. The build is repeatable once the local base image exists, but is **not yet bootstrap-reproducible** on a clean machine because that base image is not built or digest-pinned in this repository.

```sh
docker image inspect android-armv7-r27c-research:latest >/dev/null
```

## Build

Use the branch/ref containing the desired backend:

```sh
git checkout main # or the reviewed release branch
docker build --pull=false --target shairport-artifact -t shairport-echo-shairport:local .
docker build --pull=false --target nqptp-artifact -t shairport-echo-nqptp:local .
```

`shairport-artifact` builds the Android API-24 ARMv7 Shairport binary and links the separate Rust `libecho_alsa.a` through `echo_alsa.h`. `nqptp-artifact` builds the matching Android NQPTP binary.

## TWRP ZIP

Build the install and uninstall ZIPs from the complete API-24 ARMv7 artifact:

```sh
make          # build the complete ARMv7 artifact and both ZIPs
make test     # host-only ZIP/install/upgrade/uninstall regression
```

`make zip` builds only; `VERSION=0.1.1 make` selects a different ZIP version.

Artifacts are written to `out/`. The host-only test validates ZIP layout, supported-device filtering, fresh install, upgrade, uninstall, config seeding, and preservation of `/data/shairport-sync.conf`.

The installer accepts only `biscuit`, `radar`, and `radar_puffin` with `armeabi-v7a`. It saves the original regular `/system/bin/ledcontroller` as `ledcontroller.shairport-echo-orig`, installs the runtime in `/system/lib/shairport-echo/`, and replaces `ledcontroller` with a regular shell entrypoint. It never writes boot, recovery, cache, persist, or `/data`; uninstall restores the saved `ledcontroller` and keeps `/data/shairport-sync.conf`.

On first Android boot, `ledcontroller` turns off the inherited LED boot animation, then seeds `/data/shairport-sync.conf` with `name` from `ro.product.name`. Thereafter that file is authoritative: edit the name or any other Shairport Sync option and restart the `ledcontroller` service.

Install only from TWRP on an explicitly selected device:

```sh
serial=G090L91073533XR7 # replace explicitly for Biscuit or Radar
adb -s "$serial" reboot recovery
adb -s "$serial" push out/shairport-echo-0.1.0-armv7.zip /tmp/shairport-echo.zip
adb -s "$serial" shell twrp install /tmp/shairport-echo.zip
adb -s "$serial" reboot
```

The ZIP is a device-test artifact until the combined Shairport/NQPTP/Rust distribution-license review is complete.

## Extract artifacts

```sh
set -eu
out="$PWD/out"
rm -rf "$out"
mkdir -p "$out"

id=$(docker create shairport-echo-shairport:local)
for file in shairport-sync echo-alsa.conf echo-shairport-sync.conf \
  echo-shairport-sync-nosync.conf echo-shairport-sync-echo.conf \
  echo-airplay echo-route; do
  docker cp "$id:/$file" "$out/$file"
done
docker rm "$id"

id=$(docker create shairport-echo-nqptp:local)
docker cp "$id:/nqptp" "$out/nqptp"
docker rm "$id"

sha256sum "$out"/* | tee "$out/SHA256SUMS"
```

Optional: extract the standalone static library and header instead of the complete receiver:

```sh
docker build --pull=false --target echo-alsa-build -t shairport-echo-alsa:local .
id=$(docker create shairport-echo-alsa:local)
docker cp "$id:/src/echo-alsa/target/armv7-linux-androideabi/release/libecho_alsa.a" ./libecho_alsa.a
docker rm "$id"
cp libs/echo-alsa/include/echo_alsa.h ./echo_alsa.h
```

## Safe staging on the target

Set the serial explicitly. `-V` is a version-only check: it does not start NQPTP, open PCM, or change the mixer.

```sh
set -eu
serial=G090L91073533XR7                 # replace with the Biscuit or Radar serial
root=/data/local/tmp/shairport-echo
out="$PWD/out"
files='shairport-sync nqptp echo-airplay echo-route echo-alsa.conf echo-shairport-sync.conf echo-shairport-sync-nosync.conf echo-shairport-sync-echo.conf'

adb -s "$serial" shell "mkdir -p '$root'"
for file in $files; do
  adb -s "$serial" push "$out/$file" "$root/$file.next"
done
adb -s "$serial" shell "chmod 700 '$root/shairport-sync.next' '$root/nqptp.next' '$root/echo-airplay.next' '$root/echo-route.next'; '$root/shairport-sync.next' -V; sha256sum '$root/shairport-sync.next' '$root/nqptp.next'"
```

Only after that check succeeds, stop the old instance and atomically replace its files:

```sh
adb -s "$serial" shell "set -eu
root='$root'
[ ! -x \"\$root/echo-airplay\" ] || \"\$root/echo-airplay\" stop
for file in $files; do mv \"\$root/\$file.next\" \"\$root/\$file\"; done
chmod 700 \"\$root/shairport-sync\" \"\$root/nqptp\" \"\$root/echo-airplay\" \"\$root/echo-route\"
"
```

## Run

On Biscuit, `ledcontroller` normally owns the exclusive PCM device. Stop it only on an authorized test device, and restore it after stopping Shairport. Verify Radar's owner during its preflight instead of assuming it is identical.

```sh
adb -s "$serial" shell 'setprop ctl.stop ledcontroller'
```

Choose one mode:

```sh
# Upstream ALSA control plane. It never enables the speaker route itself.
adb -s "$serial" shell "$root/echo-airplay start"

# Verbose upstream ALSA diagnostics.
adb -s "$serial" shell "$root/echo-airplay verbose"

# Reversible no-local-sync diagnostic; PTP/NQPTP remains enabled.
adb -s "$serial" shell "$root/echo-airplay nosync"

# Validated Echo crate A/B backend. Use only for an attended audio test.
adb -s "$serial" shell "$root/echo-airplay crate"
```

The upstream ALSA mode requires the attended route helper before audible playback:

```sh
adb -s "$serial" shell "$root/echo-route on"
```

The `crate` backend configures the Echo output itself when audio first arrives. It has validated periodized playback and iPhone AirPlay-to-ALSA volume, but physical Echo controls to iPhone and sender-app-kill teardown still need dedicated validation.

Read-only checks:

```sh
adb -s "$serial" shell "$root/echo-airplay status"
adb -s "$serial" shell "cat /proc/asound/card0/pcm23p/sub0/status; /system/bin/tinymix 'Ext_Speaker_Amp_Switch'"
adb -s "$serial" shell "tail -80 '$root/shairport-sync.log'"
```

In `tinymix` output, `>` marks the selected enum value.

## Stop, restore, and remove

```sh
adb -s "$serial" shell "set -eu; '$root/echo-airplay' stop; '$root/echo-route' off; setprop ctl.start ledcontroller"
```

Optional test-device cleanup:

```sh
adb -s "$serial" shell "rm -rf '$root'"
```

Do not use these commands as a production install path; use the TWRP ZIP above. The manual path remains useful for development A/B work only.

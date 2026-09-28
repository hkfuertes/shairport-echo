#!/bin/sh
# Host-only TWRP ZIP structure and reversible installer regression.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
version=${VERSION:-0.2.0}
install_zip=$root/out/shairport-echo-$version-armv7.zip
uninstall_zip=$root/out/shairport-echo-uninstall-$version.zip

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

fail() {
  echo "twrp ZIP test: $*" >&2
  exit 1
}

verify_zip() {
  archive=$1
  [ -f "$archive" ] || fail "missing $archive"
  unzip -t "$archive" >/dev/null || fail "invalid ZIP: $archive"
}

[ ! -e "$root/third_party" ] || fail 'upstream sources must not be vendored'
[ -s "$root/libs/libecho_alsa.a" ] || fail 'missing vendored libecho_alsa.a'
[ -s "$root/libs/echo_alsa.h" ] || fail 'missing vendored echo_alsa.h'
[ -x "$root/libs/speakerd" ] || fail 'missing vendored speakerd'
[ -f "$root/config/speakerd.ini" ] || fail 'missing speakerd config'
[ -x "$root/scripts/nqptp-service.sh" ] || fail 'missing nqptp helper'
sh -n "$root/scripts/nqptp-service.sh" || fail 'invalid nqptp helper'
grep -Fq 'ARG SHAIRPORT_SYNC_COMMIT=' "$root/Dockerfile" || fail 'missing Shairport source pin'
grep -Fq 'ARG NQPTP_COMMIT=' "$root/Dockerfile" || fail 'missing NQPTP source pin'
grep -Fq -- '--with-metadata-multicast' "$root/Dockerfile" ||
  fail 'Shairport must include the UDP metadata sender'
verify_zip "$install_zip"
verify_zip "$uninstall_zip"
unzip -q "$install_zip" -d "$tmp/install"
unzip -q "$uninstall_zip" -d "$tmp/uninstall"

python3 - "$install_zip" "$uninstall_zip" <<'PY'
import stat
import sys
import zipfile

install, uninstall = map(zipfile.ZipFile, sys.argv[1:])
expected_install = {
    "META-INF/com/google/android/update-binary",
    "META-INF/com/google/android/updater-script",
    "payload/system/bin/speakerd",
    "payload/system/lib/shairport-echo/echo-alsa.conf",
    "payload/system/lib/shairport-echo/shairport-sync.conf",
    "payload/system/lib/shairport-echo/speakerd.ini",
    "payload/system/lib/shairport-echo/nqptp",
    "payload/system/lib/shairport-echo/nqptp-service",
    "payload/system/lib/shairport-echo/entropy-seed",
    "payload/system/lib/shairport-echo/shairport-sync",
}
expected_uninstall = {
    "META-INF/com/google/android/update-binary",
    "META-INF/com/google/android/updater-script",
}
assert set(install.namelist()) == expected_install, install.namelist()
assert set(uninstall.namelist()) == expected_uninstall, uninstall.namelist()
for archive in (install, uninstall):
    mode = archive.getinfo("META-INF/com/google/android/update-binary").external_attr >> 16
    assert stat.S_IMODE(mode) == 0o755, oct(mode)
for name in (
    "payload/system/bin/speakerd",
    "payload/system/lib/shairport-echo/nqptp",
    "payload/system/lib/shairport-echo/nqptp-service",
    "payload/system/lib/shairport-echo/entropy-seed",
    "payload/system/lib/shairport-echo/shairport-sync",
):
    mode = install.getinfo(name).external_attr >> 16
    assert stat.S_IMODE(mode) == 0o755, (name, oct(mode))
assert stat.S_IMODE(install.getinfo("payload/system/lib/shairport-echo/speakerd.ini").external_attr >> 16) == 0o644
modes = install.read("payload/system/lib/shairport-echo/speakerd.ini").decode()
assert "\n[services]\n" in modes
assert "nqptp = /system/lib/shairport-echo/nqptp-service" in modes
helper = install.read("payload/system/lib/shairport-echo/nqptp-service").decode()
assert 'exec "$R/nqptp"' in helper
assert "\n[airplay]\n" in modes
assert modes.count("\ninit = ") == 1
assert not any(name.startswith("data/") or "sha256" in name for name in install.namelist())
PY

install_script=$tmp/install/META-INF/com/google/android/update-binary
uninstall_script=$tmp/uninstall/META-INF/com/google/android/update-binary
sh -n "$install_script"
sh -n "$uninstall_script"
sh -n "$tmp/install/payload/system/lib/shairport-echo/nqptp-service"
! grep -E -q '@(MODE|VERSION)@' "$install_script" "$uninstall_script" ||
  fail 'unrendered installer placeholder'
! grep -E -q '/(boot|recovery|cache|persist)(/|$)' "$install_script" "$uninstall_script" ||
  fail 'installer references a forbidden partition'
grep -Fq 'mount /system' "$install_script" || fail 'missing recovery system mount pattern'
file "$tmp/install/payload/system/lib/shairport-echo/shairport-sync" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'Shairport payload is not ARMv7 EABI5'
file "$tmp/install/payload/system/lib/shairport-echo/nqptp" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'NQPTP payload is not ARMv7 EABI5'
file "$tmp/install/payload/system/lib/shairport-echo/entropy-seed" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'entropy-seed payload is not ARMv7 EABI5'
file "$tmp/install/payload/system/bin/speakerd" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'speakerd payload is not ARMv7 EABI5'

cat >"$tmp/getprop" <<'EOF'
#!/bin/sh
case "$1" in
  ro.product.device) printf '%s\n' "$TEST_DEVICE" ;;
  ro.product.name) printf '%s\n' "$TEST_PRODUCT" ;;
  ro.product.cpu.abi) printf '%s\n' armeabi-v7a ;;
  *) exit 1 ;;
esac
EOF
chmod 755 "$tmp/getprop"

run_update() {
  script=$1
  archive=$2
  system=$3
  device=$4
  data=${5:-"$tmp/data-$device"}
  TEST_DEVICE=$device \
    TEST_PRODUCT=$device \
    SHAIRPORT_ECHO_SYSTEM=$system \
    SHAIRPORT_ECHO_DATA="$data" \
    SHAIRPORT_ECHO_TMPDIR="$tmp/runtime-$device" \
    SHAIRPORT_ECHO_GETPROP="$tmp/getprop" \
    SHAIRPORT_ECHO_UNZIP="$(command -v unzip)" \
    SHAIRPORT_ECHO_CHCON=/bin/true \
    SHAIRPORT_ECHO_CHOWN=/bin/true \
    sh "$script" 3 1 "$archive" >/dev/null
}

check_fresh_lifecycle() {
  reported=$1
  normalized=$2
  system=$tmp/$reported/system
  data=$tmp/$reported/data
  lib=$system/lib/shairport-echo
  mkdir -p "$system/bin" "$system/etc" "$system/lib" "$data"
  printf 'original %s ledcontroller\n' "$reported" >"$system/bin/ledcontroller"
  chmod 755 "$system/bin/ledcontroller"
  cp "$system/bin/ledcontroller" "$tmp/$reported-original"

  run_update "$install_script" "$install_zip" "$system" "$reported" "$data"
  [ -L "$system/bin/ledcontroller" ] || fail "$reported ledcontroller is not a symlink"
  [ "$(readlink "$system/bin/ledcontroller")" = speakerd ] ||
    fail "$reported ledcontroller does not point to speakerd"
  cmp "$tmp/install/payload/system/bin/speakerd" "$system/bin/speakerd" ||
    fail "$reported speakerd differs"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller.shairport-echo-orig" ||
    fail "$reported original was not preserved"
  for file in entropy-seed nqptp nqptp-service shairport-sync shairport-sync.conf speakerd.ini echo-alsa.conf; do
    cmp "$tmp/install/payload/system/lib/shairport-echo/$file" "$lib/$file" ||
      fail "$reported $file differs"
  done
  [ ! -e "$lib/echo-volume-control" ] || fail "$reported kept echo-volume-control"
  grep -qx 'owner=shairport-echo' "$system/etc/shairport-echo/installed" || fail "$reported ownership marker"
  grep -qx "device=$normalized" "$system/etc/shairport-echo/installed" || fail "$reported normalized device"
  grep -Fqx "  name = \"$reported\";" "$data/shairport-sync.conf" ||
    fail "$reported Shairport config was not seeded"
  cmp "$lib/speakerd.ini" "$data/speakerd.ini" || fail "$reported speakerd config was not seeded"

  printf '%s\n' '// user config' >"$data/shairport-sync.conf"
  printf '%s\n' '# user mode config' >"$data/speakerd.ini"
  # An update restores missing payload files, removes the superseded daemon, and preserves /data.
  rm "$lib/entropy-seed"
  printf 'legacy\n' >"$lib/echo-volume-control"
  run_update "$install_script" "$install_zip" "$system" "$reported" "$data"
  [ -f "$lib/entropy-seed" ] || fail "$reported update did not add entropy-seed"
  [ ! -e "$lib/echo-volume-control" ] || fail "$reported update kept echo-volume-control"
  [ "$(cat "$data/shairport-sync.conf")" = '// user config' ] ||
    fail "$reported user Shairport config changed"
  [ "$(cat "$data/speakerd.ini")" = '# user mode config' ] ||
    fail "$reported user speakerd config changed"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller.shairport-echo-orig" ||
    fail "$reported update replaced original"

  run_update "$uninstall_script" "$uninstall_zip" "$system" "$reported" "$data"
  [ -f "$system/bin/ledcontroller" ] && [ ! -L "$system/bin/ledcontroller" ] ||
    fail "$reported original was not restored"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller" || fail "$reported restore differs"
  [ ! -e "$system/bin/speakerd" ] || fail "$reported speakerd survived uninstall"
  [ ! -e "$lib" ] || fail "$reported runtime survived uninstall"
  [ ! -e "$system/etc/shairport-echo" ] || fail "$reported marker survived uninstall"
  [ "$(cat "$data/shairport-sync.conf")" = '// user config' ] ||
    fail "$reported user Shairport config changed on uninstall"
  [ "$(cat "$data/speakerd.ini")" = '# user mode config' ] ||
    fail "$reported user speakerd config changed on uninstall"
}

check_legacy_upgrade() {
  system=$tmp/legacy/system
  data=$tmp/legacy/data
  lib=$system/lib/shairport-echo
  mkdir -p "$system/bin" "$system/etc/shairport-echo" "$system/lib" "$data" "$lib"
  printf 'vendor ledcontroller\n' >"$system/bin/ledcontroller"
  chmod 755 "$system/bin/ledcontroller"
  cp "$system/bin/ledcontroller" "$system/bin/ledcontroller.shairport-echo-orig"
  printf 'legacy launcher\n' >"$system/bin/ledcontroller"
  chmod 755 "$system/bin/ledcontroller"
  printf 'legacy control daemon\n' >"$lib/echo-volume-control"
  chmod 755 "$lib/echo-volume-control"
  cat >"$system/etc/shairport-echo/installed" <<'EOF'
owner=shairport-echo
version=0.1.0
device=biscuit
EOF

  run_update "$install_script" "$install_zip" "$system" biscuit "$data"
  [ -L "$system/bin/ledcontroller" ] || fail 'legacy update did not link ledcontroller'
  [ "$(readlink "$system/bin/ledcontroller")" = speakerd ] ||
    fail 'legacy update linked ledcontroller incorrectly'
  [ -x "$system/bin/speakerd" ] || fail 'legacy update did not install speakerd'
  [ ! -e "$lib/echo-volume-control" ] || fail 'legacy update kept echo-volume-control'
  [ -f "$data/shairport-sync.conf" ] || fail 'legacy update did not seed Shairport config'
  [ -f "$data/speakerd.ini" ] || fail 'legacy update did not seed speakerd config'
}

check_fresh_lifecycle biscuit biscuit
check_fresh_lifecycle radar radar_puffin
check_fresh_lifecycle radar_puffin radar_puffin
check_legacy_upgrade

wrong=$tmp/wrong/system
mkdir -p "$wrong/bin" "$wrong/etc" "$wrong/lib"
printf 'vendor\n' >"$wrong/bin/ledcontroller"
chmod 755 "$wrong/bin/ledcontroller"
if run_update "$install_script" "$install_zip" "$wrong" unsupported; then
  fail 'unsupported device was accepted'
fi
[ ! -e "$wrong/bin/ledcontroller.shairport-echo-orig" ] || fail 'unsupported device was modified'

unsafe=$tmp/unsafe/system
mkdir -p "$unsafe/bin" "$unsafe/etc" "$unsafe/lib"
printf 'target\n' >"$unsafe/bin/other"
ln -s other "$unsafe/bin/ledcontroller"
if run_update "$install_script" "$install_zip" "$unsafe" biscuit; then
  fail 'pre-existing ledcontroller symlink was accepted'
fi
[ ! -e "$unsafe/lib/shairport-echo" ] || fail 'unsafe service state was modified'
[ -L "$unsafe/bin/ledcontroller" ] || fail 'unsafe ledcontroller was replaced'

printf '%s\n' 'TWRP ZIP structure, config seeding, install, upgrade and uninstall checks passed'

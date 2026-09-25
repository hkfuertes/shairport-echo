#!/bin/sh
# Host-only TWRP ZIP structure and reversible installer regression.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
version=${VERSION:-0.1.0}
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
[ -x "$root/libs/echo-volume-control" ] || fail 'missing vendored echo-volume-control'
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
    "payload/system/bin/ledcontroller",
    "payload/system/lib/shairport-echo/echo-alsa.conf",
    "payload/system/lib/shairport-echo/shairport-sync.conf",
    "payload/system/lib/shairport-echo/nqptp",
    "payload/system/lib/shairport-echo/entropy-seed",
    "payload/system/lib/shairport-echo/echo-volume-control",
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
    "payload/system/bin/ledcontroller",
    "payload/system/lib/shairport-echo/nqptp",
    "payload/system/lib/shairport-echo/entropy-seed",
    "payload/system/lib/shairport-echo/echo-volume-control",
    "payload/system/lib/shairport-echo/shairport-sync",
):
    mode = install.getinfo(name).external_attr >> 16
    assert stat.S_IMODE(mode) == 0o755, (name, oct(mode))
assert not any(name.startswith("data/") or "sha256" in name for name in install.namelist())
PY

install_script=$tmp/install/META-INF/com/google/android/update-binary
uninstall_script=$tmp/uninstall/META-INF/com/google/android/update-binary
sh -n "$install_script"
sh -n "$uninstall_script"
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
file "$tmp/install/payload/system/lib/shairport-echo/echo-volume-control" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'echo-volume-control payload is not ARMv7 EABI5'

cat >"$tmp/getprop" <<'EOF'
#!/bin/sh
case "$1" in
  ro.product.device) printf '%s\n' "$TEST_DEVICE" ;;
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
  TEST_DEVICE=$device \
    SHAIRPORT_ECHO_SYSTEM=$system \
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
  printf '%s\n' '// user config' >"$data/shairport-sync.conf"

  run_update "$install_script" "$install_zip" "$system" "$reported"
  [ -f "$system/bin/ledcontroller" ] && [ ! -L "$system/bin/ledcontroller" ] ||
    fail "$reported ledcontroller is not a regular shell"
  cmp "$tmp/install/payload/system/bin/ledcontroller" "$system/bin/ledcontroller" ||
    fail "$reported ledcontroller differs"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller.shairport-echo-orig" ||
    fail "$reported original was not preserved"
  for file in entropy-seed echo-volume-control nqptp shairport-sync shairport-sync.conf echo-alsa.conf; do
    cmp "$tmp/install/payload/system/lib/shairport-echo/$file" "$lib/$file" ||
      fail "$reported $file differs"
  done
  grep -qx 'owner=shairport-echo' "$system/etc/shairport-echo/installed" || fail "$reported ownership marker"
  grep -qx "device=$normalized" "$system/etc/shairport-echo/installed" || fail "$reported normalized device"

  # An update may add payload files that the previous install did not have.
  rm "$lib/entropy-seed" "$lib/echo-volume-control"
  run_update "$install_script" "$install_zip" "$system" "$reported"
  [ -f "$lib/entropy-seed" ] || fail "$reported update did not add entropy-seed"
  [ -f "$lib/echo-volume-control" ] || fail "$reported update did not add echo-volume-control"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller.shairport-echo-orig" ||
    fail "$reported update replaced original"

  run_update "$uninstall_script" "$uninstall_zip" "$system" "$reported"
  [ -f "$system/bin/ledcontroller" ] && [ ! -L "$system/bin/ledcontroller" ] ||
    fail "$reported original was not restored"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller" || fail "$reported restore differs"
  [ ! -e "$lib" ] || fail "$reported runtime survived uninstall"
  [ ! -e "$system/etc/shairport-echo" ] || fail "$reported marker survived uninstall"
  [ "$(cat "$data/shairport-sync.conf")" = '// user config' ] ||
    fail "$reported user config changed"
}

check_fresh_lifecycle biscuit biscuit
check_fresh_lifecycle radar radar_puffin
check_fresh_lifecycle radar_puffin radar_puffin

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

"$root/tests/ledcontroller.sh"
printf '%s\n' 'TWRP ZIP structure, install, upgrade and uninstall checks passed'

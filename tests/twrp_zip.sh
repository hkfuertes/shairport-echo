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
  [ -f "$archive.sha256" ] || fail "missing $archive.sha256"
  (cd "$(dirname "$archive")" && sha256sum -c "$(basename "$archive").sha256" >/dev/null) ||
    fail "bad sidecar for $archive"
  unzip -t "$archive" >/dev/null || fail "invalid ZIP: $archive"
}

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
    "payload-manifest.sha256",
    "payload/system/bin/airplayd",
    "payload/system/lib/shairport-echo/echo-alsa.conf",
    "payload/system/lib/shairport-echo/echo-shairport-sync.conf",
    "payload/system/lib/shairport-echo/nqptp",
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
    "payload/system/bin/airplayd",
    "payload/system/lib/shairport-echo/nqptp",
    "payload/system/lib/shairport-echo/shairport-sync",
):
    mode = install.getinfo(name).external_attr >> 16
    assert stat.S_IMODE(mode) == 0o755, (name, oct(mode))
assert not any(name.startswith("data/") or "AIRPLAY_NAME" in name for name in install.namelist())
PY

install_script=$tmp/install/META-INF/com/google/android/update-binary
uninstall_script=$tmp/uninstall/META-INF/com/google/android/update-binary
sh -n "$install_script"
sh -n "$uninstall_script"
! grep -E -q '@(MODE|VERSION|MANIFEST_SHA256)@' "$install_script" "$uninstall_script" ||
  fail 'unrendered installer placeholder'
! grep -E -q '/(boot|recovery|cache|persist)(/|$)' "$install_script" "$uninstall_script" ||
  fail 'installer references a forbidden partition'
grep -Fq 'mount /system' "$install_script" || fail 'missing recovery system mount pattern'
(
  cd "$tmp/install"
  sha256sum -c payload-manifest.sha256 >/dev/null
) || fail 'payload manifest mismatch'
file "$tmp/install/payload/system/lib/shairport-echo/shairport-sync" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'Shairport payload is not ARMv7 EABI5'
file "$tmp/install/payload/system/lib/shairport-echo/nqptp" |
  grep -Eq 'ELF 32-bit LSB .*ARM.*EABI5' || fail 'NQPTP payload is not ARMv7 EABI5'

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
    SHAIRPORT_ECHO_SHA256SUM="$(command -v sha256sum)" \
    SHAIRPORT_ECHO_CHCON=/bin/true \
    SHAIRPORT_ECHO_CHOWN=/bin/true \
    SHAIRPORT_ECHO_FREE_KB=999999 \
    sh "$script" 3 1 "$archive" >/dev/null
}

check_fresh_lifecycle() {
  reported=$1
  normalized=$2
  system=$tmp/$reported/system
  data=$tmp/$reported/data
  mkdir -p "$system/bin" "$system/etc" "$system/lib" "$data"
  printf 'original %s ledcontroller\n' "$reported" >"$system/bin/ledcontroller"
  chmod 755 "$system/bin/ledcontroller"
  cp "$system/bin/ledcontroller" "$tmp/$reported-original"
  printf '%s\n' 'Custom Echo' >"$data/AIRPLAY_NAME"

  run_update "$install_script" "$install_zip" "$system" "$reported"
  [ -L "$system/bin/ledcontroller" ] || fail "$reported did not become a symlink"
  [ "$(readlink "$system/bin/ledcontroller")" = airplayd ] || fail "$reported wrong symlink"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller.airplayd-orig" ||
    fail "$reported original was not preserved"
  grep -qx 'owner=shairport-echo' "$system/etc/shairport-echo/installed" ||
    fail "$reported ownership marker"
  grep -qx "device=$normalized" "$system/etc/shairport-echo/installed" ||
    fail "$reported normalized device"
  cmp "$tmp/install/payload/system/bin/airplayd" "$system/bin/airplayd" ||
    fail "$reported entrypoint differs"
  cmp "$tmp/install/payload/system/lib/shairport-echo/shairport-sync" \
    "$system/lib/shairport-echo/shairport-sync" || fail "$reported Shairport differs"
  [ "$(cat "$data/AIRPLAY_NAME")" = 'Custom Echo' ] || fail "$reported name state changed"

  run_update "$install_script" "$install_zip" "$system" "$reported"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller.airplayd-orig" ||
    fail "$reported upgrade replaced original"

  run_update "$uninstall_script" "$uninstall_zip" "$system" "$reported"
  [ -f "$system/bin/ledcontroller" ] && [ ! -L "$system/bin/ledcontroller" ] ||
    fail "$reported original was not restored"
  cmp "$tmp/$reported-original" "$system/bin/ledcontroller" || fail "$reported restore differs"
  [ ! -e "$system/bin/airplayd" ] || fail "$reported entrypoint survived uninstall"
  [ ! -e "$system/lib/shairport-echo" ] || fail "$reported runtime survived uninstall"
  [ ! -e "$system/etc/shairport-echo" ] || fail "$reported marker survived uninstall"
  [ "$(cat "$data/AIRPLAY_NAME")" = 'Custom Echo' ] || fail "$reported name state changed on uninstall"
}

check_fresh_lifecycle biscuit biscuit
check_fresh_lifecycle radar radar_puffin
check_fresh_lifecycle radar_puffin radar_puffin

legacy=$tmp/legacy/system
legacy_data=$tmp/legacy/data
mkdir -p "$legacy/bin" "$legacy/etc/airplayd" "$legacy/lib" "$legacy_data"
printf 'vendor ledcontroller\n' >"$legacy/bin/ledcontroller.airplayd-orig"
printf 'legacy airplayd\n' >"$legacy/bin/airplayd"
chmod 755 "$legacy/bin/ledcontroller.airplayd-orig" "$legacy/bin/airplayd"
ln -s airplayd "$legacy/bin/ledcontroller"
legacy_base=$(sha256sum "$legacy/bin/ledcontroller.airplayd-orig" | awk '{print $1}')
legacy_entry=$(sha256sum "$legacy/bin/airplayd" | awk '{print $1}')
cat >"$legacy/etc/airplayd/installed" <<EOF
name=airplayd
version=0.1.0
device=biscuit
base_ledcontroller_sha256=$legacy_base
airplayd_sha256=$legacy_entry
EOF
printf '%s\n' 'Migrated Echo' >"$legacy_data/AIRPLAY_NAME"
run_update "$install_script" "$install_zip" "$legacy" biscuit
[ -L "$legacy/bin/ledcontroller" ] && [ "$(readlink "$legacy/bin/ledcontroller")" = airplayd ] ||
  fail 'legacy migration did not retain service contract'
cmp "$tmp/install/payload/system/bin/airplayd" "$legacy/bin/airplayd" ||
  fail 'legacy entrypoint was not replaced'
[ ! -e "$legacy/etc/airplayd/installed" ] || fail 'legacy marker survived migration'
grep -qx 'owner=shairport-echo' "$legacy/etc/shairport-echo/installed" || fail 'migration marker missing'
run_update "$uninstall_script" "$uninstall_zip" "$legacy" biscuit
[ -f "$legacy/bin/ledcontroller" ] && [ ! -L "$legacy/bin/ledcontroller" ] ||
  fail 'legacy uninstall did not restore vendor fallback'
[ "$(cat "$legacy_data/AIRPLAY_NAME")" = 'Migrated Echo' ] || fail 'migration changed name state'

wrong=$tmp/wrong/system
mkdir -p "$wrong/bin" "$wrong/etc" "$wrong/lib"
printf 'vendor\n' >"$wrong/bin/ledcontroller"
chmod 755 "$wrong/bin/ledcontroller"
if run_update "$install_script" "$install_zip" "$wrong" unsupported; then
  fail 'unsupported device was accepted'
fi
[ ! -e "$wrong/bin/ledcontroller.airplayd-orig" ] || fail 'unsupported device was modified'

unsafe=$tmp/unsafe/system
mkdir -p "$unsafe/bin" "$unsafe/etc" "$unsafe/lib"
printf 'target\n' >"$unsafe/bin/other"
ln -s other "$unsafe/bin/ledcontroller"
if run_update "$install_script" "$install_zip" "$unsafe" biscuit; then
  fail 'pre-existing ledcontroller symlink was accepted as a fresh install'
fi
[ ! -e "$unsafe/bin/airplayd" ] || fail 'unsafe service state was modified'

"$root/tests/airplayd_entrypoint.sh"
printf '%s\n' 'TWRP ZIP structure, fresh install, upgrade, legacy migration and uninstall checks passed'

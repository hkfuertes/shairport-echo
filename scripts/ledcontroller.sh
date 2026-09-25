#!/system/bin/sh
# Replaces the vendor ledcontroller init entrypoint: generic shairport-sync + nqptp.
set -eu

root=${SHAIRPORT_ECHO_ROOT:-/system/lib/shairport-echo}
data=${SHAIRPORT_ECHO_DATA:-/data}
# Seeded from $root/shairport-sync.conf on first boot, then authoritative.
config=$data/shairport-sync.conf
log=$data/shairport-echo.log
# ponytail: Biscuit and Radar use this vendor endpoint; use echo-controls if that changes.
ring_path=${ECHO_RING_PATH:-/sys/bus/i2c/devices/0-003f}
getprop_cmd=${GETPROP:-/system/bin/getprop}
ip_cmd=${IP:-ip}
mount_cmd=${MOUNT:-mount}
umount_cmd=${UMOUNT:-umount}
sleep_cmd=${SLEEP:-sleep}
nqptp_pid=
shairport_pid=
volume_control_pid=
made_shm=0

fail() {
  echo "ledcontroller: $*" >&2
  exit 1
}

default_name() {
  name=$($getprop_cmd ro.product.name 2>/dev/null || true)
  # ponytail: product names are plain identifiers; anything else would need libconfig/sed escaping.
  case "$name" in
    ''|*[!A-Za-z0-9_.\ -]*) name=Echo ;;
  esac
  printf '%s\n' "$name"
}

seed_config() {
  [ -e "$config" ] || {
    sed "s/@NAME@/$(default_name)/" "$root/shairport-sync.conf" >"$config.$$" &&
      mv -f "$config.$$" "$config"
  } || fail "cannot seed $config"
  [ -f "$config" ] && [ ! -L "$config" ] || fail "$config is not a regular file"
}

turn_ring_off() {
  [ -e "$ring_path/boot_animation" ] &&
    printf '0\n' >"$ring_path/boot_animation" 2>/dev/null || true
  [ -e "$ring_path/frame" ] &&
    printf '%072d\n' 0 >"$ring_path/frame" 2>/dev/null || true
}

stop_child() {
  pid=${1:-}
  [ -n "$pid" ] || return 0
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    "$sleep_cmd" 1
    kill -9 "$pid" 2>/dev/null || true
  fi
  wait "$pid" 2>/dev/null || true
}

cleanup() {
  trap - EXIT HUP INT TERM
  "$root/echo-volume-control" --amp-off >/dev/null 2>&1 || true
  stop_child "$shairport_pid"
  stop_child "$nqptp_pid"
  stop_child "$volume_control_pid"
  if [ "$made_shm" -eq 1 ]; then
    "$umount_cmd" /dev/shm 2>/dev/null || true
    rmdir /dev/shm 2>/dev/null || true
  fi
}

turn_ring_off
seed_config
if [ "${SHAIRPORT_ECHO_DRY_RUN:-0}" = 1 ]; then
  printf 'config=%s\n' "$config"
  exit 0
fi

trap cleanup EXIT
trap 'exit 0' HUP INT TERM

[ -x "$root/nqptp" ] || fail "missing $root/nqptp"
[ -x "$root/shairport-sync" ] || fail "missing $root/shairport-sync"
[ -x "$root/echo-volume-control" ] || fail "missing $root/echo-volume-control"

# No hwrng and no saved entropy: without this, getrandom() in the crypto libs
# blocks for minutes after boot. Blocks once on the very first boot.
"$root/entropy-seed" "$data/shairport-echo.seed" ||
  echo 'ledcontroller: entropy seed failed; startup may be slow' >&2

while ! "$ip_cmd" -4 addr show dev wlan0 2>/dev/null | grep -q 'inet '; do
  "$sleep_cmd" 1
done

if [ -e /dev/shm ]; then
  [ -d /dev/shm ] || fail '/dev/shm is not a directory'
else
  mkdir /dev/shm || fail 'could not create /dev/shm'
  if ! "$mount_cmd" -t tmpfs -o mode=1777,size=1m tmpfs /dev/shm; then
    rmdir /dev/shm 2>/dev/null || true
    fail 'could not mount /dev/shm'
  fi
  made_shm=1
fi

: >"$log"
"$root/echo-volume-control" --no-volume-buttons --sync-with-mic-mute --metadata-port 45678 >>"$log" 2>&1 &
volume_control_pid=$!
"$sleep_cmd" 1
if ! kill -0 "$volume_control_pid" 2>/dev/null; then
  cat "$log" >&2 || true
  fail 'echo-volume-control exited during startup'
fi

"$root/nqptp" >>"$log" 2>&1 &
nqptp_pid=$!
"$sleep_cmd" 1
if ! kill -0 "$nqptp_pid" 2>/dev/null; then
  cat "$log" >&2 || true
  fail 'nqptp exited during startup'
fi

# echo-alsa.conf only matters if the config selects output_backend = "alsa".
ALSA_CONFIG_PATH="$root/echo-alsa.conf" \
  "$root/shairport-sync" -c "$config" >>"$log" 2>&1 &
shairport_pid=$!
"$sleep_cmd" 1
if ! kill -0 "$shairport_pid" 2>/dev/null; then
  cat "$log" >&2 || true
  fail 'shairport-sync exited during startup'
fi

while kill -0 "$volume_control_pid" 2>/dev/null && kill -0 "$nqptp_pid" 2>/dev/null && kill -0 "$shairport_pid" 2>/dev/null; do
  "$sleep_cmd" 1
done
fail 'receiver child exited'

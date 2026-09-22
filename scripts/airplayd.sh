#!/system/bin/sh
# Init entrypoint installed as /system/bin/airplayd; ledcontroller points here.
set -eu

root=${SHAIRPORT_ECHO_ROOT:-/system/lib/shairport-echo}
name_file=${AIRPLAY_NAME_PATH:-/data/AIRPLAY_NAME}
state_dir=${SHAIRPORT_ECHO_STATE_DIR:-/data/shairport-echo}
getprop_cmd=${GETPROP:-/system/bin/getprop}
ip_cmd=${IP:-ip}
mount_cmd=${MOUNT:-mount}
umount_cmd=${UMOUNT:-umount}
sleep_cmd=${SLEEP:-sleep}
nqptp_pid=
shairport_pid=
made_shm=0

fail() {
  echo "airplayd: $*" >&2
  exit 1
}

read_name() {
  if [ -e "$name_file" ]; then
    [ -f "$name_file" ] || fail "$name_file is not a regular file"
  else
    name=$($getprop_cmd ro.product.name) || fail 'could not read ro.product.name'
    [ -n "$name" ] || fail 'ro.product.name is empty'
    umask 022
    printf '%s\n' "$name" >"$name_file" || fail "could not create $name_file"
  fi

  name=$(cat "$name_file") || fail "could not read $name_file"
  [ -n "$name" ] || fail "$name_file is empty"
  printf '%s\n' "$name"
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
  stop_child "$shairport_pid"
  stop_child "$nqptp_pid"
  if [ "$made_shm" -eq 1 ]; then
    "$umount_cmd" /dev/shm 2>/dev/null || true
    rmdir /dev/shm 2>/dev/null || true
  fi
}
trap cleanup EXIT
trap 'exit 0' HUP INT TERM

name=$(read_name)
if [ "${AIRPLAYD_DRY_RUN:-0}" = 1 ]; then
  printf 'name=%s\n' "$name"
  exit 0
fi

[ -x "$root/nqptp" ] || fail "missing $root/nqptp"
[ -x "$root/shairport-sync" ] || fail "missing $root/shairport-sync"
[ -f "$root/echo-alsa.conf" ] || fail "missing $root/echo-alsa.conf"
[ -f "$root/echo-shairport-sync.conf" ] || fail "missing $root/echo-shairport-sync.conf"

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

mkdir -p "$state_dir"
"$root/nqptp" >"$state_dir/nqptp.log" 2>&1 &
nqptp_pid=$!
"$sleep_cmd" 1
if ! kill -0 "$nqptp_pid" 2>/dev/null; then
  cat "$state_dir/nqptp.log" >&2 || true
  fail 'nqptp exited during startup'
fi

ALSA_CONFIG_PATH="$root/echo-alsa.conf" \
  "$root/shairport-sync" -a "$name" -c "$root/echo-shairport-sync.conf" \
  >"$state_dir/shairport-sync.log" 2>&1 &
shairport_pid=$!
"$sleep_cmd" 1
if ! kill -0 "$shairport_pid" 2>/dev/null; then
  cat "$state_dir/shairport-sync.log" >&2 || true
  fail 'shairport-sync exited during startup'
fi

while kill -0 "$nqptp_pid" 2>/dev/null && kill -0 "$shairport_pid" 2>/dev/null; do
  "$sleep_cmd" 1
done
fail 'receiver child exited'

#!/system/bin/sh
# Starts NQPTP briefly to verify Android shared memory and PTP port binding.
# It never opens PCM or enables the speaker route.
set -eu

root=${1:-$(dirname "$0")}
binary=$root/nqptp
log=$root/nqptp-smoke.log
made_shm=0
pid=

cleanup() {
  set +e
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    sleep 1
    kill -9 "$pid" 2>/dev/null || true
  fi
  if [ "$made_shm" = 1 ]; then
    umount /dev/shm 2>/dev/null || true
    rmdir /dev/shm 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

[ -x "$binary" ] || { echo "missing executable: $binary" >&2; exit 64; }
if [ -e /dev/shm ]; then
  [ -d /dev/shm ] || { echo "/dev/shm is not a directory" >&2; exit 70; }
else
  mkdir /dev/shm
  mount -t tmpfs -o mode=1777,size=1m tmpfs /dev/shm
  made_shm=1
fi

: >"$log"
"$binary" >"$log" 2>&1 &
pid=$!
sleep 2
if ! kill -0 "$pid" 2>/dev/null; then
  cat "$log" >&2
  exit 1
fi
test -f /dev/shm/nqptp

echo "nqptp smoke passed (pid $pid)"
kill "$pid"
wait "$pid" || true
pid=

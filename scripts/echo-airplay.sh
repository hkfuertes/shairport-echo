#!/system/bin/sh
# Legacy manual A/B launcher only; the installed receiver uses ledcontroller instead.
# It does not start speakerd or apply the unique mDNS hostname.
set -eu

root=${SHAIRPORT_ECHO_ROOT:-$(dirname "$0")}
shairport_pid=$root/shairport-sync.pid
nqptp_pid=$root/nqptp.pid
made_shm=$root/.made-shm

alive() {
  [ -s "$1" ] && kill -0 "$(cat "$1")" 2>/dev/null
}

stop_pid() {
  pidfile=$1
  if alive "$pidfile"; then
    kill "$(cat "$pidfile")" 2>/dev/null || true
    sleep 1
    kill -9 "$(cat "$pidfile")" 2>/dev/null || true
  fi
  rm -f "$pidfile"
}

stop_all() {
  stop_pid "$shairport_pid"
  stop_pid "$nqptp_pid"
  if [ -f "$made_shm" ]; then
    umount /dev/shm 2>/dev/null || true
    rmdir /dev/shm 2>/dev/null || true
    rm -f "$made_shm"
  fi
}

case ${1:-status} in
  start|verbose|nosync|crate)
    [ -x "$root/nqptp" ] && [ -x "$root/shairport-sync" ] || {
      echo "missing nqptp or shairport-sync in $root" >&2
      exit 64
    }
    if alive "$nqptp_pid" || alive "$shairport_pid"; then
      echo "already running; use stop first" >&2
      exit 1
    fi
    rm -f "$nqptp_pid" "$shairport_pid"
    if [ -e /dev/shm ]; then
      [ -d /dev/shm ] || { echo "/dev/shm is not a directory" >&2; exit 70; }
    else
      mkdir /dev/shm
      mount -t tmpfs -o mode=1777,size=1m tmpfs /dev/shm
      : >"$made_shm"
    fi
    setsid "$root/nqptp" >"$root/nqptp.log" 2>&1 &
    echo $! >"$nqptp_pid"
    sleep 1
    if ! alive "$nqptp_pid"; then
      cat "$root/nqptp.log" >&2
      stop_all
      exit 1
    fi
    config=$root/echo-shairport-sync.conf
    case "$1" in
      verbose) set -- -vv ;;
      nosync)
        config=$root/echo-shairport-sync-nosync.conf
        set -- -vv
        ;;
      crate)
        config=$root/echo-shairport-sync-echo.conf
        set -- -vv
        ;;
      *) set -- ;;
    esac
    ALSA_CONFIG_PATH="$root/echo-alsa.conf" \
      setsid "$root/shairport-sync" "$@" -c "$config" >"$root/shairport-sync.log" 2>&1 &
    echo $! >"$shairport_pid"
    sleep 2
    if ! alive "$shairport_pid"; then
      cat "$root/shairport-sync.log" >&2
      stop_all
      exit 1
    fi
    echo "started nqptp $(cat "$nqptp_pid"), shairport-sync $(cat "$shairport_pid")"
    ;;
  stop)
    stop_all
    echo "stopped"
    ;;
  status)
    alive "$nqptp_pid" && echo "nqptp $(cat "$nqptp_pid")" || echo "nqptp stopped"
    alive "$shairport_pid" && echo "shairport-sync $(cat "$shairport_pid")" || echo "shairport-sync stopped"
    ;;
  *)
    echo "usage: $0 {start|verbose|nosync|crate|stop|status}" >&2
    exit 64
    ;;
esac

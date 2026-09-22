#!/bin/sh
# Passive host-side capture for attended AirPlay connection tests.
set -eu

serial=${1:?"usage: $0 ADB_SERIAL [output-directory]"}
out=${2:-"/tmp/shairport-echo-monitor-$(date +%Y%m%d-%H%M%S)"}
root=/data/local/tmp/shairport-echo
interval=${MONITOR_INTERVAL_SECONDS:-1}

mkdir -p "$out"
printf 'serial=%s\nstarted=%s\n' "$serial" "$(date -Is)" > "$out/metadata"
adb -s "$serial" shell "tail -n 200 '$root/shairport-sync.log'" > "$out/initial.log" 2>&1 || true

snapshot() {
  while :; do
    {
      printf '== %s ==\n' "$(date -Is)"
      adb -s "$serial" shell "r='$root'; \"\$r/echo-airplay\" status 2>&1 || true; printf 'pcm: '; cat /proc/asound/card0/pcm23p/sub0/status 2>/dev/null | head -1 || true; tinymix 2>/dev/null | grep -E 'Ext_Speaker_Amp_Switch|Right Channel Only|HP Driver Gain Volume|PCM Playback Volume' || true"
    } >> "$out/state.log" 2>&1 || true
    sleep "$interval"
  done
}

snapshot &
snapshot_pid=$!
adb -s "$serial" shell "tail -n 0 -f '$root/shairport-sync.log'" |
  while IFS= read -r line; do
    printf '%s %s\n' "$(date -Is)" "$line"
  done > "$out/shairport.log" &
tail_pid=$!

cleanup() {
  kill "$snapshot_pid" "$tail_pid" 2>/dev/null || true
  wait "$snapshot_pid" "$tail_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

printf 'monitoring %s; Ctrl-C stops it\n' "$out" >&2
wait "$tail_pid"

#!/bin/sh
# Host-only check that the A/B launcher selects each alternate config.
set -eu

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT HUP INT TERM

cp scripts/echo-airplay.sh "$root/echo-airplay"
printf '%s\n' '# placeholder' >"$root/echo-alsa.conf"
printf '%s\n' '# placeholder' >"$root/echo-shairport-sync.conf"
printf '%s\n' '# placeholder' >"$root/echo-shairport-sync-nosync.conf"
printf '%s\n' '# placeholder' >"$root/echo-shairport-sync-echo.conf"

cat >"$root/nqptp" <<'EOF'
#!/bin/sh
exec sleep 30
EOF
cat >"$root/shairport-sync" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" >"$TEST_ROOT/shairport.args"
exec sleep 30
EOF
chmod +x "$root/nqptp" "$root/shairport-sync"

check_mode() {
  mode=$1
  config=$2
  TEST_ROOT=$root SHAIRPORT_ECHO_ROOT=$root sh "$root/echo-airplay" "$mode" >/dev/null
  TEST_ROOT=$root SHAIRPORT_ECHO_ROOT=$root sh "$root/echo-airplay" stop >/dev/null
  grep -qx -- '-vv' "$root/shairport.args"
  grep -qx -- "$root/$config" "$root/shairport.args"
}

check_mode nosync echo-shairport-sync-nosync.conf
check_mode crate echo-shairport-sync-echo.conf
printf '%s\n' 'ok: A/B modes select their configs'

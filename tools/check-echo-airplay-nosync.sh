#!/bin/sh
# Host-only check that the diagnostic launcher selects the no-sync config.
set -eu

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT HUP INT TERM

cp scripts/echo-airplay.sh "$root/echo-airplay"
printf '%s\n' '# placeholder' >"$root/echo-alsa.conf"
printf '%s\n' '# placeholder' >"$root/echo-shairport-sync.conf"
printf '%s\n' '# placeholder' >"$root/echo-shairport-sync-nosync.conf"

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

TEST_ROOT=$root SHAIRPORT_ECHO_ROOT=$root sh "$root/echo-airplay" nosync >/dev/null
TEST_ROOT=$root SHAIRPORT_ECHO_ROOT=$root sh "$root/echo-airplay" stop >/dev/null

grep -qx -- '-vv' "$root/shairport.args"
grep -qx -- "$root/echo-shairport-sync-nosync.conf" "$root/shairport.args"
printf '%s\n' 'ok: nosync selects the diagnostic config'

#!/bin/sh
# Host-only config-seeding and boot-LED check for the ledcontroller entrypoint.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

fail() {
  echo "ledcontroller test: $*" >&2
  exit 1
}

cat >"$tmp/getprop" <<'EOF'
#!/bin/sh
[ "$1" = ro.product.name ] || exit 1
printf '%s\n' "$TEST_PRODUCT"
EOF
chmod 755 "$tmp/getprop"

ring=$tmp/ring
mkdir "$ring"
printf '1\n' >"$ring/boot_animation"
printf 'spinning\n' >"$ring/frame"
config=$tmp/shairport-sync.conf

run() {
  TEST_PRODUCT=$1 \
  SHAIRPORT_ECHO_DRY_RUN=1 \
  SHAIRPORT_ECHO_ROOT="$root/config" \
  SHAIRPORT_ECHO_DATA="$tmp" \
  ECHO_RING_PATH="$ring" \
  GETPROP="$tmp/getprop" \
  sh "$root/scripts/ledcontroller.sh"
}

[ "$(run biscuit_minimal)" = "config=$config" ] || fail 'dry run did not report the config'
grep -Fqx '  name = "biscuit_minimal";' "$config" || fail 'name was not seeded from ro.product.name'
grep -Fqx '  socket_port = 45678;' "$config" || fail 'metadata port was not seeded'
grep -Fqx '  include_cover_art = "no";' "$config" || fail 'metadata cover-art setting was not seeded'
grep -Fq '@NAME@' "$config" && fail 'placeholder survived seeding'
[ "$(cat "$ring/boot_animation")" = 0 ] || fail 'boot animation was not disabled'
[ "$(cat "$ring/frame")" = '000000000000000000000000000000000000000000000000000000000000000000000000' ] ||
  fail 'ring frame was not black'

printf '%s\n' '// user edit' >>"$config"
run ignored >/dev/null
grep -Fqx '// user edit' "$config" || fail 'user config was overwritten'
grep -Fqx '  name = "biscuit_minimal";' "$config" || fail 'seeded name changed'

rm "$config"
run 'bad"name' >/dev/null
grep -Fqx '  name = "Echo";' "$config" || fail 'unsafe product name was not replaced'

printf '%s\n' 'ok: ledcontroller seeds the config once and turns LEDs off'

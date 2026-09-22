#!/bin/sh
# Host-only name-state check for the init entrypoint.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

fail() {
  echo "airplayd entrypoint test: $*" >&2
  exit 1
}

cat >"$tmp/getprop" <<'EOF'
#!/bin/sh
[ "$1" = ro.product.name ] || exit 1
printf '%s\n' biscuit_minimal
EOF
chmod 755 "$tmp/getprop"

ring=$tmp/ring
mkdir "$ring"
printf '1\n' >"$ring/boot_animation"
printf 'spinning\n' >"$ring/frame"

run() {
  AIRPLAYD_DRY_RUN=1 \
  AIRPLAY_NAME_PATH="$tmp/AIRPLAY_NAME" \
  ECHO_RING_PATH="$ring" \
  GETPROP="$tmp/getprop" \
  sh "$root/scripts/airplayd.sh"
}

[ "$(run)" = 'name=biscuit_minimal' ] || fail 'first boot did not use ro.product.name'
[ "$(cat "$ring/boot_animation")" = 0 ] || fail 'boot animation was not disabled'
[ "$(cat "$ring/frame")" = '000000000000000000000000000000000000000000000000000000000000000000000000' ] ||
  fail 'ring frame was not black'
[ "$(cat "$tmp/AIRPLAY_NAME")" = biscuit_minimal ] || fail 'default name was not persisted'
printf '%s\n' 'Kitchen Echo' >"$tmp/AIRPLAY_NAME"
[ "$(run)" = 'name=Kitchen Echo' ] || fail 'persisted name was not authoritative'
: >"$tmp/AIRPLAY_NAME"
if run >/dev/null 2>&1; then
  fail 'empty persisted name was accepted'
fi

printf '%s\n' 'ok: entrypoint persists ro.product.name once and preserves overrides'

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

run() {
  AIRPLAYD_DRY_RUN=1 \
  AIRPLAY_NAME_PATH="$tmp/AIRPLAY_NAME" \
  GETPROP="$tmp/getprop" \
  sh "$root/scripts/airplayd.sh"
}

[ "$(run)" = 'name=biscuit_minimal' ] || fail 'first boot did not use ro.product.name'
[ "$(cat "$tmp/AIRPLAY_NAME")" = biscuit_minimal ] || fail 'default name was not persisted'
printf '%s\n' 'Kitchen Echo' >"$tmp/AIRPLAY_NAME"
[ "$(run)" = 'name=Kitchen Echo' ] || fail 'persisted name was not authoritative'
: >"$tmp/AIRPLAY_NAME"
if run >/dev/null 2>&1; then
  fail 'empty persisted name was accepted'
fi

printf '%s\n' 'ok: entrypoint persists ro.product.name once and preserves overrides'

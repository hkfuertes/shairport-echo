#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
version=${VERSION:-0.2.0}
image=${TWRP_IMAGE:-shairport-echo-twrp-artifact:local}
work=$root/target/twrp
out=$root/out

fail() {
  echo "twrp build: $*" >&2
  exit 1
}

make_zip() {
  stage=$1
  output=$2
  find "$stage" -exec touch -h -t 200001010000 {} +
  rm -f "$output"
  (
    cd "$stage"
    find . -type f -print | LC_ALL=C sort | zip -X -q "$output" -@
  )
}

render_update_binary() {
  mode=$1
  destination=$2
  sed \
    -e "s/@MODE@/$mode/g" \
    -e "s/@VERSION@/$version/g" \
    "$root/twrp/META-INF/com/google/android/update-binary.in" >"$destination"
  chmod 755 "$destination"
}

rm -rf "$work"
mkdir -p "$work" "$out"
docker build --target twrp-artifact -t "$image" "$root"
container=$(docker create "$image" /bin/true)
trap 'docker rm -f "$container" >/dev/null 2>&1 || true; rm -rf "$work"' EXIT HUP INT TERM
docker cp "$container:/payload" "$work/payload"
docker rm "$container" >/dev/null
container=

[ -z "$(find "$work/payload" -type l -print)" ] || fail 'artifact contains a symlink'
[ -x "$work/payload/system/bin/speakerd" ] || fail 'missing speakerd binary'
for file in nqptp nqptp-service shairport-sync entropy-seed; do
  [ -x "$work/payload/system/lib/shairport-echo/$file" ] || fail "missing $file"
done
[ -f "$work/payload/system/lib/shairport-echo/speakerd.ini" ] || fail 'missing speakerd.ini'

install=$work/install
mkdir -p "$install/META-INF/com/google/android"
cp -a "$work/payload" "$install/payload"
render_update_binary install "$install/META-INF/com/google/android/update-binary"
cp "$root/twrp/META-INF/com/google/android/updater-script" \
  "$install/META-INF/com/google/android/updater-script"
make_zip "$install" "$out/shairport-echo-$version-armv7.zip"

uninstall=$work/uninstall
mkdir -p "$uninstall/META-INF/com/google/android"
render_update_binary uninstall "$uninstall/META-INF/com/google/android/update-binary"
cp "$root/twrp/META-INF/com/google/android/updater-script" \
  "$uninstall/META-INF/com/google/android/updater-script"
make_zip "$uninstall" "$out/shairport-echo-uninstall-$version.zip"

printf '%s\n' "$out/shairport-echo-$version-armv7.zip" "$out/shairport-echo-uninstall-$version.zip"

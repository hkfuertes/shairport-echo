#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
version=${VERSION:-0.1.0}
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
  rm -f "$output" "$output.sha256"
  (
    cd "$stage"
    find . -type f -print | LC_ALL=C sort | zip -X -q "$output" -@
  )
  (
    cd "$(dirname "$output")"
    sha256sum "$(basename "$output")" >"$(basename "$output").sha256"
  )
}

render_update_binary() {
  mode=$1
  destination=$2
  sed \
    -e "s/@MODE@/$mode/g" \
    -e "s/@VERSION@/$version/g" \
    -e "s/@MANIFEST_SHA256@/$manifest_sha256/g" \
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
[ -x "$work/payload/system/bin/airplayd" ] || fail 'missing airplayd entrypoint'
for file in nqptp shairport-sync; do
  [ -x "$work/payload/system/lib/shairport-echo/$file" ] || fail "missing $file"
done

install=$work/install
mkdir -p "$install/META-INF/com/google/android"
cp -a "$work/payload" "$install/payload"
(
  cd "$install"
  find payload -type f -print | LC_ALL=C sort | while IFS= read -r file; do sha256sum "$file"; done
) >"$install/payload-manifest.sha256"
manifest_sha256=$(sha256sum "$install/payload-manifest.sha256" | awk '{print $1}')
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

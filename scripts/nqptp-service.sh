#!/system/bin/sh
set -eu

R=/system/lib/shairport-echo
"$R/entropy-seed" /data/shairport-echo.seed || true

while ! ip -4 addr show dev wlan0 2>/dev/null | grep -q 'inet '; do
  sleep 1
done

mac=$(tr -cd '[:xdigit:]' </sys/class/net/wlan0/address)
[ "${#mac}" -eq 12 ] || exit 1
hostname "shairport-$mac" || exit 1

if [ -e /dev/shm ]; then
  [ -d /dev/shm ] || exit 1
else
  mkdir /dev/shm
  mount -t tmpfs -o mode=1777,size=1m tmpfs /dev/shm
fi

exec "$R/nqptp"

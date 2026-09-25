#!/system/bin/sh
# Legacy upstream-ALSA experiment only. Never run `on` with packaged ledcontroller:
# it writes the amplifier directly and bypasses the physical-privacy kill switch.
set -eu

mixer=/system/bin/tinymix
eq='128 0 1 0 0 0 0 0 0 0 0 0 0 0 0
128 0 1 0 0 0 0 0 0 0 0 0 0 0 0
128 0 1 0 0 0 0 0 0 0 0 0 0 0 0
128 0 1 0 0 0 0 0 0 0 0 0 0 0 0
128 0 1 0 0 0 0 0 0 0 0 0 0 0 0
128 0 1 0 0 0 0 0 0 0 0 0 0 0 0
127 247 0 0 128 9 0 0 127 239 0 0 0 17 0
0 0 17 0 0 127 222 0 0 15 0 0'

off() {
  "$mixer" "Ext_Speaker_Amp_Switch" Off
  "$mixer" "Audio_DacMux_Setting" On
  "$mixer" "Ignore Ramp Up" Off
  "$mixer" "Right Channel Only" Off
  "$mixer" "HP Driver Gain Volume" 0 0
  # shellcheck disable=SC2086
  "$mixer" "biquad coefficients" $eq
}

case ${1:-} in
  on)
    off
    "$mixer" "Audio_DacMux_Setting" Off
    "$mixer" "Right Channel Only" On
    "$mixer" "HP Driver Gain Volume" 10 10
    sleep 1.1
    "$mixer" "Ext_Speaker_Amp_Switch" On
    ;;
  off) off ;;
  status)
    for control in "Ext_Speaker_Amp_Switch" "Audio_DacMux_Setting" "Ignore Ramp Up" \
      "Right Channel Only" "HP Driver Gain Volume" "biquad coefficients"; do
      "$mixer" "$control"
    done
    ;;
  *)
    echo "usage: $0 {on|off|status}" >&2
    exit 64
    ;;
esac

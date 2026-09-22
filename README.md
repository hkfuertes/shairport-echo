# shairport-echo

Echo hardware layer and vendored upstream base for an Android ARMv7 port of Shairport Sync.

- `libs/echo-alsa`: PCM 0:23, mixer, amplifier/mute lifecycle, preflight and ABI probe.
- `libs/echo-controls`: LED ring and input controls, preflight and ABI probe.

See [PORTING_HANDOFF.md](PORTING_HANDOFF.md) and [third_party/README.md](third_party/README.md). The upstream sources are vendored; no Echo C adapter has been added yet.

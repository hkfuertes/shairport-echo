# shairport-echo

Echo hardware layer for a future Android ARMv7 port of Shairport Sync.

- `libs/echo-alsa`: PCM 0:23, mixer, amplifier/mute lifecycle, preflight and ABI probe.
- `libs/echo-controls`: LED ring and input controls, preflight and ABI probe.

See [PORTING_HANDOFF.md](PORTING_HANDOFF.md). No Shairport Sync code or C adapter has been added yet.

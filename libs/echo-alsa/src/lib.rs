#![deny(unsafe_op_in_unsafe_fn)]

mod alsa;
mod ffi;
#[cfg(feature = "shairplay")]
mod sink;

pub use alsa::{
    CARD, CHANNELS, ControlInfo, ControlType, DEVICE, MasterVolume, MixerEvent, PERIOD_FRAMES,
    PERIODS, PcmConfig, PreflightReport, REQUIRED_CONTROLS, SAMPLE_RATE, SystemMixer, preflight,
};
#[cfg(feature = "shairplay")]
pub use sink::{EchoAlsaSink, EchoAudioHandler, Telemetry};

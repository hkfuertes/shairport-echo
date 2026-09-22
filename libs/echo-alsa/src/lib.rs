#![deny(unsafe_op_in_unsafe_fn)]

mod alsa;
mod sink;

pub use alsa::{
    CARD, CHANNELS, ControlInfo, ControlType, DEVICE, MasterVolume, PERIOD_FRAMES, PERIODS,
    PcmConfig, PreflightReport, REQUIRED_CONTROLS, SAMPLE_RATE, preflight,
};
pub use sink::{EchoAlsaSink, EchoAudioHandler, Telemetry};

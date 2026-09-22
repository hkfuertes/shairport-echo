use crate::alsa::{CHANNELS, MasterVolume, Mixer, Pcm, PcmConfig, is_xrun};
use std::{
    ffi::c_int,
    io,
    mem::MaybeUninit,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::Mutex,
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EchoAlsaConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub period_frames: u32,
    pub periods: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EchoAlsaStats {
    pub raw_measurement_time_ns: u64,
    pub corrected_measurement_time_ns: u64,
    pub frames_sent_to_dac: u64,
    pub delay_frames: i32,
    pub reserved: u32,
}

pub struct EchoAlsaHandle {
    device: Mutex<EchoAlsaDevice>,
}

struct EchoAlsaDevice {
    pcm: Pcm,
    mixer: Mixer,
    volume_db: f32,
    muted: bool,
    started: bool,
    frames_sent_to_dac: u64,
    stats_discontinuity: bool,
}

impl EchoAlsaDevice {
    fn open() -> io::Result<Self> {
        let pcm = Pcm::open_default()?;
        let mixer = Mixer::open()?;
        let volume_db = current_volume_db(mixer.volume()?);
        Ok(Self {
            pcm,
            mixer,
            volume_db,
            muted: false,
            started: false,
            frames_sent_to_dac: 0,
            stats_discontinuity: true,
        })
    }

    fn config(&self) -> EchoAlsaConfig {
        let PcmConfig {
            sample_rate,
            channels,
            period_frames,
            periods,
        } = self.pcm.config();
        EchoAlsaConfig {
            sample_rate,
            channels,
            period_frames,
            periods,
        }
    }

    fn start(&mut self) -> io::Result<()> {
        self.pcm.prepare()?;
        if let Err(error) = self
            .mixer
            .configure_output()
            .and_then(|_| self.apply_volume())
        {
            let _ = self.mixer.disable_amp();
            return Err(error);
        }
        self.started = true;
        self.stats_discontinuity = true;
        Ok(())
    }

    fn stop(&mut self) -> io::Result<()> {
        let pcm_result = if self.started {
            self.pcm.drop_stream()
        } else {
            Ok(())
        };
        let mixer_result = self.mixer.disable_amp();
        self.started = false;
        self.stats_discontinuity = true;
        pcm_result.and(mixer_result)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.pcm.drop_stream()?;
        self.pcm.prepare()?;
        self.stats_discontinuity = true;
        Ok(())
    }

    fn write(&mut self, samples: &[i16]) -> io::Result<u32> {
        if !self.started {
            return Err(io::Error::from_raw_os_error(libc::ENOTCONN));
        }
        match self.pcm.write_frames(samples) {
            Ok(frames) => {
                self.frames_sent_to_dac += u64::from(frames);
                Ok(frames)
            }
            Err(error) => {
                if is_xrun(&error) {
                    self.stats_discontinuity = true;
                }
                Err(error)
            }
        }
    }

    fn delay_frames(&self) -> io::Result<i32> {
        self.pcm.delay_frames()
    }

    fn stats(&mut self) -> io::Result<(EchoAlsaStats, bool)> {
        let delay_frames = self.delay_frames()?;
        if delay_frames < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ALSA returned a negative playback delay",
            ));
        }
        let measurement_time_ns = monotonic_time_ns()?;
        let stats = EchoAlsaStats {
            raw_measurement_time_ns: measurement_time_ns,
            corrected_measurement_time_ns: measurement_time_ns,
            frames_sent_to_dac: self.frames_sent_to_dac,
            delay_frames,
            reserved: 0,
        };
        let discontinuity = std::mem::replace(&mut self.stats_discontinuity, false);
        Ok((stats, discontinuity))
    }

    fn set_volume_db(&mut self, volume_db: f64) -> io::Result<()> {
        if !volume_db.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "non-finite volume",
            ));
        }
        let volume_db = volume_db as f32;
        if !volume_db.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "volume is outside the supported range",
            ));
        }
        self.volume_db = volume_db;
        if self.started && !self.muted {
            self.mixer.set_db(self.volume_db)?;
        }
        Ok(())
    }

    fn set_mute(&mut self, muted: bool) -> io::Result<()> {
        self.muted = muted;
        if self.started {
            self.apply_volume()?;
        }
        Ok(())
    }

    fn volume_db(&self) -> io::Result<f64> {
        Ok(f64::from(current_volume_db(self.mixer.volume()?)))
    }

    fn adjust_volume_db(&mut self, steps: c_int) -> io::Result<f64> {
        let current = self.mixer.volume()?;
        let target = stepped_volume_db(current, steps);
        self.volume_db = target;
        if !self.muted {
            self.mixer.set_db(target)?;
        }
        self.volume_db()
    }

    fn apply_volume(&mut self) -> io::Result<()> {
        self.mixer
            .set_db(if self.muted { -144.0 } else { self.volume_db })
    }
}

fn current_volume_db(volume: MasterVolume) -> f32 {
    volume
        .gain_db
        .into_iter()
        .zip(volume.enabled)
        .filter_map(|(db, enabled)| enabled.then_some(db))
        .fold(-144.0, f32::max)
}

fn stepped_volume_db(volume: MasterVolume, steps: c_int) -> f32 {
    let current = current_volume_db(volume).clamp(-30.0, 0.0);
    let next = (current + steps.clamp(-30, 30) as f32).clamp(-30.0, 0.0);
    if next <= -30.0 { -144.0 } else { next }
}

fn read_system_volume_db() -> io::Result<f64> {
    Ok(f64::from(current_volume_db(Mixer::open()?.volume()?)))
}

fn adjust_system_volume_db(steps: c_int) -> io::Result<f64> {
    let mut mixer = Mixer::open()?;
    mixer.set_db(stepped_volume_db(mixer.volume()?, steps))?;
    Ok(f64::from(current_volume_db(mixer.volume()?)))
}

fn monotonic_time_ns() -> io::Result<u64> {
    let mut timestamp = MaybeUninit::<libc::timespec>::zeroed();
    // SAFETY: timestamp points to writable storage for a timespec.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, timestamp.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: clock_gettime succeeded and initialized timestamp.
    let timestamp = unsafe { timestamp.assume_init() };
    let seconds = u64::try_from(timestamp.tv_sec).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "monotonic clock returned a negative second count",
        )
    })?;
    let nanoseconds = u64::try_from(timestamp.tv_nsec).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "monotonic clock returned a negative nanosecond count",
        )
    })?;
    if nanoseconds >= 1_000_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "monotonic clock returned an invalid nanosecond count",
        ));
    }
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "monotonic clock overflow"))
}

fn ffi_status(operation: impl FnOnce() -> io::Result<()>) -> c_int {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => error_status(&error),
        Err(_) => -libc::EFAULT,
    }
}

fn error_status(error: &io::Error) -> c_int {
    let errno = error.raw_os_error().unwrap_or_else(|| match error.kind() {
        io::ErrorKind::InvalidInput => libc::EINVAL,
        io::ErrorKind::Unsupported => libc::ENOTSUP,
        _ => libc::EIO,
    });
    -errno
}

fn invalid_argument() -> io::Error {
    io::Error::from_raw_os_error(libc::EINVAL)
}

fn with_device(
    handle: *mut EchoAlsaHandle,
    operation: impl FnOnce(&mut EchoAlsaDevice) -> io::Result<()>,
) -> c_int {
    ffi_status(|| {
        if handle.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: callers may only pass handles returned by echo_alsa_open that have not closed.
        let handle = unsafe { &*handle };
        let mut device = handle
            .device
            .lock()
            .map_err(|_| io::Error::other("Echo ALSA handle lock poisoned"))?;
        operation(&mut device)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_open(out: *mut *mut EchoAlsaHandle) -> c_int {
    ffi_status(|| {
        if out.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = ptr::null_mut() };
        let device = EchoAlsaDevice::open()?;
        let handle = Box::new(EchoAlsaHandle {
            device: Mutex::new(device),
        });
        // SAFETY: out remains valid for this call and receives the owned opaque handle.
        unsafe { *out = Box::into_raw(handle) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_close(handle: *mut EchoAlsaHandle) -> c_int {
    ffi_status(|| {
        if handle.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: the C caller transfers exactly one live handle back to this function.
        let handle = unsafe { Box::from_raw(handle) };
        let mut device = handle
            .device
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        device.stop()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_get_config(
    handle: *mut EchoAlsaHandle,
    out: *mut EchoAlsaConfig,
) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    with_device(handle, |device| {
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = device.config() };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_start(handle: *mut EchoAlsaHandle) -> c_int {
    with_device(handle, EchoAlsaDevice::start)
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_stop(handle: *mut EchoAlsaHandle) -> c_int {
    with_device(handle, EchoAlsaDevice::stop)
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_flush(handle: *mut EchoAlsaHandle) -> c_int {
    with_device(handle, EchoAlsaDevice::flush)
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_write_s16le(
    handle: *mut EchoAlsaHandle,
    samples: *const i16,
    frames: u32,
    frames_written: *mut u32,
) -> c_int {
    if frames_written.is_null() || (frames != 0 && samples.is_null()) {
        return -libc::EINVAL;
    }
    // SAFETY: frames_written was checked for null and belongs to the C caller.
    unsafe { *frames_written = 0 };
    let sample_count = match usize::try_from(frames)
        .ok()
        .and_then(|frames| frames.checked_mul(CHANNELS as usize))
    {
        Some(sample_count) => sample_count,
        None => return -libc::EINVAL,
    };
    with_device(handle, |device| {
        let samples = if sample_count == 0 {
            &[]
        } else {
            // SAFETY: C promises a valid interleaved S16_LE buffer for the supplied frame count.
            unsafe { std::slice::from_raw_parts(samples, sample_count) }
        };
        let written = device.write(samples)?;
        // SAFETY: frames_written was checked for null and belongs to the C caller.
        unsafe { *frames_written = written };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_delay_frames(handle: *mut EchoAlsaHandle, out: *mut i32) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    with_device(handle, |device| {
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = device.delay_frames()? };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_stats(handle: *mut EchoAlsaHandle, out: *mut EchoAlsaStats) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    with_device(handle, |device| {
        let (stats, discontinuity) = device.stats()?;
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = stats };
        if discontinuity {
            Err(io::Error::from_raw_os_error(libc::EPIPE))
        } else {
            Ok(())
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_set_volume_db(handle: *mut EchoAlsaHandle, volume_db: f64) -> c_int {
    with_device(handle, |device| device.set_volume_db(volume_db))
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_set_mute(handle: *mut EchoAlsaHandle, muted: c_int) -> c_int {
    with_device(handle, |device| device.set_mute(muted != 0))
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_get_volume_db(handle: *mut EchoAlsaHandle, out: *mut f64) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    with_device(handle, |device| {
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = device.volume_db()? };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_read_system_volume_db(out: *mut f64) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    ffi_status(|| {
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = read_system_volume_db()? };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_adjust_system_volume_db(steps: c_int, out: *mut f64) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    ffi_status(|| {
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = adjust_system_volume_db(steps)? };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_alsa_adjust_volume_db(
    handle: *mut EchoAlsaHandle,
    steps: c_int,
    out: *mut f64,
) -> c_int {
    if out.is_null() {
        return -libc::EINVAL;
    }
    with_device(handle, |device| {
        // SAFETY: out was checked for null and belongs to the C caller.
        unsafe { *out = device.adjust_volume_db(steps)? };
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn c_layouts_are_stable() {
        assert_eq!(size_of::<EchoAlsaConfig>(), 16);
        assert_eq!(size_of::<EchoAlsaStats>(), 32);
    }

    #[test]
    fn current_volume_uses_hardware_mute() {
        assert_eq!(
            current_volume_db(MasterVolume {
                gain_db: [-20.0, -18.0],
                enabled: [true, true],
            }),
            -18.0
        );
        assert_eq!(
            current_volume_db(MasterVolume {
                gain_db: [0.0, 0.0],
                enabled: [false, false],
            }),
            -144.0
        );
    }

    #[test]
    fn status_is_a_negative_errno() {
        assert_eq!(error_status(&invalid_argument()), -libc::EINVAL);
    }

    #[test]
    fn volume_steps_read_the_mixer_and_mute_at_the_bottom() {
        let volume = MasterVolume {
            gain_db: [-20.5, -20.0],
            enabled: [true, true],
        };
        assert_eq!(stepped_volume_db(volume, 1), -19.0);
        assert_eq!(stepped_volume_db(volume, -20), -144.0);
        assert_eq!(
            stepped_volume_db(
                MasterVolume {
                    gain_db: [0.0, 0.0],
                    enabled: [false, false],
                },
                1,
            ),
            -29.0
        );
    }
}

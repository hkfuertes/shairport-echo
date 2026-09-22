use std::{ffi::c_void, io, mem::size_of, os::fd::RawFd};

pub const CARD: u32 = 0;
pub const DEVICE: u32 = 23;
pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u32 = 2;
pub const PERIOD_FRAMES: u32 = 1_024;
pub const PERIODS: u32 = 4;

// ponytail: keep Off for Echo and test Biscuit before adding model-specific routing.
const RIGHT_CHANNEL_ONLY: bool = false;

pub const REQUIRED_CONTROLS: &[&str] = &[
    "Ext_Speaker_Amp_Switch",
    "Audio_DacMux_Setting",
    "Right Channel Only",
    "Ignore Ramp Up",
    "HP Driver Gain Volume",
    "PCM Playback Volume",
    "HP DAC Playback Switch",
    "MFP Gpio Mute",
    "biquad coefficients",
];

const PCM_PATH: &[u8] = b"/dev/snd/pcmC0D23p\0";
const CONTROL_PATH: &[u8] = b"/dev/snd/controlC0\0";

const HW_PARAMS_SIZE: usize = 604;
const SW_PARAMS_SIZE: usize = 104;
const XFERI_SIZE: usize = 12;
const MASK_SIZE: usize = 32;
const INTERVAL_SIZE: usize = 12;
const MASKS_OFFSET: usize = 4;
const INTERVALS_OFFSET: usize = 260;
const RMASK_OFFSET: usize = 512;
#[cfg(test)]
const FIFO_SIZE_OFFSET: usize = 536;
#[cfg(test)]
const RESERVED_OFFSET: usize = 540;
const XFERI_RESULT_OFFSET: usize = 0;
const XFERI_BUFFER_OFFSET: usize = 4;
const XFERI_FRAMES_OFFSET: usize = 8;

#[cfg(test)]
const ELEM_ID_SIZE: usize = 64;
const ELEM_LIST_SIZE: usize = 72;
const ELEM_INFO_SIZE: usize = 272;
const ELEM_VALUE_SIZE: usize = 712;
const ELEM_VALUE_DATA_OFFSET: usize = 72;
const ELEM_LIST_USED_OFFSET: usize = 8;
const ELEM_LIST_COUNT_OFFSET: usize = 12;
const ELEM_LIST_PIDS_OFFSET: usize = 16;
const ELEM_INFO_TYPE_OFFSET: usize = 64;
const ELEM_INFO_ACCESS_OFFSET: usize = 68;
const ELEM_INFO_COUNT_OFFSET: usize = 72;
const ELEM_INFO_ENUM_ITEMS_OFFSET: usize = 80;
const ELEM_INFO_ENUM_ITEM_OFFSET: usize = 84;
const ELEM_INFO_ENUM_NAME_OFFSET: usize = 88;
const ELEM_INFO_ENUM_NAME_LENGTH: usize = 64;
const ELEM_ID_NAME_OFFSET: usize = 16;
const ELEM_ID_NAME_LENGTH: usize = 44;

const MAX_CONTROLS: usize = 4_096;
const MAX_ENUM_ITEMS: u32 = 256;

const PCM_ACCESS_RW_INTERLEAVED: u32 = 3;
const PCM_FORMAT_S16_LE: u32 = 2;
const PCM_SUBFORMAT_STD: u32 = 0;

const HW_PARAM_ACCESS: usize = 0;
const HW_PARAM_FORMAT: usize = 1;
const HW_PARAM_SUBFORMAT: usize = 2;
const HW_PARAM_SAMPLE_BITS: usize = 8;
const HW_PARAM_FRAME_BITS: usize = 9;
const HW_PARAM_CHANNELS: usize = 10;
const HW_PARAM_RATE: usize = 11;
const HW_PARAM_PERIOD_SIZE: usize = 13;
const HW_PARAM_PERIODS: usize = 15;
const HW_PARAM_FIRST_MASK: usize = HW_PARAM_ACCESS;
const HW_PARAM_FIRST_INTERVAL: usize = HW_PARAM_SAMPLE_BITS;
const HW_PARAM_MASK_COUNT: usize = 3;
const HW_PARAM_INTERVAL_COUNT: usize = 12;

const CONTROL_TYPE_BOOLEAN: u32 = 1;
const CONTROL_TYPE_INTEGER: u32 = 2;
const CONTROL_TYPE_ENUMERATED: u32 = 3;
const CONTROL_TYPE_BYTES: u32 = 4;
const CONTROL_TYPE_INTEGER64: u32 = 5;

const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;

const fn ioc(direction: u32, kind: u8, number: u8, size: usize) -> u32 {
    (direction << 30) | ((size as u32) << 16) | ((kind as u32) << 8) | number as u32
}

const PCM_IOCTL_HW_PARAMS: u32 = ioc(IOC_READ | IOC_WRITE, b'A', 0x11, HW_PARAMS_SIZE);
const PCM_IOCTL_SW_PARAMS: u32 = ioc(IOC_READ | IOC_WRITE, b'A', 0x13, SW_PARAMS_SIZE);
const PCM_IOCTL_PREPARE: u32 = ioc(0, b'A', 0x40, 0);
const PCM_IOCTL_DELAY: u32 = ioc(IOC_READ, b'A', 0x21, size_of::<i32>());
const PCM_IOCTL_DROP: u32 = ioc(0, b'A', 0x43, 0);
const PCM_IOCTL_WRITEI_FRAMES: u32 = ioc(IOC_WRITE, b'A', 0x50, XFERI_SIZE);
const CTL_IOCTL_ELEM_LIST: u32 = ioc(IOC_READ | IOC_WRITE, b'U', 0x10, ELEM_LIST_SIZE);
const CTL_IOCTL_ELEM_INFO: u32 = ioc(IOC_READ | IOC_WRITE, b'U', 0x11, ELEM_INFO_SIZE);
const CTL_IOCTL_ELEM_READ: u32 = ioc(IOC_READ | IOC_WRITE, b'U', 0x12, ELEM_VALUE_SIZE);
const CTL_IOCTL_ELEM_WRITE: u32 = ioc(IOC_READ | IOC_WRITE, b'U', 0x13, ELEM_VALUE_SIZE);
const CTL_IOCTL_TLV_READ: u32 = ioc(IOC_READ | IOC_WRITE, b'U', 0x1a, 8);

type IoctlRequest = libc::Ioctl;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub period_frames: u32,
    pub periods: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlType {
    Boolean,
    Integer,
    Enumerated,
    Bytes,
    Integer64,
    Unknown(u32),
}

impl From<u32> for ControlType {
    fn from(value: u32) -> Self {
        match value {
            CONTROL_TYPE_BOOLEAN => Self::Boolean,
            CONTROL_TYPE_INTEGER => Self::Integer,
            CONTROL_TYPE_ENUMERATED => Self::Enumerated,
            CONTROL_TYPE_BYTES => Self::Bytes,
            CONTROL_TYPE_INTEGER64 => Self::Integer64,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlInfo {
    pub numid: u32,
    pub name: String,
    pub control_type: ControlType,
    pub count: u32,
    pub access: u32,
    pub enum_items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PreflightReport {
    pub pcm: PcmConfig,
    pub controls: Vec<ControlInfo>,
    pub missing_controls: Vec<String>,
}

pub fn preflight() -> io::Result<PreflightReport> {
    require_armv7()?;

    let controls = enumerate_controls()?;
    let missing_controls = REQUIRED_CONTROLS
        .iter()
        .filter(|required| !controls.iter().any(|control| control.name == **required))
        .map(|required| (*required).to_owned())
        .collect();

    let mut pcm = Pcm::open_default()?;
    let config = pcm.config();
    pcm.drop_stream()?;

    Ok(PreflightReport {
        pcm: config,
        controls,
        missing_controls,
    })
}

/// Current ALSA DAC gain and playback switch for each channel, read from hardware.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MasterVolume {
    pub gain_db: [f32; 2],
    pub enabled: [bool; 2],
}

// System master and speaker gates, never a per-stream software multiplier.
pub(crate) struct Mixer {
    fd: Fd,
    volume_id: u32,
    switch_id: u32,
    gpio_mute_id: u32,
    amp_id: u32,
    right_channel_id: u32,
}

impl Mixer {
    pub(crate) fn open() -> io::Result<Self> {
        require_armv7()?;
        let fd = Fd::open(CONTROL_PATH)?;
        let volume = mixer_info(fd.raw(), "PCM Playback Volume")?;
        let switch = mixer_info(fd.raw(), "HP DAC Playback Switch")?;
        for (info, kind, max) in [
            (&volume, ControlType::Integer, 175),
            (&switch, ControlType::Boolean, 1),
        ] {
            if info.control_type() != kind
                || info.count() != 2
                || info.access() & 3 != 3 // SNDRV_CTL_ELEM_ACCESS_READ | WRITE
                || info.get_u32(80) != 0 // integer.min (ARMv7 long)
                || info.get_u32(84) != max
            // integer.max
            {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("unexpected Echo mixer layout: {}", info.name()),
                ));
            }
        }

        // Verify the kernel's dB scale before mapping AirPlay dB to mixer steps.
        // snd_ctl_tlv has an 8-byte header followed by a variable-length payload.
        let mut tlv = [volume.numid(), 16, 0, 0, 0, 0];
        ioctl(fd.raw(), CTL_IOCTL_TLV_READ, tlv.as_mut_ptr().cast())?;
        if tlv[2..] != [1, 8, (-6350_i32) as u32, 50] {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PCM Playback Volume must use -63.5 dB + 0.5 dB/step",
            ));
        }
        Ok(Self {
            gpio_mute_id: enum_switch_id(fd.raw(), "MFP Gpio Mute")?,
            amp_id: enum_switch_id(fd.raw(), "Ext_Speaker_Amp_Switch")?,
            right_channel_id: enum_switch_id(fd.raw(), "Right Channel Only")?,
            fd,
            volume_id: volume.numid(),
            switch_id: switch.numid(),
        })
    }

    pub(crate) fn configure_output(&mut self) -> io::Result<()> {
        self.write(self.right_channel_id, &[u32::from(RIGHT_CHANNEL_ONLY)])?;
        self.write(self.amp_id, &[1])?;
        self.release_gpio_mute()
    }

    pub(crate) fn release_gpio_mute(&mut self) -> io::Result<()> {
        // The codec's hardware mute is separate from AirPlay's master-volume mute.
        self.write(self.gpio_mute_id, &[0])
    }

    pub(crate) fn disable_amp(&mut self) -> io::Result<()> {
        self.write(self.amp_id, &[0])
    }

    pub(crate) fn set_db(&mut self, db: f32) -> io::Result<()> {
        let (level, enabled) = master_volume_setting(db)?;
        if !enabled {
            self.write(self.switch_id, &[0; 2])?;
        }
        self.write(self.volume_id, &[level; 2])?;
        if enabled {
            self.write(self.switch_id, &[1; 2])?;
        }
        Ok(())
    }

    pub(crate) fn volume(&self) -> io::Result<MasterVolume> {
        let levels = self.read_stereo(self.volume_id)?;
        let switches = self.read_stereo(self.switch_id)?;
        if levels.iter().any(|&level| level > 175) || switches.iter().any(|&value| value > 1) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected ALSA master value",
            ));
        }
        Ok(MasterVolume {
            gain_db: levels.map(|level| level as f32 * 0.5 - 63.5),
            enabled: switches.map(|value| value != 0),
        })
    }

    fn read_stereo(&self, numid: u32) -> io::Result<[u32; 2]> {
        let mut value = ElemValue::new(numid, &[]);
        ioctl(
            self.fd.raw(),
            CTL_IOCTL_ELEM_READ,
            value.0.as_mut_ptr().cast(),
        )?;
        Ok(value.stereo())
    }

    fn write(&self, numid: u32, values: &[u32]) -> io::Result<()> {
        let mut value = ElemValue::new(numid, values);
        ioctl(
            self.fd.raw(),
            CTL_IOCTL_ELEM_WRITE,
            value.0.as_mut_ptr().cast(),
        )
    }
}

fn enum_switch_id(fd: RawFd, name: &str) -> io::Result<u32> {
    let mut info = mixer_info(fd, name)?;
    if info.control_type() != ControlType::Enumerated
        || info.count() != 1
        || info.enum_items() != 2
        || info.access() & 2 == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unexpected switch: {name}"),
        ));
    }
    for (index, expected) in ["Off", "On"].into_iter().enumerate() {
        info.set_enum_item(index as u32);
        ioctl(fd, CTL_IOCTL_ELEM_INFO, info.as_mut_ptr())?;
        if info.enum_name() != expected {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unexpected switch labels: {name}"),
            ));
        }
    }
    Ok(info.numid())
}

fn master_volume_setting(db: f32) -> io::Result<(u32, bool)> {
    if !db.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "non-finite volume",
        ));
    }
    if db <= -144.0 {
        return Ok((0, false));
    }
    // 127 is 0 dB. Never use the DAC's positive-gain range (+24 dB).
    Ok((
        (127.0 + db.min(0.0) * 2.0).round().clamp(0.0, 127.0) as u32,
        true,
    ))
}

#[repr(C, align(8))]
struct ElemValue([u8; ELEM_VALUE_SIZE]);

impl ElemValue {
    fn new(numid: u32, values: &[u32]) -> Self {
        assert!(values.len() <= 128);
        let mut result = Self([0; ELEM_VALUE_SIZE]);
        result.0[..4].copy_from_slice(&numid.to_ne_bytes());
        for (channel, value) in values.iter().enumerate() {
            let offset = ELEM_VALUE_DATA_OFFSET + channel * 4;
            result.0[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
        }
        result
    }

    fn stereo(&self) -> [u32; 2] {
        std::array::from_fn(|channel| {
            let offset = ELEM_VALUE_DATA_OFFSET + channel * 4;
            u32::from_ne_bytes(self.0[offset..offset + 4].try_into().unwrap())
        })
    }
}

fn mixer_info(fd: RawFd, name: &str) -> io::Result<ElemInfo> {
    if name.len() >= ELEM_ID_NAME_LENGTH || name.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid mixer name",
        ));
    }
    let mut id = ElemId {
        iface: 2,
        ..ElemId::default()
    }; // SNDRV_CTL_ELEM_IFACE_MIXER
    id.name[..name.len()].copy_from_slice(name.as_bytes());
    let mut info = ElemInfo::for_id(id);
    ioctl(fd, CTL_IOCTL_ELEM_INFO, info.as_mut_ptr())?;
    Ok(info)
}

pub(crate) struct Pcm {
    fd: Fd,
    config: PcmConfig,
}

impl Pcm {
    pub(crate) fn open_default() -> io::Result<Self> {
        require_armv7()?;

        let fd = Fd::open(PCM_PATH)?;
        let mut params = HwParams::any();
        params.set_mask(HW_PARAM_ACCESS, PCM_ACCESS_RW_INTERLEAVED);
        params.set_mask(HW_PARAM_FORMAT, PCM_FORMAT_S16_LE);
        params.set_mask(HW_PARAM_SUBFORMAT, PCM_SUBFORMAT_STD);
        params.set_interval(HW_PARAM_SAMPLE_BITS, 16);
        params.set_interval(HW_PARAM_FRAME_BITS, 16 * CHANNELS);
        params.set_interval(HW_PARAM_CHANNELS, CHANNELS);
        params.set_interval(HW_PARAM_RATE, SAMPLE_RATE);
        params.set_interval(HW_PARAM_PERIOD_SIZE, PERIOD_FRAMES);
        params.set_interval(HW_PARAM_PERIODS, PERIODS);

        ioctl(fd.raw(), PCM_IOCTL_HW_PARAMS, params.as_mut_ptr())?;

        let config = PcmConfig {
            sample_rate: params.interval(HW_PARAM_RATE),
            channels: params.interval(HW_PARAM_CHANNELS),
            period_frames: params.interval(HW_PARAM_PERIOD_SIZE),
            periods: params.interval(HW_PARAM_PERIODS),
        };
        let requested = PcmConfig {
            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
            period_frames: PERIOD_FRAMES,
            periods: PERIODS,
        };
        if config != requested
            || !params.mask_contains(HW_PARAM_ACCESS, PCM_ACCESS_RW_INTERLEAVED)
            || !params.mask_contains(HW_PARAM_FORMAT, PCM_FORMAT_S16_LE)
            || !params.mask_contains(HW_PARAM_SUBFORMAT, PCM_SUBFORMAT_STD)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("ALSA granted {config:?}, expected {requested:?}"),
            ));
        }

        let mut software = SwParams::playback();
        ioctl(
            fd.raw(),
            PCM_IOCTL_SW_PARAMS,
            software.0.as_mut_ptr().cast(),
        )?;
        ioctl_noarg(fd.raw(), PCM_IOCTL_PREPARE)?;
        Ok(Self { fd, config })
    }

    pub(crate) fn config(&self) -> PcmConfig {
        self.config
    }

    pub(crate) fn prepare(&mut self) -> io::Result<()> {
        ioctl_noarg(self.fd.raw(), PCM_IOCTL_PREPARE)
    }

    pub(crate) fn drop_stream(&mut self) -> io::Result<()> {
        ioctl_noarg(self.fd.raw(), PCM_IOCTL_DROP)
    }

    #[cfg(feature = "shairplay")]
    pub(crate) fn write_period(&mut self, samples: &[i16]) -> io::Result<()> {
        let expected_samples = (PERIOD_FRAMES * CHANNELS) as usize;
        if samples.len() != expected_samples {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected {expected_samples} samples, got {}", samples.len()),
            ));
        }
        self.write_frames(samples).map(|_| ())
    }

    pub(crate) fn write_frames(&mut self, samples: &[i16]) -> io::Result<u32> {
        if !samples.len().is_multiple_of(CHANNELS as usize) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected interleaved stereo samples, got {}", samples.len()),
            ));
        }
        let total_frames = u32::try_from(samples.len() / CHANNELS as usize)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many PCM frames"))?;

        let mut written_frames = 0_u32;
        while written_frames < total_frames {
            let remaining_frames = total_frames - written_frames;
            let sample_offset = written_frames as usize * CHANNELS as usize;
            let buffer = pointer32(samples[sample_offset..].as_ptr())?;
            let mut transfer = XferI::new(buffer, remaining_frames);
            ioctl(
                self.fd.raw(),
                PCM_IOCTL_WRITEI_FRAMES,
                transfer.as_mut_ptr(),
            )?;

            let completed_frames = transfer.result();
            if completed_frames <= 0 || completed_frames as u32 > remaining_frames {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    format!("invalid ALSA write result {completed_frames}"),
                ));
            }
            written_frames += completed_frames as u32;
        }

        Ok(written_frames)
    }

    pub(crate) fn delay_frames(&self) -> io::Result<i32> {
        let mut delay_frames = 0_i32;
        ioctl(
            self.fd.raw(),
            PCM_IOCTL_DELAY,
            (&mut delay_frames as *mut i32).cast(),
        )?;
        Ok(delay_frames)
    }
}

pub(crate) fn is_xrun(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::EPIPE)
}

struct Fd(RawFd);

impl Fd {
    fn open(path: &[u8]) -> io::Result<Self> {
        // SAFETY: path is a static NUL-terminated byte string; open returns an owned fd.
        let fd = unsafe {
            libc::open(
                path.as_ptr().cast::<libc::c_char>(),
                libc::O_RDWR | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(fd))
    }

    fn raw(&self) -> RawFd {
        self.0
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        // SAFETY: self.0 is owned by this wrapper and close is best-effort during Drop.
        unsafe {
            libc::close(self.0);
        }
    }
}

fn require_armv7() -> io::Result<()> {
    if cfg!(all(target_arch = "arm", target_pointer_width = "32")) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "raw Echo ALSA UAPI is only enabled for 32-bit ARM",
        ))
    }
}

fn pointer32<T>(pointer: *const T) -> io::Result<u32> {
    u32::try_from(pointer as usize).map_err(|_| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "raw Echo ALSA UAPI requires a 32-bit userspace pointer",
        )
    })
}

fn ioctl(fd: RawFd, request: u32, argument: *mut c_void) -> io::Result<()> {
    // SAFETY: callers pass an ARMv7 UAPI buffer, including any declared TLV payload.
    let result = unsafe { libc::ioctl(fd, request as IoctlRequest, argument) };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn ioctl_noarg(fd: RawFd, request: u32) -> io::Result<()> {
    // SAFETY: this ioctl has no payload; zero is the unused variadic argument.
    let result = unsafe { libc::ioctl(fd, request as IoctlRequest, 0) };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[repr(C, align(4))]
struct HwParams([u8; HW_PARAMS_SIZE]);

impl HwParams {
    fn any() -> Self {
        let mut params = Self([0; HW_PARAMS_SIZE]);
        for mask in 0..HW_PARAM_MASK_COUNT {
            let offset = MASKS_OFFSET + mask * MASK_SIZE;
            params.0[offset..offset + MASK_SIZE].fill(u8::MAX);
        }
        for interval in 0..HW_PARAM_INTERVAL_COUNT {
            let offset = INTERVALS_OFFSET + interval * INTERVAL_SIZE;
            params.put_u32(offset, 0);
            params.put_u32(offset + 4, u32::MAX);
        }
        params.put_u32(RMASK_OFFSET, u32::MAX);
        params
    }

    fn set_mask(&mut self, parameter: usize, value: u32) {
        let offset = MASKS_OFFSET + (parameter - HW_PARAM_FIRST_MASK) * MASK_SIZE;
        self.0[offset..offset + MASK_SIZE].fill(0);
        let word_offset = offset + (value as usize / 32) * size_of::<u32>();
        self.put_u32(word_offset, 1_u32 << (value % 32));
    }

    fn mask_contains(&self, parameter: usize, value: u32) -> bool {
        let offset = MASKS_OFFSET + (parameter - HW_PARAM_FIRST_MASK) * MASK_SIZE;
        let word_offset = offset + (value as usize / 32) * size_of::<u32>();
        self.get_u32(word_offset) & (1_u32 << (value % 32)) != 0
    }

    fn set_interval(&mut self, parameter: usize, value: u32) {
        let offset = INTERVALS_OFFSET + (parameter - HW_PARAM_FIRST_INTERVAL) * INTERVAL_SIZE;
        self.put_u32(offset, value);
        self.put_u32(offset + 4, value);
        self.put_u32(offset + 8, 1 << 2); // snd_interval.integer
    }

    fn interval(&self, parameter: usize) -> u32 {
        let offset = INTERVALS_OFFSET + (parameter - HW_PARAM_FIRST_INTERVAL) * INTERVAL_SIZE;
        self.get_u32(offset)
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }

    fn put_u32(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn get_u32(&self, offset: usize) -> u32 {
        u32::from_ne_bytes(
            self.0[offset..offset + size_of::<u32>()]
                .try_into()
                .unwrap(),
        )
    }
}

// ARMv7 snd_pcm_sw_params is 26 32-bit words; offsets are C-asserted.
#[repr(C)]
struct SwParams([u32; SW_PARAMS_SIZE / 4]);

impl SwParams {
    fn playback() -> Self {
        let mut params = Self([0; SW_PARAMS_SIZE / 4]);
        params.0[1] = 1; // period_step, offset 4
        params.0[3] = PERIOD_FRAMES; // avail_min, offset 12
        params.0[4] = 1; // xfer_align, offset 16
        // Let ALSA start only after three periods, including after XRUN recovery.
        params.0[5] = PERIOD_FRAMES * (PERIODS - 1); // start_threshold, offset 20
        params.0[6] = PERIOD_FRAMES * PERIODS; // stop_threshold, offset 24
        params.0[9] = 1 << 30; // 32-bit pointer wrap boundary, offset 36
        params
    }
}

#[repr(C, align(4))]
struct XferI([u8; XFERI_SIZE]);

impl XferI {
    fn new(buffer: u32, frames: u32) -> Self {
        let mut transfer = Self([0; XFERI_SIZE]);
        transfer.put_u32(XFERI_BUFFER_OFFSET, buffer);
        transfer.put_u32(XFERI_FRAMES_OFFSET, frames);
        transfer
    }

    fn result(&self) -> i32 {
        i32::from_ne_bytes(
            self.0[XFERI_RESULT_OFFSET..XFERI_RESULT_OFFSET + size_of::<i32>()]
                .try_into()
                .unwrap(),
        )
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }

    fn put_u32(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_ne_bytes());
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ElemId {
    numid: u32,
    iface: u32,
    device: u32,
    subdevice: u32,
    name: [u8; ELEM_ID_NAME_LENGTH],
    index: u32,
}

impl Default for ElemId {
    fn default() -> Self {
        Self {
            numid: 0,
            iface: 0,
            device: 0,
            subdevice: 0,
            name: [0; ELEM_ID_NAME_LENGTH],
            index: 0,
        }
    }
}

#[repr(C, align(4))]
struct ElemList([u8; ELEM_LIST_SIZE]);

impl ElemList {
    fn query() -> Self {
        Self([0; ELEM_LIST_SIZE])
    }

    fn fetch(space: u32, pids: u32) -> Self {
        let mut list = Self::query();
        list.put_u32(4, space);
        list.put_u32(ELEM_LIST_PIDS_OFFSET, pids);
        list
    }

    fn used(&self) -> u32 {
        self.get_u32(ELEM_LIST_USED_OFFSET)
    }

    fn count(&self) -> u32 {
        self.get_u32(ELEM_LIST_COUNT_OFFSET)
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }

    fn put_u32(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn get_u32(&self, offset: usize) -> u32 {
        u32::from_ne_bytes(
            self.0[offset..offset + size_of::<u32>()]
                .try_into()
                .unwrap(),
        )
    }
}

#[repr(C, align(4))]
struct ElemInfo([u8; ELEM_INFO_SIZE]);

impl ElemInfo {
    fn for_id(id: ElemId) -> Self {
        let mut info = Self([0; ELEM_INFO_SIZE]);
        info.put_u32(0, id.numid);
        info.put_u32(4, id.iface);
        info.put_u32(8, id.device);
        info.put_u32(12, id.subdevice);
        info.0[ELEM_ID_NAME_OFFSET..ELEM_ID_NAME_OFFSET + ELEM_ID_NAME_LENGTH]
            .copy_from_slice(&id.name);
        info.put_u32(60, id.index);
        info
    }

    fn set_enum_item(&mut self, item: u32) {
        self.put_u32(ELEM_INFO_ENUM_ITEM_OFFSET, item);
    }

    fn numid(&self) -> u32 {
        self.get_u32(0)
    }

    fn name(&self) -> String {
        c_string(&self.0[ELEM_ID_NAME_OFFSET..ELEM_ID_NAME_OFFSET + ELEM_ID_NAME_LENGTH])
    }

    fn control_type(&self) -> ControlType {
        self.get_u32(ELEM_INFO_TYPE_OFFSET).into()
    }

    fn access(&self) -> u32 {
        self.get_u32(ELEM_INFO_ACCESS_OFFSET)
    }

    fn count(&self) -> u32 {
        self.get_u32(ELEM_INFO_COUNT_OFFSET)
    }

    fn enum_items(&self) -> u32 {
        self.get_u32(ELEM_INFO_ENUM_ITEMS_OFFSET)
    }

    fn enum_name(&self) -> String {
        c_string(
            &self.0[ELEM_INFO_ENUM_NAME_OFFSET
                ..ELEM_INFO_ENUM_NAME_OFFSET + ELEM_INFO_ENUM_NAME_LENGTH],
        )
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }

    fn put_u32(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn get_u32(&self, offset: usize) -> u32 {
        u32::from_ne_bytes(
            self.0[offset..offset + size_of::<u32>()]
                .try_into()
                .unwrap(),
        )
    }
}

fn enumerate_controls() -> io::Result<Vec<ControlInfo>> {
    let fd = Fd::open(CONTROL_PATH)?;
    let mut count_query = ElemList::query();
    ioctl(fd.raw(), CTL_IOCTL_ELEM_LIST, count_query.as_mut_ptr())?;
    let count = count_query.count() as usize;
    if count > MAX_CONTROLS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("ALSA reported an unreasonable control count: {count}"),
        ));
    }
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut ids = vec![ElemId::default(); count];
    let pointer = pointer32(ids.as_mut_ptr())?;
    let mut list = ElemList::fetch(count as u32, pointer);
    ioctl(fd.raw(), CTL_IOCTL_ELEM_LIST, list.as_mut_ptr())?;
    let used = list.used() as usize;
    if used > count {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("ALSA returned {used} controls for a {count}-slot list"),
        ));
    }
    ids.truncate(used);

    ids.into_iter()
        .map(|id| control_info(fd.raw(), id))
        .collect()
}

fn control_info(fd: RawFd, id: ElemId) -> io::Result<ControlInfo> {
    let mut info = ElemInfo::for_id(id);
    ioctl(fd, CTL_IOCTL_ELEM_INFO, info.as_mut_ptr())?;
    let control_type = info.control_type();
    let enum_items = if control_type == ControlType::Enumerated {
        let items = info.enum_items();
        if items > MAX_ENUM_ITEMS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("ALSA control {} reports {items} enum items", info.name()),
            ));
        }
        (0..items)
            .map(|item| {
                let mut enum_info = ElemInfo::for_id(id);
                enum_info.set_enum_item(item);
                ioctl(fd, CTL_IOCTL_ELEM_INFO, enum_info.as_mut_ptr())?;
                Ok(enum_info.enum_name())
            })
            .collect::<io::Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    Ok(ControlInfo {
        numid: info.numid(),
        name: info.name(),
        control_type,
        count: info.count(),
        access: info.access(),
        enum_items,
    })
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::align_of;

    #[test]
    fn master_volume_maps_to_system_mixer_steps() {
        for (db, level, enabled) in [
            (0.0, 127, true),
            (-20.0, 87, true),
            (-30.0, 67, true),
            (-12.5, 102, true),
            (-144.0, 0, false),
            (-200.0, 0, false),
            (24.0, 127, true),
            (-64.0, 0, true),
        ] {
            assert_eq!(master_volume_setting(db).unwrap(), (level, enabled));
        }
        for db in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(master_volume_setting(db).is_err());
        }
        let value = ElemValue::new(62, &[87, 102]);
        assert_eq!(size_of::<ElemValue>(), 712);
        assert_eq!(align_of::<ElemValue>(), 8);
        assert_eq!(&value.0[..4], &62_u32.to_ne_bytes());
        assert_eq!(&value.0[72..76], &87_u32.to_ne_bytes());
        assert_eq!(&value.0[76..80], &102_u32.to_ne_bytes());
        assert_eq!(value.stereo(), [87, 102]);
        assert_eq!(CTL_IOCTL_TLV_READ, 0xc008_551a);
    }

    #[test]
    fn pcm_waits_for_three_periods_before_starting() {
        let params = SwParams::playback();
        assert_eq!(size_of::<SwParams>(), 104);
        assert_eq!(align_of::<SwParams>(), 4);
        assert_eq!(params.0[3], 1024); // avail_min
        assert_eq!(params.0[5], 3072); // start_threshold
        assert_eq!(params.0[6], 4096); // stop_threshold
        assert_eq!(params.0[9], 1 << 30); // boundary
    }

    #[test]
    fn armv7_pcm_layout_is_explicit() {
        assert_eq!(size_of::<HwParams>(), 604);
        assert_eq!(align_of::<HwParams>(), 4);
        assert_eq!(size_of::<XferI>(), 12);
        assert_eq!(align_of::<XferI>(), 4);
        assert_eq!(MASKS_OFFSET, 4);
        assert_eq!(INTERVALS_OFFSET, 260);
        assert_eq!(RMASK_OFFSET, 512);
        assert_eq!(FIFO_SIZE_OFFSET, 536);
        assert_eq!(RESERVED_OFFSET, 540);
        assert_eq!(XFERI_RESULT_OFFSET, 0);
        assert_eq!(XFERI_BUFFER_OFFSET, 4);
        assert_eq!(XFERI_FRAMES_OFFSET, 8);
    }

    #[test]
    fn armv7_control_layout_is_explicit() {
        assert_eq!(size_of::<ElemId>(), ELEM_ID_SIZE);
        assert_eq!(align_of::<ElemId>(), 4);
        assert_eq!(size_of::<ElemList>(), 72);
        assert_eq!(size_of::<ElemInfo>(), 272);
        assert_eq!(ELEM_VALUE_SIZE, 712);
        assert_eq!(ELEM_LIST_COUNT_OFFSET, 12);
        assert_eq!(ELEM_LIST_PIDS_OFFSET, 16);
        assert_eq!(ELEM_INFO_TYPE_OFFSET, 64);
        assert_eq!(ELEM_INFO_ENUM_ITEMS_OFFSET, 80);
        assert_eq!(ELEM_INFO_ENUM_ITEM_OFFSET, 84);
        assert_eq!(ELEM_INFO_ENUM_NAME_OFFSET, 88);
    }

    #[test]
    fn ioctl_requests_use_32_bit_abi_sizes() {
        assert_eq!(PCM_IOCTL_HW_PARAMS, 0xc25c_4111);
        assert_eq!(PCM_IOCTL_PREPARE, 0x0000_4140);
        assert_eq!(PCM_IOCTL_DELAY, 0x8004_4121);
        assert_eq!(PCM_IOCTL_SW_PARAMS, 0xc068_4113);
        assert_eq!(PCM_IOCTL_DROP, 0x0000_4143);
        assert_eq!(PCM_IOCTL_WRITEI_FRAMES, 0x400c_4150);
        assert_eq!(CTL_IOCTL_ELEM_LIST, 0xc048_5510);
        assert_eq!(CTL_IOCTL_ELEM_INFO, 0xc110_5511);
        assert_eq!(CTL_IOCTL_ELEM_READ, 0xc2c8_5512);
        assert_eq!(CTL_IOCTL_ELEM_WRITE, 0xc2c8_5513);
    }

    #[test]
    fn exact_interval_and_mask_constraints_round_trip() {
        let mut params = HwParams::any();
        params.set_mask(HW_PARAM_FORMAT, PCM_FORMAT_S16_LE);
        params.set_interval(HW_PARAM_RATE, SAMPLE_RATE);
        assert!(params.mask_contains(HW_PARAM_FORMAT, PCM_FORMAT_S16_LE));
        assert_eq!(params.interval(HW_PARAM_RATE), SAMPLE_RATE);
    }
}

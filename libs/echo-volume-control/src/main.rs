#![deny(unsafe_op_in_unsafe_fn)]

use echo_alsa::{MasterVolume, MixerEvent, SystemMixer};
use echo_controls::{
    Button, ButtonReader, ButtonState, RING_SEGMENTS, Rgb, Ring, input_devices,
};
use std::{
    env,
    io::{self, ErrorKind},
    net::UdpSocket,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

const DEFAULT_MIC_MUTE_STATE: &str = "/sys/devices/soc/10010000.keypad/amz_privacy/privacy_state";
const MIC_STATE_POLL: Duration = Duration::from_millis(20);
const VOLUME_VISIBLE_FOR: Duration = Duration::from_secs(2);
const VOLUME_MIN_DB: f32 = -30.0;
const VOLUME_MAX_DB: f32 = 0.0;
const RING_BRIGHTNESS: u8 = 32;
const STARTUP_FLASH: Duration = Duration::from_millis(200);

#[derive(Debug, Eq, PartialEq)]
struct Options {
    no_volume_buttons: bool,
    sync_with_mic_mute: bool,
    metadata_port: Option<u16>,
    mic_mute_state: PathBuf,
    amp_off: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            no_volume_buttons: false,
            sync_with_mic_mute: false,
            metadata_port: None,
            mic_mute_state: PathBuf::from(DEFAULT_MIC_MUTE_STATE),
            amp_off: false,
        }
    }
}

#[derive(Default)]
struct State {
    mic_muted: bool,
    hide_ring_at: Option<Instant>,
}

fn usage() -> &'static str {
    "usage: echo-volume-control [--no-volume-buttons] [--sync-with-mic-mute] \\\n--metadata-port PORT [--mic-mute-state PATH] [--amp-off]"
}

fn main() -> ExitCode {
    let options = match parse_options(env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("echo-volume-control: {error}\n{}", usage());
            return ExitCode::from(2);
        }
    };

    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("echo-volume-control: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_options(arguments: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut options = Options::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => return Ok(None),
            "--no-volume-buttons" => options.no_volume_buttons = true,
            "--sync-with-mic-mute" => options.sync_with_mic_mute = true,
            "--amp-off" => options.amp_off = true,
            "--metadata-port" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--metadata-port needs a port".to_owned())?;
                let port = value
                    .parse::<u16>()
                    .map_err(|_| format!("invalid metadata port: {value}"))?;
                if port == 0 {
                    return Err("metadata port must be non-zero".to_owned());
                }
                options.metadata_port = Some(port);
            }
            "--mic-mute-state" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--mic-mute-state needs a path".to_owned())?;
                options.mic_mute_state = PathBuf::from(value);
            }
            _ => return Err(format!("unknown option: {argument}")),
        }
    }
    if options.amp_off
        && (options.no_volume_buttons
            || options.sync_with_mic_mute
            || options.metadata_port.is_some())
    {
        return Err("--amp-off cannot be combined with controller options".to_owned());
    }
    Ok(Some(options))
}

fn run(options: Options) -> io::Result<()> {
    let mut mixer = SystemMixer::open()?;
    if options.amp_off {
        return mixer.set_amp(false);
    }

    mixer.subscribe_events()?;
    // Fail closed while the physical privacy state is read.
    mixer.set_amp(false)?;

    let mut ring = Ring::open_default()?;
    ring.set_boot_animation(false)?;
    ring.set_all(Rgb::new(0, RING_BRIGHTNESS, 0))?;
    std::thread::sleep(STARTUP_FLASH);
    ring.off()?;

    let mut buttons = open_buttons()?;
    let metadata = options.metadata_port.map(bind_metadata).transpose()?;
    let mut state = State::default();
    let mut next_mic_state = Instant::now();

    if options.sync_with_mic_mute {
        state.mic_muted = read_mic_mute_state(&options.mic_mute_state)?;
    }
    apply_amp(&mut mixer, &state)?;

    loop {
        let now = Instant::now();
        if options.sync_with_mic_mute && now >= next_mic_state {
            let muted = read_mic_mute_state(&options.mic_mute_state)?;
            if state.mic_muted != muted {
                state.mic_muted = muted;
                apply_amp(&mut mixer, &state)?;
            }
            next_mic_state = now + MIC_STATE_POLL;
        }
        if state.hide_ring_at.is_some_and(|deadline| now >= deadline) {
            ring.off()?;
            state.hide_ring_at = None;
        }

        let mut fds = poll_fds(&mixer, &buttons, metadata.as_ref());
        let timeout = poll_timeout(now, &state, options.sync_with_mic_mute.then_some(next_mic_state));
        // SAFETY: fds is valid writable memory for the duration of poll.
        let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if result == 0 {
            continue;
        }

        let mixer_ready = fds[0].revents & libc::POLLIN != 0;
        if mixer_ready && matches!(mixer.next_event()?, Some(MixerEvent::Volume)) {
            show_volume(&mut ring, mixer.volume()?, &mut state)?;
        }

        let button_start = 1;
        for (index, reader) in buttons.iter_mut().enumerate() {
            if fds[button_start + index].revents & libc::POLLIN == 0 {
                continue;
            }
            while let Some(event) = reader.try_next_event()? {
                if !options.no_volume_buttons
                    && matches!(event.button, Button::VolumeDown | Button::VolumeUp)
                    && matches!(event.state, ButtonState::Pressed | ButtonState::Repeat)
                {
                    let step = if event.button == Button::VolumeUp { 1 } else { -1 };
                    mixer.adjust_volume_db(step)?;
                    show_volume(&mut ring, mixer.volume()?, &mut state)?;
                }
            }
        }

        if let Some(socket) = metadata.as_ref() {
            let metadata_index = button_start + buttons.len();
            if fds[metadata_index].revents & libc::POLLIN != 0 {
                receive_metadata(socket, &mut mixer, &mut ring, &mut state)?;
            }
        }
    }
}

fn open_buttons() -> io::Result<Vec<ButtonReader>> {
    let mut readers = Vec::new();
    for device in input_devices()? {
        if device.name != "keys" && device.name != "mtk-kpd" {
            continue;
        }
        match ButtonReader::open_nonblocking(&device.path) {
            Ok(reader) => readers.push(reader),
            Err(error) => eprintln!("echo-volume-control: {}: {error}", device.path.display()),
        }
    }
    Ok(readers)
}

fn bind_metadata(port: u16) -> io::Result<UdpSocket> {
    let socket = UdpSocket::bind(("127.0.0.1", port))?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

fn poll_fds(
    mixer: &SystemMixer,
    buttons: &[ButtonReader],
    metadata: Option<&UdpSocket>,
) -> Vec<libc::pollfd> {
    let mut fds = Vec::with_capacity(1 + buttons.len() + usize::from(metadata.is_some()));
    fds.push(libc::pollfd {
        fd: mixer.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    });
    fds.extend(buttons.iter().map(|reader| libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    }));
    if let Some(socket) = metadata {
        fds.push(libc::pollfd {
            fd: socket.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
    }
    fds
}

fn poll_timeout(now: Instant, state: &State, next_mic_state: Option<Instant>) -> i32 {
    let deadline = [state.hide_ring_at, next_mic_state]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(now + Duration::from_secs(1));
    let duration = deadline.saturating_duration_since(now);
    i32::try_from(duration.as_millis().max(1)).unwrap_or(i32::MAX)
}

fn receive_metadata(
    socket: &UdpSocket,
    mixer: &mut SystemMixer,
    ring: &mut Ring,
    state: &mut State,
) -> io::Result<()> {
    let mut packet = [0_u8; 512];
    loop {
        let (length, _) = match socket.recv_from(&mut packet) {
            Ok(message) => message,
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error),
        };
        if length < 8 || &packet[..4] != b"ssnc" {
            continue;
        }
        match &packet[4..8] {
            b"pvol" => {
                if let Some(db) = parse_pvol(&packet[8..length]) {
                    mixer.set_volume_db(db)?;
                    show_volume(ring, mixer.volume()?, state)?;
                }
            }
            _ => {}
        }
    }
}

fn parse_pvol(payload: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(payload).ok()?;
    let mut fields = text.split(',');
    let airplay_db = fields.next()?.trim().parse::<f32>().ok()?;
    if airplay_db == -144.0 {
        return Some(-144.0);
    }
    let db = fields.next()?.trim().parse::<f32>().ok()?;
    db.is_finite().then_some(db)
}

fn read_mic_mute_state(path: &Path) -> io::Result<bool> {
    parse_mic_mute_state(&std::fs::read_to_string(path)?)
}

fn parse_mic_mute_state(value: &str) -> io::Result<bool> {
    match value.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            "mic mute state must be 0 or 1",
        )),
    }
}

fn amp_enabled(state: &State) -> bool {
    !state.mic_muted
}

fn apply_amp(mixer: &mut SystemMixer, state: &State) -> io::Result<()> {
    mixer.set_amp(amp_enabled(state))
}

fn show_volume(ring: &mut Ring, volume: MasterVolume, state: &mut State) -> io::Result<()> {
    ring.set_segments(volume_frame(volume))?;
    state.hide_ring_at = Some(Instant::now() + VOLUME_VISIBLE_FOR);
    Ok(())
}

fn volume_frame(volume: MasterVolume) -> [Rgb; RING_SEGMENTS] {
    let db = volume
        .gain_db
        .into_iter()
        .zip(volume.enabled)
        .filter_map(|(db, enabled)| enabled.then_some(db))
        .fold(VOLUME_MIN_DB, f32::max);
    let fraction = ((db - VOLUME_MIN_DB) / (VOLUME_MAX_DB - VOLUME_MIN_DB)).clamp(0.0, 1.0);
    let lit = (fraction * RING_SEGMENTS as f32).round() as usize;
    let mut frame = [Rgb::BLACK; RING_SEGMENTS];
    frame[..lit].fill(Rgb::new(RING_BRIGHTNESS, RING_BRIGHTNESS, RING_BRIGHTNESS));
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_controller_flags() {
        let options = parse_options([
            "--no-volume-buttons".to_owned(),
            "--sync-with-mic-mute".to_owned(),
            "--metadata-port".to_owned(),
            "45678".to_owned(),
        ])
        .unwrap()
        .unwrap();
        assert!(options.no_volume_buttons);
        assert!(options.sync_with_mic_mute);
        assert_eq!(options.metadata_port, Some(45678));
    }

    #[test]
    fn rejects_bad_cli_combinations() {
        assert!(parse_options(["--metadata-port".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_options(["--amp-off".to_owned(), "--no-volume-buttons".to_owned()]).is_err());
    }

    #[test]
    fn parses_shairport_hardware_volume_from_pvol() {
        assert_eq!(parse_pvol(b"-20.00,-17.10,-30.00,0.00"), Some(-17.1));
        assert_eq!(parse_pvol(b"-144.00,0.00,0.00,0.00"), Some(-144.0));
        assert_eq!(parse_pvol(b"broken"), None);
    }

    #[test]
    fn parses_the_privacy_gpio_contract() {
        assert!(!parse_mic_mute_state("0\n").unwrap());
        assert!(parse_mic_mute_state("1\n").unwrap());
        assert!(parse_mic_mute_state("unknown").is_err());
    }

    #[test]
    fn mic_mute_is_the_amp_kill_switch() {
        assert!(amp_enabled(&State::default()));
        assert!(!amp_enabled(&State {
            mic_muted: true,
            ..State::default()
        }));
    }

    #[test]
    fn maps_actual_mixer_volume_to_the_ring() {
        let frame = volume_frame(MasterVolume {
            gain_db: [-15.0; 2],
            enabled: [true; 2],
        });
        assert_eq!(frame.iter().filter(|color| **color != Rgb::BLACK).count(), 6);
        assert!(volume_frame(MasterVolume {
            gain_db: [0.0; 2],
            enabled: [false; 2],
        })
        .iter()
        .all(|color| *color == Rgb::BLACK));
    }
}

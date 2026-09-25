#![deny(unsafe_op_in_unsafe_fn)]

//! Minimal direct controls for the Echo Dot LED ring and top buttons.
//! Run the bundled `controls-preflight` on the target before writing LEDs.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    mem::size_of,
    os::{fd::{AsRawFd, RawFd}, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
};

mod ffi;

/// Echo Dot IS31FL3236 sysfs directory. Validate it with `controls-preflight` first.
pub const DEFAULT_RING_PATH: &str = "/sys/bus/i2c/devices/0-003f";
pub const RING_SEGMENTS: usize = 12;
pub const RING_CHANNELS: usize = RING_SEGMENTS * 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

/// Whole-frame LED-ring writer. It owns no animation policy and performs no writes at open time.
pub struct Ring {
    root: PathBuf,
    last_frame: Option<[u8; RING_CHANNELS]>,
}

impl Ring {
    pub fn open_default() -> io::Result<Self> {
        Self::open(DEFAULT_RING_PATH)
    }

    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let root = path.as_ref().to_path_buf();
        fs::metadata(root.join("frame"))?;
        Ok(Self {
            root,
            last_frame: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Sets all 12 RGB ring segments in ascending hardware channel order.
    pub fn set_segments(&mut self, segments: [Rgb; RING_SEGMENTS]) -> io::Result<()> {
        let frame = encode_frame(&segments);
        if self.last_frame == Some(frame) {
            return Ok(());
        }

        let mut encoded = Vec::with_capacity(RING_CHANNELS * 2 + 1);
        for value in frame {
            encoded.push(HEX[(value >> 4) as usize]);
            encoded.push(HEX[(value & 0x0f) as usize]);
        }
        encoded.push(b'\n');
        fs::write(self.root.join("frame"), encoded)?;
        self.last_frame = Some(frame);
        Ok(())
    }

    pub fn set_all(&mut self, color: Rgb) -> io::Result<()> {
        self.set_segments([color; RING_SEGMENTS])
    }

    pub fn off(&mut self) -> io::Result<()> {
        self.set_all(Rgb::BLACK)
    }

    pub fn set_current(&self, current: u8) -> io::Result<()> {
        fs::write(self.root.join("led_current"), current.to_string())
    }

    pub fn set_boot_animation(&self, enabled: bool) -> io::Result<()> {
        fs::write(
            self.root.join("boot_animation"),
            if enabled { b"1" } else { b"0" },
        )
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn encode_frame(segments: &[Rgb; RING_SEGMENTS]) -> [u8; RING_CHANNELS] {
    let mut frame = [0; RING_CHANNELS];
    for (segment, color) in segments.iter().enumerate() {
        let channel = segment * 3;
        frame[channel] = color.red;
        frame[channel + 1] = color.green;
        frame[channel + 2] = color.blue;
    }
    frame
}

pub const KEY_MUTE: u16 = 113;
pub const KEY_VOLUME_DOWN: u16 = 114;
pub const KEY_VOLUME_UP: u16 = 115;
pub const KEY_ACTION: u16 = 138;
const EV_KEY: u16 = 0x01;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Button {
    Mute,
    VolumeDown,
    VolumeUp,
    Action,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonState {
    Pressed,
    Released,
    Repeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ButtonEvent {
    pub button: Button,
    pub state: ButtonState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputDevice {
    pub path: PathBuf,
    pub name: String,
}

/// Lists input nodes without claiming or reading from them.
pub fn input_devices() -> io::Result<Vec<InputDevice>> {
    let mut devices = Vec::new();
    for entry in fs::read_dir("/dev/input")? {
        let entry = entry?;
        let event_name = entry.file_name();
        if !event_name.to_string_lossy().starts_with("event") {
            continue;
        }

        let name = fs::read_to_string(
            Path::new("/sys/class/input")
                .join(&event_name)
                .join("device/name"),
        )
        .map(|name| name.trim().to_owned())
        .unwrap_or_else(|_| "unknown".to_owned());
        devices.push(InputDevice {
            path: entry.path(),
            name,
        });
    }
    devices.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(devices)
}

/// Blocking reader for one `/dev/input/event*` node.
pub struct ButtonReader {
    file: File,
}

impl ButtonReader {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: OpenOptions::new().read(true).open(path)?,
        })
    }

    /// Opens without grabbing the device; mute and other system consumers remain untouched.
    pub fn open_nonblocking(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)?,
        })
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// Returns `None` when a nonblocking reader has no pending key event.
    pub fn try_next_event(&mut self) -> io::Result<Option<ButtonEvent>> {
        match self.next_event() {
            Ok(event) => Ok(Some(event)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Blocks until this input node reports one of the four Echo top-button key events.
    pub fn next_event(&mut self) -> io::Result<ButtonEvent> {
        let long_bytes = size_of::<libc::c_long>();
        let event_bytes = input_event_size(long_bytes)?;
        let mut buffer = [0_u8; 24];
        loop {
            self.file.read_exact(&mut buffer[..event_bytes])?;
            if let Some(event) = decode_input_event(&buffer[..event_bytes], long_bytes)? {
                return Ok(event);
            }
        }
    }
}

fn input_event_size(long_bytes: usize) -> io::Result<usize> {
    match long_bytes {
        4 | 8 => Ok(2 * long_bytes + 8),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "unsupported input_event long width",
        )),
    }
}

fn decode_input_event(bytes: &[u8], long_bytes: usize) -> io::Result<Option<ButtonEvent>> {
    let event_bytes = input_event_size(long_bytes)?;
    if bytes.len() != event_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "short input_event record",
        ));
    }

    let event_type = u16::from_le_bytes([bytes[2 * long_bytes], bytes[2 * long_bytes + 1]]);
    let code = u16::from_le_bytes([bytes[2 * long_bytes + 2], bytes[2 * long_bytes + 3]]);
    if event_type == 0 && code == 3 {
        // SYN_DROPPED: a release may be missing. Stop rather than leave volume repeating.
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "evdev events lost; reopen buttons",
        ));
    }
    if event_type != EV_KEY {
        return Ok(None);
    }
    let Some(button) = button_for(code) else {
        return Ok(None);
    };
    let value = i32::from_le_bytes([
        bytes[2 * long_bytes + 4],
        bytes[2 * long_bytes + 5],
        bytes[2 * long_bytes + 6],
        bytes[2 * long_bytes + 7],
    ]);
    let state = match value {
        0 => ButtonState::Released,
        1 => ButtonState::Pressed,
        2 => ButtonState::Repeat,
        _ => return Ok(None),
    };
    Ok(Some(ButtonEvent { button, state }))
}

fn button_for(code: u16) -> Option<Button> {
    match code {
        KEY_MUTE => Some(Button::Mute),
        KEY_VOLUME_DOWN => Some(Button::VolumeDown),
        KEY_VOLUME_UP => Some(Button::VolumeUp),
        KEY_ACTION => Some(Button::Action),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_segments_encode_as_consecutive_triplets() {
        let mut segments = [Rgb::BLACK; RING_SEGMENTS];
        segments[0] = Rgb::new(1, 2, 3);
        segments[RING_SEGMENTS - 1] = Rgb::new(4, 5, 6);

        let frame = encode_frame(&segments);
        assert_eq!(&frame[..3], &[1, 2, 3]);
        assert_eq!(&frame[RING_CHANNELS - 3..], &[4, 5, 6]);
    }

    #[test]
    fn armv7_input_event_decodes_echo_action_button() {
        let mut event = [0_u8; 16];
        event[8..10].copy_from_slice(&EV_KEY.to_le_bytes());
        event[10..12].copy_from_slice(&KEY_ACTION.to_le_bytes());
        event[12..16].copy_from_slice(&1_i32.to_le_bytes());

        assert_eq!(
            decode_input_event(&event, 4).unwrap(),
            Some(ButtonEvent {
                button: Button::Action,
                state: ButtonState::Pressed,
            })
        );
    }

    #[test]
    fn lost_events_must_cancel_held_buttons() {
        let mut event = [0_u8; 16];
        event[10..12].copy_from_slice(&3_u16.to_le_bytes());
        assert_eq!(
            decode_input_event(&event, 4).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn unrelated_input_event_is_ignored() {
        let mut event = [0_u8; 16];
        event[8..10].copy_from_slice(&EV_KEY.to_le_bytes());
        event[10..12].copy_from_slice(&999_u16.to_le_bytes());
        event[12..16].copy_from_slice(&1_i32.to_le_bytes());

        assert_eq!(decode_input_event(&event, 4).unwrap(), None);
    }
}

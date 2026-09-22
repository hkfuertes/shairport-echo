use crate::{
    Button, ButtonEvent, ButtonReader, ButtonState, RING_CHANNELS, RING_SEGMENTS, Rgb, Ring,
};
use std::{
    ffi::{CStr, c_char, c_int},
    io,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr,
    sync::Mutex,
};

pub struct EchoControlsRing {
    ring: Mutex<Ring>,
}

pub struct EchoControlsButtons {
    reader: Mutex<ButtonReader>,
}

#[repr(C)]
pub struct EchoControlsButtonEvent {
    pub button: u32,
    pub state: u32,
}

const BUTTON_MUTE: u32 = 1;
const BUTTON_VOLUME_DOWN: u32 = 2;
const BUTTON_VOLUME_UP: u32 = 3;
const BUTTON_ACTION: u32 = 4;
const BUTTON_PRESSED: u32 = 1;
const BUTTON_RELEASED: u32 = 2;
const BUTTON_REPEAT: u32 = 3;

fn ffi_status(operation: impl FnOnce() -> io::Result<()>) -> c_int {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => error_status(&error),
        Err(_) => -libc::EFAULT,
    }
}

fn error_status(error: &io::Error) -> c_int {
    let errno = error.raw_os_error().unwrap_or_else(|| match error.kind() {
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => libc::EINVAL,
        io::ErrorKind::WouldBlock => libc::EAGAIN,
        io::ErrorKind::NotFound => libc::ENOENT,
        io::ErrorKind::PermissionDenied => libc::EACCES,
        io::ErrorKind::Unsupported => libc::ENOTSUP,
        _ => libc::EIO,
    });
    -errno
}

fn invalid_argument() -> io::Error {
    io::Error::from_raw_os_error(libc::EINVAL)
}

fn c_path(path: *const c_char) -> io::Result<PathBuf> {
    if path.is_null() {
        return Err(invalid_argument());
    }
    // SAFETY: callers must pass a non-null, NUL-terminated path valid for this call.
    let path = unsafe { CStr::from_ptr(path) }
        .to_str()
        .map_err(|_| invalid_argument())?;
    if path.is_empty() {
        return Err(invalid_argument());
    }
    Ok(PathBuf::from(path))
}

fn clear_out<T>(out: *mut *mut T) -> io::Result<()> {
    if out.is_null() {
        return Err(invalid_argument());
    }
    // SAFETY: callers must pass writable storage for one handle pointer.
    unsafe { *out = ptr::null_mut() };
    Ok(())
}

fn set_out<T>(out: *mut *mut T, value: T) {
    // SAFETY: clear_out has checked that out is valid writable storage.
    unsafe { *out = Box::into_raw(Box::new(value)) };
}

fn with_ring(
    handle: *mut EchoControlsRing,
    operation: impl FnOnce(&mut Ring) -> io::Result<()>,
) -> c_int {
    ffi_status(|| {
        if handle.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: callers may only pass a live handle returned by echo_controls_ring_open*.
        let handle = unsafe { &*handle };
        let mut ring = handle
            .ring
            .lock()
            .map_err(|_| io::Error::other("Echo controls ring lock poisoned"))?;
        operation(&mut ring)
    })
}

fn with_buttons(
    handle: *mut EchoControlsButtons,
    operation: impl FnOnce(&mut ButtonReader) -> io::Result<()>,
) -> c_int {
    ffi_status(|| {
        if handle.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: callers may only pass a live handle returned by echo_controls_buttons_open.
        let handle = unsafe { &*handle };
        let mut reader = handle
            .reader
            .lock()
            .map_err(|_| io::Error::other("Echo controls button lock poisoned"))?;
        operation(&mut reader)
    })
}

fn c_button_event(event: ButtonEvent) -> EchoControlsButtonEvent {
    let button = match event.button {
        Button::Mute => BUTTON_MUTE,
        Button::VolumeDown => BUTTON_VOLUME_DOWN,
        Button::VolumeUp => BUTTON_VOLUME_UP,
        Button::Action => BUTTON_ACTION,
    };
    let state = match event.state {
        ButtonState::Pressed => BUTTON_PRESSED,
        ButtonState::Released => BUTTON_RELEASED,
        ButtonState::Repeat => BUTTON_REPEAT,
    };
    EchoControlsButtonEvent { button, state }
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_open_default(out: *mut *mut EchoControlsRing) -> c_int {
    ffi_status(|| {
        clear_out(out)?;
        set_out(
            out,
            EchoControlsRing {
                ring: Mutex::new(Ring::open_default()?),
            },
        );
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_open(
    path: *const c_char,
    out: *mut *mut EchoControlsRing,
) -> c_int {
    ffi_status(|| {
        clear_out(out)?;
        set_out(
            out,
            EchoControlsRing {
                ring: Mutex::new(Ring::open(c_path(path)?)?),
            },
        );
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_close(handle: *mut EchoControlsRing) -> c_int {
    ffi_status(|| {
        if handle.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: ownership transfers back exactly once from the caller to this function.
        unsafe { drop(Box::from_raw(handle)) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_set_frame(
    handle: *mut EchoControlsRing,
    rgb: *const u8,
    bytes: usize,
) -> c_int {
    with_ring(handle, |ring| {
        if rgb.is_null() || bytes != RING_CHANNELS {
            return Err(invalid_argument());
        }
        // SAFETY: callers must supply exactly RING_CHANNELS readable bytes.
        let rgb = unsafe { std::slice::from_raw_parts(rgb, bytes) };
        let mut segments = [Rgb::BLACK; RING_SEGMENTS];
        for (index, segment) in segments.iter_mut().enumerate() {
            let offset = index * 3;
            *segment = Rgb::new(rgb[offset], rgb[offset + 1], rgb[offset + 2]);
        }
        ring.set_segments(segments)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_set_all(
    handle: *mut EchoControlsRing,
    red: u8,
    green: u8,
    blue: u8,
) -> c_int {
    with_ring(handle, |ring| ring.set_all(Rgb::new(red, green, blue)))
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_off(handle: *mut EchoControlsRing) -> c_int {
    with_ring(handle, Ring::off)
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_set_current(
    handle: *mut EchoControlsRing,
    current: u8,
) -> c_int {
    with_ring(handle, |ring| ring.set_current(current))
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_ring_set_boot_animation(
    handle: *mut EchoControlsRing,
    enabled: c_int,
) -> c_int {
    with_ring(handle, |ring| ring.set_boot_animation(enabled != 0))
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_buttons_open(
    path: *const c_char,
    nonblocking: c_int,
    out: *mut *mut EchoControlsButtons,
) -> c_int {
    ffi_status(|| {
        clear_out(out)?;
        let path = c_path(path)?;
        let reader = if nonblocking == 0 {
            ButtonReader::open(path)?
        } else {
            ButtonReader::open_nonblocking(path)?
        };
        set_out(
            out,
            EchoControlsButtons {
                reader: Mutex::new(reader),
            },
        );
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_buttons_close(handle: *mut EchoControlsButtons) -> c_int {
    ffi_status(|| {
        if handle.is_null() {
            return Err(invalid_argument());
        }
        // SAFETY: ownership transfers back exactly once from the caller to this function.
        unsafe { drop(Box::from_raw(handle)) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_controls_buttons_next(
    handle: *mut EchoControlsButtons,
    out: *mut EchoControlsButtonEvent,
) -> c_int {
    with_buttons(handle, |reader| {
        if out.is_null() {
            return Err(invalid_argument());
        }
        let event = c_button_event(reader.next_event()?);
        // SAFETY: callers must pass writable storage for one event.
        unsafe { *out = event };
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::CString,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_ring_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("echo-controls-ffi-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).expect("create temporary ring root");
        fs::write(root.join("frame"), []).expect("create frame endpoint");
        root
    }

    #[test]
    fn ffi_ring_writes_one_rgb_triplet_per_segment() {
        let root = temporary_ring_root();
        let path = CString::new(root.to_string_lossy().as_bytes()).expect("path has no NUL");
        let mut ring = ptr::null_mut();
        assert_eq!(echo_controls_ring_open(path.as_ptr(), &mut ring), 0);

        let mut frame = [0_u8; RING_CHANNELS];
        for segment in frame.chunks_exact_mut(3) {
            segment.copy_from_slice(&[1, 2, 3]);
        }
        assert_eq!(
            echo_controls_ring_set_frame(ring, frame.as_ptr(), frame.len()),
            0
        );
        assert_eq!(
            fs::read(root.join("frame")).expect("read frame"),
            b"010203"
                .repeat(RING_SEGMENTS)
                .into_iter()
                .chain([b'\n'])
                .collect::<Vec<_>>()
        );
        assert_eq!(echo_controls_ring_close(ring), 0);
        fs::remove_dir_all(root).expect("remove temporary ring root");
    }

    #[test]
    fn ffi_button_values_are_stable_for_c() {
        let event = c_button_event(ButtonEvent {
            button: Button::Action,
            state: ButtonState::Pressed,
        });
        assert_eq!((event.button, event.state), (BUTTON_ACTION, BUTTON_PRESSED));
    }
}

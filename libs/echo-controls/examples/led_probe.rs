use echo_controls::{RING_SEGMENTS, Rgb, Ring};
use std::{error::Error, fs, io, thread, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    let mut ring = Ring::open_default()?;
    if fs::read_to_string(ring.path().join("boot_animation"))?.trim() != "0" {
        return Err(io::Error::other("stop the boot animation before testing the ring").into());
    }
    let path = ring.path().join("frame");
    let original = fs::read(&path)?;

    // Preserve the hardware current limit and restore the previous frame, even on I/O failure.
    let result = probe(&mut ring);
    let restore = fs::write(&path, original);
    result?;
    restore?;
    println!("PASS: all frames read back correctly; previous frame restored.");
    Ok(())
}

fn probe(ring: &mut Ring) -> io::Result<()> {
    for (name, color) in [
        ("red", Rgb::new(32, 0, 0)),
        ("green", Rgb::new(0, 32, 0)),
        ("blue", Rgb::new(0, 0, 32)),
    ] {
        println!("Ring: {name}");
        checked_frame(ring, [color; RING_SEGMENTS])?;
        thread::sleep(Duration::from_secs(2));
    }
    println!("Walking one white segment through hardware indices 0..11");
    for index in 0..RING_SEGMENTS {
        let mut segments = [Rgb::BLACK; RING_SEGMENTS];
        segments[index] = Rgb::new(32, 32, 32);
        checked_frame(ring, segments)?;
        thread::sleep(Duration::from_millis(250));
    }
    checked_frame(ring, [Rgb::BLACK; RING_SEGMENTS])
}

fn checked_frame(ring: &mut Ring, segments: [Rgb; RING_SEGMENTS]) -> io::Result<()> {
    ring.set_segments(segments)?;
    let expected: String = segments
        .iter()
        .flat_map(|color| [color.red, color.green, color.blue])
        .map(|value| format!("{value:02x}"))
        .collect();
    let actual = fs::read_to_string(ring.path().join("frame"))?;
    if actual.trim() != expected {
        return Err(io::Error::other(
            "LED frame read-back differs from requested RGB values",
        ));
    }
    Ok(())
}

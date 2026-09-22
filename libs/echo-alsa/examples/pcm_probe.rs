//! Hardware regression: paced callbacks with 8 ms delivery jitter, without AirPlay.
//! Silent by default. `--tone` emits a short, low-level 750 Hz tone.
use echo_alsa::{CHANNELS, EchoAlsaSink, PERIOD_FRAMES, SAMPLE_RATE};
use shairplay::{AudioCodec, AudioFormat, AudioHandler};
use std::{
    error::Error,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let tone = match args.as_slice() {
        [] => false,
        [arg] if arg == "--tone" => true,
        _ => return Err("usage: pcm_probe [--tone]".into()),
    };
    let mut sink = EchoAlsaSink::open()?;
    let mut session = sink.audio_handler().audio_init(AudioFormat {
        codec: AudioCodec::Pcm,
        bits: 32,
        channels: CHANNELS as u8,
        sample_rate: SAMPLE_RATE,
    });
    let block: Vec<f32> = (0..PERIOD_FRAMES)
        .flat_map(|frame| {
            let sample = if tone {
                0.1 * (std::f32::consts::TAU * 750.0 * frame as f32 / SAMPLE_RATE as f32).sin()
            } else {
                0.0
            };
            [sample; CHANNELS as usize]
        })
        .collect();

    let periods = 48_u64;
    for burst in 0..2 {
        // Do not stall the PCM producer with diagnostic subprocesses.
        let observer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(350));
            let status = std::fs::read_to_string("/proc/asound/card0/pcm23p/sub0/status").unwrap();
            assert!(status.contains("state: RUNNING"), "{status}");
            for name in ["MFP Gpio Mute", "Right Channel Only"] {
                let output = Command::new("/system/bin/tinymix")
                    .arg(name)
                    .output()
                    .unwrap();
                assert!(output.status.success());
                let state = String::from_utf8(output.stdout).unwrap();
                assert!(state.contains(">Off"), "burst {burst}: {state}");
            }
        });
        let start = Instant::now();
        for period in 0..periods {
            session.audio_process(&block);
            let deadline = start
                + Duration::from_nanos(
                    (period + 1) * u64::from(PERIOD_FRAMES) * 1_000_000_000
                        / u64::from(SAMPLE_RATE),
                )
                + Duration::from_millis(if period % 2 == 0 { 8 } else { 0 });
            thread::sleep(deadline.saturating_duration_since(Instant::now()));
        }
        observer
            .join()
            .expect("hardware playback/mute check failed");
        // Force one XRUN between bursts: recovery must also release GPIO mute.
        thread::sleep(Duration::from_millis(160));
        let telemetry = sink.telemetry();
        println!("burst {burst}: {telemetry:?}");
        assert_eq!(
            telemetry.underruns, burst,
            "only the deliberate gap may underrun"
        );
    }
    sink.shutdown();
    let telemetry = sink.telemetry();
    println!("{telemetry:?}");
    assert_eq!(
        telemetry.written_frames,
        2 * periods * u64::from(PERIOD_FRAMES)
    );
    assert_eq!(telemetry.underruns, 1);
    assert_eq!(telemetry.writer_errors, 0);
    assert_eq!(telemetry.dropped_frames, 0);
    Ok(())
}

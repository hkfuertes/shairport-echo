use crate::alsa::{CHANNELS, MasterVolume, Mixer, PERIOD_FRAMES, Pcm, SAMPLE_RATE, is_xrun};
use shairplay::{AudioFormat, AudioHandler, AudioSession};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread::{self, JoinHandle},
};

const BLOCK_SAMPLES: usize = (PERIOD_FRAMES * CHANNELS) as usize;
const QUEUE_PERIODS: usize = 8;

#[derive(Debug, Clone, Copy, Default)]
pub struct Telemetry {
    pub queued_blocks: u64,
    pub max_queued_blocks: u64,
    pub dropped_blocks: u64,
    pub dropped_frames: u64,
    pub stale_blocks: u64,
    pub underruns: u64,
    pub writer_errors: u64,
    pub mixer_errors: u64,
    pub written_frames: u64,
    pub unsupported_streams: u64,
    pub malformed_samples: u64,
    pub last_errno: i32,
}

pub struct EchoAlsaSink {
    sender: SyncSender<Message>,
    shared: Arc<Shared>,
    mixer: Arc<Mutex<Mixer>>,
    writer: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct EchoAudioHandler {
    sender: SyncSender<Message>,
    shared: Arc<Shared>,
    mixer: Arc<Mutex<Mixer>>,
}

struct EchoAudioSession {
    sender: SyncSender<Message>,
    shared: Arc<Shared>,
    generation: u64,
    valid_format: bool,
    partial: Vec<f32>,
}

enum Message {
    Audio(AudioBlock),
    Stop,
}

struct AudioBlock {
    generation: u64,
    samples: Vec<f32>,
}

struct Shared {
    generation: AtomicU64,
    stopping: AtomicBool,
    queued_blocks: AtomicU64,
    max_queued_blocks: AtomicU64,
    dropped_blocks: AtomicU64,
    dropped_frames: AtomicU64,
    stale_blocks: AtomicU64,
    underruns: AtomicU64,
    writer_errors: AtomicU64,
    mixer_errors: AtomicU64,
    written_frames: AtomicU64,
    unsupported_streams: AtomicU64,
    malformed_samples: AtomicU64,
    last_errno: AtomicI32,
}

impl Shared {
    fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            stopping: AtomicBool::new(false),
            queued_blocks: AtomicU64::new(0),
            max_queued_blocks: AtomicU64::new(0),
            dropped_blocks: AtomicU64::new(0),
            dropped_frames: AtomicU64::new(0),
            stale_blocks: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            writer_errors: AtomicU64::new(0),
            mixer_errors: AtomicU64::new(0),
            written_frames: AtomicU64::new(0),
            unsupported_streams: AtomicU64::new(0),
            malformed_samples: AtomicU64::new(0),
            last_errno: AtomicI32::new(0),
        }
    }

    fn observe_queue_depth(&self, depth: u64) {
        let mut previous = self.max_queued_blocks.load(Ordering::Relaxed);
        while depth > previous {
            match self.max_queued_blocks.compare_exchange_weak(
                previous,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => previous = actual,
            }
        }
    }

    fn record_writer_error(&self, error: &io::Error) {
        self.writer_errors.fetch_add(1, Ordering::Relaxed);
        self.last_errno
            .store(error.raw_os_error().unwrap_or(0), Ordering::Relaxed);
    }

    fn telemetry(&self) -> Telemetry {
        Telemetry {
            queued_blocks: self.queued_blocks.load(Ordering::Relaxed),
            max_queued_blocks: self.max_queued_blocks.load(Ordering::Relaxed),
            dropped_blocks: self.dropped_blocks.load(Ordering::Relaxed),
            dropped_frames: self.dropped_frames.load(Ordering::Relaxed),
            stale_blocks: self.stale_blocks.load(Ordering::Relaxed),
            underruns: self.underruns.load(Ordering::Relaxed),
            writer_errors: self.writer_errors.load(Ordering::Relaxed),
            mixer_errors: self.mixer_errors.load(Ordering::Relaxed),
            written_frames: self.written_frames.load(Ordering::Relaxed),
            unsupported_streams: self.unsupported_streams.load(Ordering::Relaxed),
            malformed_samples: self.malformed_samples.load(Ordering::Relaxed),
            last_errno: self.last_errno.load(Ordering::Relaxed),
        }
    }
}

impl EchoAlsaSink {
    pub fn open() -> io::Result<Self> {
        let mut mixer = Mixer::open()?;
        let pcm = Pcm::open_default()?;
        // Configure only after owning/preparing PCM; a busy device must not be rerouted.
        mixer.configure_output()?;
        let mixer = Arc::new(Mutex::new(mixer));
        let (sender, receiver) = sync_channel(QUEUE_PERIODS);
        let shared = Arc::new(Shared::new());
        let writer_shared = Arc::clone(&shared);
        let writer_mixer = Arc::clone(&mixer);
        let writer = thread::Builder::new()
            .name("echo-alsa-writer".to_owned())
            .spawn(move || writer_loop(pcm, receiver, writer_shared, writer_mixer))?;

        Ok(Self {
            sender,
            shared,
            mixer,
            writer: Some(writer),
        })
    }

    pub fn audio_handler(&self) -> EchoAudioHandler {
        EchoAudioHandler {
            sender: self.sender.clone(),
            shared: Arc::clone(&self.shared),
            mixer: Arc::clone(&self.mixer),
        }
    }

    pub fn telemetry(&self) -> Telemetry {
        self.shared.telemetry()
    }

    /// Reads the system mixer, including changes made outside AirPlay.
    /// Performs control I/O; do not call this from the PCM callback.
    pub fn master_volume(&self) -> io::Result<MasterVolume> {
        self.mixer
            .lock()
            .map_err(|_| io::Error::other("Echo master mixer lock poisoned"))?
            .volume()
    }

    /// Adjusts the ALSA master in 1 dB steps within the AirPlay -30..0 dB range.
    /// The bottom step mutes. Reads, updates, and reads back under the same mixer lock;
    /// never call from the PCM callback. Source notification is the caller's responsibility.
    pub fn adjust_master_volume(&self, steps: i8) -> io::Result<MasterVolume> {
        let mut mixer = self
            .mixer
            .lock()
            .map_err(|_| io::Error::other("Echo master mixer lock poisoned"))?;
        let current = mixer.volume()?;
        if steps == 0 {
            return Ok(current);
        }
        mixer.set_db(stepped_volume_db(current, steps))?;
        mixer.volume()
    }

    pub fn shutdown(&mut self) {
        let Some(writer) = self.writer.take() else {
            return;
        };

        self.shared.stopping.store(true, Ordering::Release);
        let _ = self.sender.send(Message::Stop);
        if writer.join().is_err() {
            self.shared.writer_errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn stepped_volume_db(volume: MasterVolume, steps: i8) -> f32 {
    let db = volume
        .gain_db
        .into_iter()
        .zip(volume.enabled)
        .filter_map(|(db, enabled)| enabled.then_some(db))
        .fold(f32::NEG_INFINITY, f32::max)
        .clamp(-30.0, 0.0);
    let next = (db + f32::from(steps)).clamp(-30.0, 0.0);
    if next <= -30.0 { -144.0 } else { next }
}

impl Drop for EchoAlsaSink {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl EchoAudioHandler {
    pub fn telemetry(&self) -> Telemetry {
        self.shared.telemetry()
    }
}

impl AudioHandler for EchoAudioHandler {
    fn on_volume(&self, volume: f32) {
        // This callback runs on the RTSP thread, never in audio_process.
        let result = self
            .mixer
            .lock()
            .map_err(|_| io::Error::other("Echo master mixer lock poisoned"))
            .and_then(|mut mixer| mixer.set_db(volume));
        if let Err(error) = result {
            self.shared.mixer_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("Echo master volume: {error}");
        }
    }

    fn audio_init(&self, format: AudioFormat) -> Box<dyn AudioSession> {
        let valid_format = format.sample_rate == SAMPLE_RATE
            && u32::from(format.channels) == CHANNELS
            && format.bits == 32;
        let generation = if valid_format {
            self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1
        } else {
            self.shared
                .unsupported_streams
                .fetch_add(1, Ordering::Relaxed);
            self.shared.generation.load(Ordering::Acquire)
        };

        Box::new(EchoAudioSession {
            sender: self.sender.clone(),
            shared: Arc::clone(&self.shared),
            generation,
            valid_format,
            // ponytail: a fixed bounded stdlib queue is enough for the POC; add a reusable buffer pool only if callback allocation is measured.
            partial: Vec::with_capacity(BLOCK_SAMPLES),
        })
    }
}

impl AudioSession for EchoAudioSession {
    fn audio_process(&mut self, samples: &[f32]) {
        if !self.valid_format
            || self.shared.stopping.load(Ordering::Acquire)
            || self.generation != self.shared.generation.load(Ordering::Acquire)
        {
            self.shared.dropped_frames.fetch_add(
                (samples.len() / CHANNELS as usize) as u64,
                Ordering::Relaxed,
            );
            return;
        }

        let complete_samples = samples.len() - samples.len() % CHANNELS as usize;
        if complete_samples != samples.len() {
            self.shared
                .malformed_samples
                .fetch_add((samples.len() - complete_samples) as u64, Ordering::Relaxed);
        }

        let mut remaining = &samples[..complete_samples];
        while !remaining.is_empty() {
            let available = BLOCK_SAMPLES - self.partial.len();
            let copied = available.min(remaining.len());
            self.partial.extend_from_slice(&remaining[..copied]);
            remaining = &remaining[copied..];

            if self.partial.len() == BLOCK_SAMPLES {
                let block = std::mem::replace(&mut self.partial, Vec::with_capacity(BLOCK_SAMPLES));
                self.enqueue(block);
            }
        }
    }

    fn audio_flush(&mut self) {
        self.shared.dropped_frames.fetch_add(
            (self.partial.len() / CHANNELS as usize) as u64,
            Ordering::Relaxed,
        );
        self.partial.clear();
        let next_generation = self.generation.wrapping_add(1);
        if self
            .shared
            .generation
            .compare_exchange(
                self.generation,
                next_generation,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            self.generation = next_generation;
        }
    }
}

impl EchoAudioSession {
    fn enqueue(&mut self, samples: Vec<f32>) {
        let depth = self.shared.queued_blocks.fetch_add(1, Ordering::Relaxed) + 1;
        self.shared.observe_queue_depth(depth);

        match self.sender.try_send(Message::Audio(AudioBlock {
            generation: self.generation,
            samples,
        })) {
            Ok(()) => {}
            Err(TrySendError::Full(Message::Audio(block))) => {
                self.shared.queued_blocks.fetch_sub(1, Ordering::Relaxed);
                self.shared.dropped_blocks.fetch_add(1, Ordering::Relaxed);
                self.shared.dropped_frames.fetch_add(
                    (block.samples.len() / CHANNELS as usize) as u64,
                    Ordering::Relaxed,
                );
                self.partial = block.samples;
                self.partial.clear();
            }
            Err(TrySendError::Disconnected(Message::Audio(block))) => {
                self.shared.queued_blocks.fetch_sub(1, Ordering::Relaxed);
                self.shared.dropped_blocks.fetch_add(1, Ordering::Relaxed);
                self.shared.dropped_frames.fetch_add(
                    (block.samples.len() / CHANNELS as usize) as u64,
                    Ordering::Relaxed,
                );
            }
            Err(TrySendError::Full(Message::Stop) | TrySendError::Disconnected(Message::Stop)) => {
                unreachable!("only audio blocks are sent from the callback")
            }
        }
    }
}

fn writer_loop(
    mut pcm: Pcm,
    receiver: Receiver<Message>,
    shared: Arc<Shared>,
    mixer: Arc<Mutex<Mixer>>,
) {
    let mut pcm_samples = vec![0_i16; BLOCK_SAMPLES];

    while let Ok(message) = receiver.recv() {
        let Message::Audio(block) = message else {
            break;
        };
        shared.queued_blocks.fetch_sub(1, Ordering::Relaxed);

        if block.generation != shared.generation.load(Ordering::Acquire) {
            shared.stale_blocks.fetch_add(1, Ordering::Relaxed);
            shared.dropped_frames.fetch_add(
                (block.samples.len() / CHANNELS as usize) as u64,
                Ordering::Relaxed,
            );
            continue;
        }
        if block.samples.len() != BLOCK_SAMPLES {
            shared.record_writer_error(&io::Error::new(
                io::ErrorKind::InvalidData,
                format!("writer received {} samples", block.samples.len()),
            ));
            continue;
        }

        for (output, input) in pcm_samples.iter_mut().zip(block.samples) {
            *output = f32_to_s16(input);
        }

        match write_with_recovery(&mut pcm, &pcm_samples, &shared, &mixer) {
            Ok(()) => {
                shared
                    .written_frames
                    .fetch_add(PERIOD_FRAMES as u64, Ordering::Relaxed);
            }
            Err(error) => shared.record_writer_error(&error),
        }
    }

    let _ = pcm.drop_stream();
    if let Err(error) = mixer
        .lock()
        .map_err(|_| io::Error::other("Echo mixer lock poisoned"))
        .and_then(|mut mixer| mixer.disable_amp())
    {
        shared.record_writer_error(&error);
    }
}

fn write_with_recovery(
    pcm: &mut Pcm,
    samples: &[i16],
    shared: &Shared,
    mixer: &Mutex<Mixer>,
) -> io::Result<()> {
    match pcm.write_period(samples) {
        Ok(()) => Ok(()),
        Err(error) if is_xrun(&error) => {
            shared.underruns.fetch_add(1, Ordering::Relaxed);
            pcm.prepare()?;
            mixer
                .lock()
                .map_err(|_| io::Error::other("Echo mixer lock poisoned"))?
                .release_gpio_mute()?;
            pcm.write_period(samples)
        }
        Err(error) => Err(error),
    }
}

fn f32_to_s16(sample: f32) -> i16 {
    if !sample.is_finite() {
        0
    } else if sample <= -1.0 {
        i16::MIN
    } else if sample >= 1.0 {
        i16::MAX
    } else {
        (sample * i16::MAX as f32).round() as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_steps_use_actual_gain_and_hardware_mute() {
        for (gain_db, enabled, step, expected) in [
            ([-20.5; 2], [true; 2], 1, -19.5),
            ([-20.5; 2], [true; 2], -1, -21.5),
            ([0.0; 2], [true; 2], 1, 0.0),
            ([-29.0; 2], [true; 2], -1, -144.0),
            ([0.0; 2], [false; 2], 1, -29.0),
            ([0.0; 2], [false; 2], -1, -144.0),
            ([0.0, -20.0], [false, true], 1, -19.0),
        ] {
            assert_eq!(
                stepped_volume_db(MasterVolume { gain_db, enabled }, step),
                expected
            );
        }
    }

    #[test]
    fn conversion_clamps_and_silences_non_finite_input() {
        assert_eq!(f32_to_s16(-1.0), i16::MIN);
        assert_eq!(f32_to_s16(1.0), i16::MAX);
        assert_eq!(f32_to_s16(0.0), 0);
        assert_eq!(f32_to_s16(0.5), 16_384);
        assert_eq!(f32_to_s16(f32::NAN), 0);
    }

    #[test]
    fn stale_session_flush_cannot_take_over_a_newer_stream() {
        let shared = Arc::new(Shared::new());
        shared.generation.store(2, Ordering::Release);
        let (sender, receiver) = sync_channel(1);
        let mut session = EchoAudioSession {
            sender,
            shared: Arc::clone(&shared),
            generation: 1,
            valid_format: true,
            partial: Vec::with_capacity(BLOCK_SAMPLES),
        };

        session.audio_flush();
        session.audio_process(&vec![0.0; BLOCK_SAMPLES]);

        assert_eq!(shared.generation.load(Ordering::Acquire), 2);
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn full_queue_drops_the_newest_period_and_counts_it() {
        let shared = Arc::new(Shared::new());
        shared.generation.store(1, Ordering::Release);
        let (sender, receiver) = sync_channel(1);
        let mut session = EchoAudioSession {
            sender,
            shared: Arc::clone(&shared),
            generation: 1,
            valid_format: true,
            partial: Vec::with_capacity(BLOCK_SAMPLES),
        };

        session.audio_process(&vec![0.25; BLOCK_SAMPLES]);
        session.audio_process(&vec![0.5; BLOCK_SAMPLES]);

        assert!(matches!(receiver.try_recv(), Ok(Message::Audio(_))));
        assert_eq!(shared.dropped_blocks.load(Ordering::Acquire), 1);
        assert_eq!(
            shared.dropped_frames.load(Ordering::Acquire),
            PERIOD_FRAMES as u64
        );
    }
}

//! A push-to-talk recording: the microphone is open only while it lasts.

use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use crownui_audio::{
    Enhancement, FrameConsumer, Microphone, MicrophoneOptions, MicrophoneStream, SAMPLE_RATE,
    StreamState, sources,
};

use super::envelope::WaveEnvelope;
use super::source_choice::capture_source;
use crate::state::{Shared, push_wave};

/// RNNoise-class suppression: it takes out steady fan and mains hum without the
/// spectral smearing heavier models leave behind, which costs the recognizer
/// more than the noise did.
const ENHANCEMENT: Enhancement = Enhancement::Light;
/// Hard cap on the recording (5 min at 48 kHz ≈ 55 MB of f32), a whole number
/// of frames.
const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 300;
const EXPECTED_SAMPLES: usize = SAMPLE_RATE as usize * 8;
/// Well inside the one second of frames the capture ring holds.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct Recording {
    microphone: MicrophoneStream,
    collector: JoinHandle<Vec<f32>>,
}

impl Recording {
    /// Close the microphone and return what it captured: mono at
    /// [`SAMPLE_RATE`].
    pub fn stop(self) -> Vec<f32> {
        log_outcome(&self.microphone.state().borrow());
        drop(self.microphone);
        self.collector.join().unwrap_or_else(|_| {
            log::error!("audio: the collector thread panicked");
            Vec::new()
        })
    }
}

/// Open the microphone named in `input.ron` (`None` for the system default)
/// and start capturing. Loudness is streamed into `shared` for the overlay.
pub fn start(shared: Shared, wanted: Option<&str>) -> Result<Recording> {
    let mut options = MicrophoneOptions::default().enhancement(ENHANCEMENT);
    if let Some(source) = capture_source(wanted, &sources().borrow()) {
        log::info!("audio: capturing from '{}'", source.as_str());
        options = options.source(source);
    }
    let mut microphone = Microphone::open(options).context("open the microphone")?;
    let frames = microphone
        .take_frames()
        .context("the microphone's frames were already taken")?;
    let collector = thread::Builder::new()
        .name("audio".into())
        .spawn(move || collect(frames, &shared))
        .context("spawn the audio collector thread")?;
    Ok(Recording {
        microphone,
        collector,
    })
}

/// Start following the PipeWire graph, so the first recording can already tell
/// a connected microphone from an unplugged one.
pub fn watch_sources() {
    drop(sources());
}

/// Drain frames until the capture thread is gone, which is when the
/// [`MicrophoneStream`] is dropped.
fn collect(mut frames: FrameConsumer, shared: &Shared) -> Vec<f32> {
    let mut samples = Vec::with_capacity(EXPECTED_SAMPLES);
    let mut envelope = WaveEnvelope::default();
    loop {
        let finished = frames.is_abandoned();
        while let Ok(frame) = frames.pop() {
            if samples.len() < MAX_SAMPLES {
                samples.extend_from_slice(&frame.samples);
            }
            if let Some(loudness) = envelope.push(&frame.samples) {
                push_wave(shared, loudness);
            }
        }
        if finished {
            return samples;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn log_outcome(state: &StreamState) {
    match state {
        StreamState::Live {
            enhanced_by: Some(backend),
        } => log::info!("audio: enhanced by {backend}"),
        StreamState::Live { enhanced_by: None } => log::info!("audio: raw microphone"),
        StreamState::Unenhanced(reason) => log::info!("audio: raw microphone, {reason}"),
        StreamState::Starting => log::warn!("audio: the microphone never started"),
        StreamState::Failed(reason) => log::error!("audio: capture failed: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::resample_to_16k;
    use crate::state::new_shared;

    const RECORDING_LENGTH: Duration = Duration::from_secs(3);

    #[test]
    #[ignore = "needs PipeWire and a microphone"]
    fn records_audible_16k_audio_from_the_default_microphone() -> Result<()> {
        let recording = start(new_shared(), None)?;
        thread::sleep(RECORDING_LENGTH);
        let samples_16k = resample_to_16k(&recording.stop(), SAMPLE_RATE);

        let seconds = samples_16k.len() as f32 / 16_000.0;
        let peak = samples_16k.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        println!("{seconds:.2}s at 16 kHz, peak {peak:.4}");
        assert!(seconds > RECORDING_LENGTH.as_secs_f32() - 0.5);
        assert!(peak > 0.0, "the microphone delivered only silence");
        Ok(())
    }
}

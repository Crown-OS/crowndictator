//! The loudness curve the overlay's waveform is drawn from.

use crownui_audio::FRAME_SAMPLES;

/// Samples behind each waveform point: 20 ms at 48 kHz, so the overlay's
/// 56-slot history spans a little over a second.
const SAMPLES_PER_POINT: usize = FRAME_SAMPLES * 2;
/// Speech RMS sits well below full scale, so it is lifted before the curve.
const LOUDNESS_GAIN: f32 = 9.0;
const LOUDNESS_EXPONENT: f32 = 0.7;

/// Folds captured audio into one perceptual loudness point per 20 ms.
#[derive(Debug, Default)]
pub struct WaveEnvelope {
    energy: f32,
    samples: usize,
}

impl WaveEnvelope {
    /// Adds `samples` and returns a loudness in `0.0..=1.0` whenever a point
    /// is complete.
    pub fn push(&mut self, samples: &[f32]) -> Option<f32> {
        self.energy += samples.iter().map(|sample| sample * sample).sum::<f32>();
        self.samples += samples.len();
        if self.samples < SAMPLES_PER_POINT {
            return None;
        }
        let rms = (self.energy / self.samples as f32).sqrt();
        *self = Self::default();
        Some((rms * LOUDNESS_GAIN).powf(LOUDNESS_EXPONENT).min(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SILENCE: [f32; FRAME_SAMPLES] = [0.0; FRAME_SAMPLES];

    #[test]
    fn a_point_takes_two_frames() {
        let mut envelope = WaveEnvelope::default();
        assert_eq!(envelope.push(&SILENCE), None);
        assert_eq!(envelope.push(&SILENCE), Some(0.0));
        assert_eq!(envelope.push(&SILENCE), None, "starts over after a point");
    }

    #[test]
    fn loud_audio_saturates_at_one() {
        let mut envelope = WaveEnvelope::default();
        let full_scale = [1.0; FRAME_SAMPLES];
        envelope.push(&full_scale);
        assert_eq!(envelope.push(&full_scale), Some(1.0));
    }

    #[test]
    fn quiet_speech_lands_mid_scale() {
        let mut envelope = WaveEnvelope::default();
        let quiet = [0.05; FRAME_SAMPLES];
        envelope.push(&quiet);
        let level = envelope.push(&quiet).unwrap_or_default();
        assert!((0.4..0.8).contains(&level), "{level}");
    }
}

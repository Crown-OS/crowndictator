//! Offline resampling to the 16 kHz the speech model expects.

/// The sample rate the speech model is trained on.
const ASR_SAMPLE_RATE: u32 = 16_000;

/// Resample mono audio to 16 kHz with a windowed-sinc kernel.
pub fn resample_to_16k(samples: &[f32], rate: u32) -> Vec<f32> {
    const TARGET: f64 = ASR_SAMPLE_RATE as f64;
    if rate == ASR_SAMPLE_RATE || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = rate as f64 / TARGET; // input samples per output sample
    let out_len = (samples.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);

    // Low-pass cutoff at the lower Nyquist, slightly under to reduce aliasing.
    let cutoff = (TARGET / rate as f64).min(1.0) * 0.92;
    let half_taps = if ratio > 1.0 {
        (12.0 * ratio).ceil() as isize
    } else {
        12
    };

    for i in 0..out_len {
        let center = i as f64 * ratio;
        let c0 = center.floor() as isize;
        let mut sum = 0.0f64;
        let mut wsum = 0.0f64;
        for k in (c0 - half_taps)..=(c0 + half_taps) {
            if k < 0 || k as usize >= samples.len() {
                continue;
            }
            let x = (k as f64 - center) * cutoff;
            let sinc = if x.abs() < 1e-9 {
                1.0
            } else {
                (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
            };
            // Hann window over the kernel extent.
            let w = 0.5
                + 0.5
                    * (std::f64::consts::PI * (k as f64 - center) / (half_taps as f64 + 1.0)).cos();
            let coeff = sinc * w * cutoff;
            sum += samples[k as usize] as f64 * coeff;
            wsum += coeff;
        }
        out.push(if wsum.abs() > 1e-12 {
            (sum / wsum) as f32
        } else {
            0.0
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAPTURE_RATE: u32 = 48_000;

    fn sine(frequency: f32, rate: u32, seconds: f32) -> Vec<f32> {
        let length = (rate as f32 * seconds) as usize;
        (0..length)
            .map(|index| (std::f32::consts::TAU * frequency * index as f32 / rate as f32).sin())
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn audio_already_at_16k_passes_through() {
        let samples = sine(440.0, ASR_SAMPLE_RATE, 0.1);
        assert_eq!(resample_to_16k(&samples, ASR_SAMPLE_RATE), samples);
    }

    #[test]
    fn capture_rate_audio_shrinks_threefold() {
        let samples = sine(440.0, CAPTURE_RATE, 1.0);
        assert_eq!(
            resample_to_16k(&samples, CAPTURE_RATE).len(),
            samples.len() / 3
        );
    }

    #[test]
    fn speech_band_tones_keep_their_level() {
        let resampled = resample_to_16k(&sine(1_000.0, CAPTURE_RATE, 0.5), CAPTURE_RATE);
        assert!((rms(&resampled) - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.02);
    }

    #[test]
    fn tones_above_the_new_nyquist_are_filtered_out() {
        let resampled = resample_to_16k(&sine(12_000.0, CAPTURE_RATE, 0.5), CAPTURE_RATE);
        assert!(rms(&resampled) < 0.05);
    }
}

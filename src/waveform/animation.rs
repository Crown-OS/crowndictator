use std::sync::PoisonError;
use std::time::Duration;

use super::metrics::BARS;
use crate::state::{Phase, Shared, WAVE_N, set_phase, wave_snapshot};

/// How long the success/error flash stays before the pill retracts.
const RESULT_FLASH_SECS: f32 = 0.55;
/// Largest step taken at once, so a stall never flings the springs.
const MAX_STEP_SECS: f32 = 0.05;
/// Below this much presence a hidden pill is gone: nothing drawn, no frames.
const RETRACTED: f32 = 0.004;

/// The pill's motion, advanced once per display frame.
pub struct Overlay {
    pub phase: Phase,
    /// Seconds on the frame clock, for the synthetic motion.
    pub clock: f32,
    /// 0 = fully retracted, 1 = fully shown.
    pub presence: f32,
    /// Smoothed per-bar heights, 0..1.
    pub bars: [f32; BARS],
    was_visible: bool,
    last_frame: Duration,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            phase: Phase::Hidden,
            clock: 0.0,
            presence: 0.0,
            bars: [0.0; BARS],
            was_visible: false,
            last_frame: Duration::ZERO,
        }
    }
}

/// Peak of the waveform history that falls into bar `i`.
fn bar_peak(wave: &[f32; WAVE_N], i: usize) -> f32 {
    let seg = WAVE_N as f32 / BARS as f32;
    let a = (i as f32 * seg) as usize;
    let b = (((i + 1) as f32 * seg).ceil() as usize).clamp(a + 1, WAVE_N);
    wave[a..b].iter().copied().fold(0.0, f32::max)
}

impl Overlay {
    pub fn is_retracted(&self) -> bool {
        self.presence < RETRACTED && self.phase == Phase::Hidden
    }

    /// Advances to frame time `now`; returns whether another frame is needed.
    pub fn step(&mut self, shared: &Shared, now: Duration) -> bool {
        let dt = (now.saturating_sub(self.last_frame))
            .as_secs_f32()
            .min(MAX_STEP_SECS);
        self.last_frame = now;
        self.clock = now.as_secs_f32();

        let (phase, since, wave_raw) = {
            let s = shared.lock().unwrap_or_else(PoisonError::into_inner);
            (
                s.phase,
                s.phase_since.elapsed().as_secs_f32(),
                wave_snapshot(&s),
            )
        };
        let phase = if matches!(phase, Phase::Success | Phase::Error) && since > RESULT_FLASH_SECS {
            set_phase(shared, Phase::Hidden);
            Phase::Hidden
        } else {
            phase
        };
        self.phase = phase;
        let visible = phase != Phase::Hidden;

        if visible && !self.was_visible {
            self.bars = [0.0; BARS];
        }
        self.was_visible = visible;

        let target = if visible { 1.0 } else { 0.0 };
        let rate = if visible { 16.0 } else { 11.0 };
        self.presence += (target - self.presence) * (1.0 - (-dt * rate).exp());

        if self.is_retracted() {
            return false;
        }
        self.step_bars(&wave_raw, dt);
        true
    }

    /// Listening follows the mic; the other phases get synthetic motion so
    /// the pill never looks frozen.
    fn step_bars(&mut self, wave_raw: &[f32; WAVE_N], dt: f32) {
        let t = self.clock;
        for (i, bar) in self.bars.iter_mut().enumerate() {
            let s = i as f32 / (BARS - 1) as f32;
            let want = match self.phase {
                Phase::Thinking => {
                    // A soft bump scanning left to right.
                    let pos = (t * 0.85) % 1.4 - 0.2;
                    let d = (s - pos) / 0.18;
                    0.06 + 0.9 * (-d * d).exp()
                }
                Phase::Error => 0.10 + 0.06 * (t * 9.0 + s * 6.0).sin(),
                Phase::Success | Phase::Hidden => 0.0,
                Phase::Listening => bar_peak(wave_raw, i),
            };
            // Fast attack, slower release, so peaks pop but don't flicker.
            let k = if want > *bar { 30.0 } else { 11.0 };
            *bar += (want - *bar) * (1.0 - (-dt * k).exp());
        }
    }
}

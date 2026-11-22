//! Captures every overlay phase for the CrownOS-Design Figma file. Run both appearances with
//! `../crownui/capture/scripts/capture-app.sh .`; output lands in `$CROWN_DESIGN_DIR`.

#[path = "../src/state.rs"]
mod state;
#[allow(dead_code)]
#[path = "../src/util/mod.rs"]
mod util;
#[path = "../src/waveform/mod.rs"]
mod waveform;

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use anyhow::Context;
use capture::{CaptureApp, Carrier};
use state::{Phase, Shared, WAVE_N, push_wave, set_phase};
use util::wake::Wake;
use view::{TaskExecutor, TaskFuture, TaskId};

const GROUP: &str = "Dictation pill";
/// Keeps a result flash on screen while the capture settles on its simulated clock.
const FLASH_HOLD: Duration = Duration::from_secs(3600);

/// One spoken syllable on the waveform history: first point, length in points, loudness.
struct Syllable {
    start: usize,
    points: usize,
    loudness: f32,
}

impl Syllable {
    const fn new(start: usize, points: usize, loudness: f32) -> Self {
        Self {
            start,
            points,
            loudness,
        }
    }
}

/// "Send it to the team by Friday", give or take: bursts of varied loudness with a short pause.
const SPEECH: [Syllable; 7] = [
    Syllable::new(0, 6, 0.35),
    Syllable::new(7, 9, 0.8),
    Syllable::new(17, 7, 0.55),
    Syllable::new(25, 5, 0.22),
    Syllable::new(33, 10, 0.95),
    Syllable::new(44, 6, 0.6),
    Syllable::new(51, 5, 0.72),
];
/// Where the thinking bump scans to, 0 = left end of the strip, 1 = right end.
const THINKING_BUMP_AT: f32 = 0.5;
const THINKING_SPEED: f32 = 0.85;
const THINKING_PERIOD: f32 = 1.4;
const THINKING_LEAD: f32 = 0.2;
/// How long `capture_state` settles a pill that never stops moving.
const SETTLE_SECS: f32 = 10.0;

/// Holds the wake task so the harness resolves it, as a worker waking the UI would.
struct HeldTasks(mpsc::Sender<TaskId>);

impl TaskExecutor for HeldTasks {
    fn spawn(&mut self, id: TaskId, _future: TaskFuture) {
        let _ = self.0.send(id);
    }

    fn cancel(&mut self, _id: TaskId) {}
}

/// The loudness of waveform point `index` within [`SPEECH`], with a little roughness.
fn speech_sample(index: usize) -> f32 {
    SPEECH
        .iter()
        .find(|syllable| (syllable.start..syllable.start + syllable.points).contains(&index))
        .map_or(0.0, |syllable| {
            let progress = (index - syllable.start) as f32 + 0.5;
            let arc = (std::f32::consts::PI * progress / syllable.points as f32)
                .sin()
                .powf(0.7);
            let roughness = 0.85 + 0.15 * (index as f32 * 2.3).sin();
            syllable.loudness * arc * roughness
        })
}

fn hold_flash(shared: &Shared, phase: Phase) {
    set_phase(shared, phase);
    if let Ok(mut viz) = shared.lock() {
        viz.phase_since = Instant::now() + FLASH_HOLD;
    }
}

/// Mounts the overlay root, lets `prepare` set the shared state, then wakes the UI.
fn open(prepare: impl FnOnce(&Shared)) -> anyhow::Result<CaptureApp> {
    let shared = state::new_shared();
    let mut app = capture::mount_app(
        waveform::overlay(shared.clone(), Wake::default(), || {}),
        &waveform::app_config(),
        None,
    )?;
    let (sender, started) = mpsc::channel();
    app.ui().set_task_executor(Box::new(HeldTasks(sender)));
    prepare(&shared);
    let wake = started
        .try_recv()
        .context("the overlay did not wait for a wake")?;
    app.ui().complete_task(wake, Box::new(()));
    Ok(app)
}

/// Advances the clock so the scan sits at [`THINKING_BUMP_AT`] once the capture has settled.
fn align_thinking_bump(app: &mut CaptureApp) {
    let settled_at = app.inline().now().as_secs_f32() + SETTLE_SECS;
    let scanned = (settled_at * THINKING_SPEED) % THINKING_PERIOD;
    let wanted = THINKING_BUMP_AT + THINKING_LEAD;
    let wait_secs = (wanted - scanned).rem_euclid(THINKING_PERIOD) / THINKING_SPEED;
    app.advance_ms((wait_secs * 1000.0).round() as u64);
}

fn main() -> anyhow::Result<()> {
    let mut carrier = Carrier::new(
        "Dictator",
        "crowndictator: the push-to-talk dictation pill on its bottom-anchored overlay surface, captured headlessly from the current source.",
    );

    let mut app = open(|shared| set_phase(shared, Phase::Listening))?;
    carrier.capture_state(
        &mut app,
        GROUP,
        "Listening, silence",
        "Shortcut held, microphone open, nothing said yet: the strip rests as a dotted baseline",
    )?;
    drop(app);

    let mut app = open(|shared| {
        set_phase(shared, Phase::Listening);
        (0..WAVE_N).for_each(|index| push_wave(shared, speech_sample(index)));
    })?;
    carrier.capture_state(
        &mut app,
        GROUP,
        "Listening, speaking",
        "Live waveform of the last second of speech, newest on the right",
    )?;
    drop(app);

    let mut app = open(|shared| set_phase(shared, Phase::Thinking))?;
    align_thinking_bump(&mut app);
    carrier.capture_state(
        &mut app,
        GROUP,
        "Transcribing",
        "Shortcut released, Parakeet is running: a soft bump scans the strip and the dots chase",
    )?;
    drop(app);

    let mut app = open(|shared| hold_flash(shared, Phase::Success))?;
    carrier.capture_state(
        &mut app,
        GROUP,
        "Inserted",
        "Text typed into the focused window: the strip settles flat before the pill retracts",
    )?;
    drop(app);

    let mut app = open(|shared| hold_flash(shared, Phase::Error))?;
    carrier.capture_state(
        &mut app,
        GROUP,
        "Error",
        "No microphone, a failed transcription or injection: the pill flashes red, then retracts",
    )?;
    drop(app);

    let path = carrier.write(None)?;
    println!("{}", path.display());
    capture::finish()
}

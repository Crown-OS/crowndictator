//! crowndictator — push-to-talk dictation for Wayland.
//!
//! Hold the shortcut, speak, release: the recording is transcribed locally
//! with NVIDIA Parakeet TDT 0.6B v2 (ONNX Runtime, CUDA when available)
//! and typed into the focused window. A small waveform pill rendered
//! through crownui sits at the bottom of the screen while it listens
//! and thinks.
//!
//! The shortcut — along with the switch that turns the whole thing off, which
//! microphone to record from, and whether to use the GPU — comes from
//! `~/.config/crownos/input.ron`, which the settings panel's Input page writes
//! and this daemon follows live. See [`settings`].
//!
//! Debug modes:
//!   crowndictator --demo              cycle overlay states with fake audio
//!   crowndictator --transcribe f.wav  run the ASR pipeline on a wav file
//!   crowndictator --cpu               skip CUDA even if available

mod asr;
mod audio;
mod controller;
mod hotkey;
mod inject;
mod settings;
mod state;
mod util;
mod wav;
mod waveform;

use anyhow::{Context, Result, anyhow};
use crownui::AppConfig;
use crownui::prelude::{
    Anchor, KeyboardInteractivity, Layer, LayerExtent, LayerOptions, TRANSPARENT,
};

use util::wake::Wake;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let force_cpu = args.iter().any(|a| a == "--cpu");
    let demo = args.iter().any(|a| a == "--demo");

    if let Some(i) = args.iter().position(|a| a == "--transcribe") {
        let path = args
            .get(i + 1)
            .ok_or_else(|| anyhow!("--transcribe needs a wav path"))?;
        let samples = wav::read_wav_16k(std::path::Path::new(path))?;
        log::info!("loaded {:.1}s of audio", samples.len() as f32 / 16_000.0);
        // One shot, so the backend is decided here and now: the flag, else
        // whatever the user configured for the daemon.
        let prefer_gpu = !force_cpu && settings::load().dictation_gpu;
        let mut engine = asr::Engine::load(prefer_gpu)?;
        log::info!(
            "engine backend: {}",
            if engine.gpu { "CUDA" } else { "CPU" }
        );
        println!("{}", engine.transcribe(&samples)?);
        return Ok(());
    }

    let shared = state::new_shared();
    let wake = Wake::default();
    let start_workers = {
        let shared = shared.clone();
        let wake = wake.clone();
        move || {
            if demo {
                controller::spawn_demo(shared, wake);
            } else {
                // The controller says what it is listening for once it has read
                // `input.ron` — there is no shortcut to name from out here.
                controller::spawn(shared, wake, controller::Options { force_cpu });
            }
        }
    };

    // The overlay is purely decorative: its root takes no pointer input, so
    // clicks pass through the surface.
    let layer = LayerOptions::new("crowndictator", Anchor::BOTTOM)
        .layer(Layer::Overlay)
        .width(LayerExtent::Fixed(waveform::WIN_W))
        .height(LayerExtent::Fixed(waveform::WIN_H))
        .exclusive_zone(0)
        .keyboard(KeyboardInteractivity::None);
    let config = AppConfig::new("Crown Dictator")
        .app_id("crowndictator")
        .size(waveform::WIN_W, waveform::WIN_H)
        .clear_color(TRANSPARENT)
        .accessibility(false)
        .layer(layer);
    crownui::run(waveform::overlay(shared, wake, start_workers), config)
        .context("the overlay stopped")
}

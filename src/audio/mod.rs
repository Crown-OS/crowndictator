//! Microphone capture through the CrownUI audio crate (PipeWire, enhanced by
//! crown-voiced when it runs) and offline resampling to 16 kHz mono.

mod envelope;
mod recording;
mod resample;
mod source_choice;

pub use crownui_audio::SAMPLE_RATE as CAPTURE_RATE;
pub use recording::{Recording, start, watch_sources};
pub use resample::resample_to_16k;

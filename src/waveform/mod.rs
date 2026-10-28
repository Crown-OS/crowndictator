//! The dictation overlay: a small dark pill pinned to the bottom of the
//! screen showing a live waveform, in the style of Wispr Flow.
//!
//! Layout inside the pill, left to right:
//!   • a few idle dots (they pulse while the model is thinking)
//!   • a text-cursor tick
//!   • the waveform strip — one bar per slot, oldest left, newest right.
//!     A silent slot collapses to a dot, so quiet audio reads as a dotted
//!     baseline rather than an empty gap.

mod animation;
mod metrics;
mod paint;
mod view;

pub use metrics::{WIN_H, WIN_W};
pub use view::overlay;

pub const PAD_X: f32 = 14.0;
pub const PILL_H: f32 = 54.0;
pub const PILL_R: f32 = 16.0;
/// Distance from the bottom screen edge to the bottom of the pill.
pub const BOTTOM_PAD: f32 = 22.0;

pub const DOTS: usize = 4;
pub const DOT_R: f32 = 1.5;
pub const DOT_PITCH: f32 = 7.0;
pub const DOTS_W: f32 = (DOTS - 1) as f32 * DOT_PITCH + DOT_R * 2.0;

pub const CURSOR_W: f32 = 2.0;
/// Slightly taller than the loudest bar, as in the reference.
pub const CURSOR_H: f32 = 28.0;

pub const BARS: usize = 28;
pub const BAR_W: f32 = 2.6;
pub const BAR_GAP: f32 = 2.4;
pub const BAR_PITCH: f32 = BAR_W + BAR_GAP;
pub const BAR_MAX: f32 = 26.0;
pub const STRIP_W: f32 = BARS as f32 * BAR_PITCH - BAR_GAP;

/// Space between the dot group, the cursor and the waveform strip.
pub const GAP: f32 = 9.0;
pub const PILL_W: f32 = PAD_X * 2.0 + DOTS_W + GAP + CURSOR_W + GAP + STRIP_W;

/// Surface size — the pill plus room for its shadow and the slide-in.
pub const WIN_W: u32 = PILL_W as u32 + 40;
pub const WIN_H: u32 = 100;

use crownui::ext::{DrawCommand, DrawList};
use crownui::prelude::{Color, CornerRadii, Rect, Size, Vector};

use super::animation::Overlay;
use super::metrics::{
    BAR_MAX, BAR_PITCH, BAR_W, BOTTOM_PAD, CURSOR_H, CURSOR_W, DOT_PITCH, DOT_R, DOTS, DOTS_W, GAP,
    PAD_X, PILL_H, PILL_R, PILL_W,
};
use crate::state::Phase;

const SHADOW: Color = Color::from_rgba8(0, 0, 0, 120);
/// Box-shadow blur radius: twice the Gaussian's standard deviation of 9.
const SHADOW_BLUR: f32 = 18.0;
const SHADOW_OFFSET: Vector = Vector::new(0.0, 4.0);
const SHADOW_INSET_X: f32 = 2.0;
const BODY: Color = Color::from_rgba8(30, 30, 32, 242);
const RIM: Color = Color::from_rgba8(255, 255, 255, 20);
/// How far below its resting place the pill starts sliding in from.
const SLIDE: f32 = 20.0;

/// Neutral while working, warm red on failure.
fn foreground(phase: Phase) -> (u8, u8, u8) {
    match phase {
        Phase::Error => (255, 128, 116),
        _ => (226, 226, 231),
    }
}

fn capsule(list: &mut DrawList, rect: Rect, color: Color) {
    let radius = rect.size.width.min(rect.size.height) * 0.5;
    list.fill(rect, CornerRadii::uniform(radius), color);
}

pub fn paint(overlay: &Overlay, size: Size, list: &mut DrawList) {
    if overlay.is_retracted() {
        return;
    }
    let ease = 1.0 - (1.0 - overlay.presence) * (1.0 - overlay.presence);
    let cy = size.height - BOTTOM_PAD - PILL_H * 0.5 + (1.0 - ease) * SLIDE;
    let x0 = size.width * 0.5 - PILL_W * 0.5;

    list.push(DrawCommand::SaveLayer {
        bounds: Some(Rect::from_xywh(0.0, 0.0, size.width, size.height)),
        opacity: overlay.presence.powf(1.2),
        backdrop_blur: None,
    });
    pill(list, Rect::from_xywh(x0, cy - PILL_H * 0.5, PILL_W, PILL_H));

    let (r, g, b) = foreground(overlay.phase);
    let tint = |alpha: u8| Color::from_rgba8(r, g, b, alpha);
    dots(list, overlay, x0 + PAD_X, cy, tint);

    let cursor_x = x0 + PAD_X + DOTS_W + GAP;
    capsule(
        list,
        Rect::from_xywh(cursor_x, cy - CURSOR_H * 0.5, CURSOR_W, CURSOR_H),
        tint(210),
    );
    bars(list, &overlay.bars, cursor_x + CURSOR_W + GAP, cy, tint);
    list.restore();
}

fn pill(list: &mut DrawList, body: Rect) {
    let radii = CornerRadii::uniform(PILL_R);
    let caster = Rect::from_xywh(
        body.origin.x + SHADOW_INSET_X,
        body.origin.y,
        body.size.width - SHADOW_INSET_X * 2.0,
        body.size.height,
    );
    list.shadow(caster, radii, SHADOW, SHADOW_BLUR, SHADOW_OFFSET);
    list.fill(body, radii, BODY);
    list.stroke(body, radii, 1.0, RIM);
}

/// The dots chase each other while the model is thinking.
fn dots(list: &mut DrawList, overlay: &Overlay, left: f32, cy: f32, tint: impl Fn(u8) -> Color) {
    for i in 0..DOTS {
        let alpha = if overlay.phase == Phase::Thinking {
            let pulse = (overlay.clock * 2.4 - i as f32 * 0.45).sin() * 0.5 + 0.5;
            (70.0 + 150.0 * pulse) as u8
        } else {
            110
        };
        let x = left + i as f32 * DOT_PITCH;
        capsule(
            list,
            Rect::from_xywh(x, cy - DOT_R, DOT_R * 2.0, DOT_R * 2.0),
            tint(alpha),
        );
    }
}

/// Louder bars are brighter, so the strip has depth even at rest.
fn bars(list: &mut DrawList, heights: &[f32], left: f32, cy: f32, tint: impl Fn(u8) -> Color) {
    for (i, &height) in heights.iter().enumerate() {
        let level = height.clamp(0.0, 1.0);
        let bar_h = BAR_W + (BAR_MAX - BAR_W) * level;
        capsule(
            list,
            Rect::from_xywh(left + i as f32 * BAR_PITCH, cy - bar_h * 0.5, BAR_W, bar_h),
            tint((155.0 + 100.0 * level) as u8),
        );
    }
}

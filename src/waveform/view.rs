use crownui::prelude::{Cx, Signal, Store, Styled, View, canvas, component};

use super::animation::Overlay;
use super::paint::paint;
use crate::state::Shared;
use crate::util::wake::Wake;

#[derive(Clone, Copy)]
struct Pill {
    motion: Signal<Overlay>,
    frame_pending: Store<bool>,
}

/// The overlay surface's root. `start_workers` runs on the first frame: the
/// threads it starts must wait until crownui has picked the GPU driver
/// through the process environment.
pub fn overlay(shared: Shared, wake: Wake, start_workers: impl FnOnce() + 'static) -> impl View {
    component(move |cx| {
        let pill = Pill {
            motion: cx.signal(Overlay::default()),
            frame_pending: cx.store(false),
        };
        cx.request_animation_frame(move |cx, _| {
            start_workers();
            follow(cx, pill, shared, wake);
        });
        canvas(move |runtime, size, list| {
            pill.motion
                .with(runtime, |overlay| paint(overlay, size, list))
        })
        .absolute()
        .inset(0.0)
        .pointer_events_none()
    })
}

/// Waits for the next wake, starts the frame loop, and waits again.
fn follow(cx: &mut Cx, pill: Pill, shared: Shared, wake: Wake) {
    cx.spawn(wake.clone().woken(), move |cx, ()| {
        animate(cx, pill, shared.clone());
        follow(cx, pill, shared, wake);
    });
}

/// Asks for a frame unless one is already coming.
fn animate(cx: &mut Cx, pill: Pill, shared: Shared) {
    if !pill
        .frame_pending
        .update(cx, |pending| std::mem::replace(pending, true))
    {
        cx.request_animation_frame(move |cx, now| frame(cx, pill, shared, now));
    }
}

/// Steps the pill and keeps frames coming until it has fully retracted.
fn frame(cx: &mut Cx, pill: Pill, shared: Shared, now: std::time::Duration) {
    pill.frame_pending.update(cx, |pending| *pending = false);
    let mut moving = false;
    pill.motion
        .update(cx, |overlay| moving = overlay.step(&shared, now));
    if moving {
        animate(cx, pill, shared);
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the injector milestone")
)]
pub mod text_diff;
pub mod wake;

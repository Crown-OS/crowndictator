use crownui::AppConfig;
use crownui::prelude::{
    Anchor, KeyboardInteractivity, Layer, LayerExtent, LayerOptions, TRANSPARENT,
};

use super::metrics::{WIN_H, WIN_W};

/// The overlay's layer surface: pinned to the bottom edge, above everything,
/// never taking the keyboard.
pub fn app_config() -> AppConfig {
    let layer = LayerOptions::new("crowndictator", Anchor::BOTTOM)
        .layer(Layer::Overlay)
        .width(LayerExtent::Fixed(WIN_W))
        .height(LayerExtent::Fixed(WIN_H))
        .exclusive_zone(0)
        .keyboard(KeyboardInteractivity::None);
    AppConfig::new("Crown Dictator")
        .app_id("crowndictator")
        .size(WIN_W, WIN_H)
        .clear_color(TRANSPARENT)
        .accessibility(false)
        .layer(layer)
}

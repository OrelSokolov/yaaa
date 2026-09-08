//! Shared helpers for querying the current screen (viewport) geometry.

/// Full size (width, height) of the current screen in points.
#[allow(dead_code)] // shared utility; consumers may only need some of these
pub fn size(ctx: &egui::Context) -> egui::Vec2 {
    ctx.content_rect().size()
}

/// Width of the current screen in points.
pub fn width(ctx: &egui::Context) -> f32 {
    ctx.content_rect().width()
}

/// Height of the current screen in points.
#[allow(dead_code)] // shared utility; consumers may only need some of these
pub fn height(ctx: &egui::Context) -> f32 {
    ctx.content_rect().height()
}

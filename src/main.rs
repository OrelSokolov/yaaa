#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::IconData;

mod app;
mod config;
mod constants;
mod font_setup;
mod git_status;
mod hotkeys;
mod menu;
mod terminal;
mod theme;
mod ui;

const APP_ICON: &[u8] = include_bytes!("icons/app_icon.png");

fn main() -> eframe::Result {
    env_logger::init();

    let icon = load_icon();

    // Window transparency is supported by the wgpu backend on all desktop
    // platforms in egui/eframe 0.34+. Try wgpu first for best performance,
    // and fall back to Glow (OpenGL) if it fails.
    if let Err(err) = try_run(eframe::Renderer::Wgpu, icon.clone()) {
        if let eframe::Error::Wgpu(_) = &err {
            log::warn!("Wgpu renderer failed: {err}. Falling back to Glow (OpenGL).");
            return try_run(eframe::Renderer::Glow, icon);
        }
        return Err(err);
    }

    Ok(())
}

fn try_run(renderer: eframe::Renderer, icon: IconData) -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([400.0, 300.0])
        .with_min_inner_size([300.0, 220.0])
        .with_title("YAAA byOrlov")
        .with_app_id("yaaa")
        .with_icon(icon)
        .with_transparent(true)
        .with_has_shadow(false);

    let native_options = eframe::NativeOptions {
        viewport,
        renderer,
        ..Default::default()
    };

    eframe::run_native(
        "YAAA byOrlov",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

fn load_icon() -> IconData {
    let image = image::load_from_memory(APP_ICON)
        .expect("Failed to load icon")
        .into_rgba8();
    let (width, height) = image.dimensions();

    IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

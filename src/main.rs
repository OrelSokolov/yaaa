#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::IconData;

mod app;
mod menu;

const APP_ICON: &[u8] = include_bytes!("icons/app_icon.png");

fn main() -> eframe::Result {
    env_logger::init();

    let icon = load_icon();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_title("Yet Another AI Agent")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Yet Another AI Agent",
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

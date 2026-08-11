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
mod system_monitor;
mod terminal;
mod theme;
mod ui;

const APP_ICON: &[u8] = include_bytes!("icons/app_icon.png");

fn main() -> eframe::Result {
    env_logger::init();

    log_startup_env();
    let icon = load_icon();
    let renderer = select_renderer();

    // On Linux + Wayland the wgpu/EGL path hits known driver/compositor bugs
    // that *hang* rather than return an error: the eglSwapBuffers deadlock in
    // libEGL_nvidia.so 595.x (Ubuntu 26.04, GNOME 50) and the winit Wayland
    // event-loop freeze (rust-windowing/winit#3551). A hang never reaches the
    // error fallback below, so `select_renderer` picks Glow up-front on that
    // combo. Everywhere else we still try wgpu first (best performance) and
    // fall back to Glow only on a hard wgpu failure.
    //
    // Window transparency works on both renderers — it is driven by the
    // `.with_transparent(true)` viewport flag in `try_run`, not by the renderer.
    if renderer == eframe::Renderer::Wgpu {
        if let Err(err) = try_run(eframe::Renderer::Wgpu, icon.clone()) {
            if let eframe::Error::Wgpu(_) = &err {
                log::warn!("Wgpu renderer failed: {err}. Falling back to Glow (OpenGL).");
                return try_run(eframe::Renderer::Glow, icon);
            }
            return Err(err);
        }
        Ok(())
    } else {
        try_run(eframe::Renderer::Glow, icon)
    }
}

/// Decide which renderer to boot, honoring an explicit override first.
///
/// `YAAA_RENDERER=glow|wgpu` forces a choice. In `auto` mode (the default) we
/// prefer Glow on Linux under a Wayland session to avoid the wgpu/EGL hangs
/// documented above; on every other platform we prefer wgpu.
fn select_renderer() -> eframe::Renderer {
    match std::env::var("YAAA_RENDERER")
        .ok()
        .map(|v| v.trim().to_lowercase())
        .as_deref()
    {
        Some("glow") => {
            log::info!("Renderer: Glow (forced by YAAA_RENDERER=glow)");
            return eframe::Renderer::Glow;
        }
        Some("wgpu") => {
            log::info!("Renderer: wgpu (forced by YAAA_RENDERER=wgpu)");
            return eframe::Renderer::Wgpu;
        }
        Some(other) => {
            log::warn!(
                "Unknown YAAA_RENDERER={other:?}, falling back to auto detection"
            );
        }
        None => {}
    }

    #[cfg(target_os = "linux")]
    {
        let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let on_wayland = session.trim() == "wayland";
        if on_wayland {
            log::warn!(
                "Linux + Wayland session detected: defaulting to the Glow renderer to avoid \
                 known wgpu/eglSwapBuffers hangs on NVIDIA 595.x and the winit Wayland event-loop \
                 freeze. Set YAAA_RENDERER=wgpu to force wgpu, or run under X11 \
                 (XDG_SESSION_TYPE=x11 / XWayland)."
            );
            return eframe::Renderer::Glow;
        }
    }

    log::info!("Renderer: wgpu (auto)");
    eframe::Renderer::Wgpu
}

/// Log the rendering-relevant environment once at startup so the active mode is
/// visible in `RUST_LOG=info` output. This is the fastest way to confirm which
/// combination is actually running when diagnosing a hang.
fn log_startup_env() {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    log::info!(
        "Startup env: XDG_SESSION_TYPE={session:?}, WAYLAND_DISPLAY={wayland_display:?}, \
         XDG_CURRENT_DESKTOP={desktop:?}"
    );

    #[cfg(target_os = "linux")]
    {
        // Best-effort NVIDIA driver sniff: libEGL_nvidia 595.x is the version
        // family with the confirmed eglSwapBuffers deadlock on 26.04.
        if let Ok(driver) = std::fs::read_to_string("/proc/driver/nvidia/version") {
            let first_line = driver.lines().next().unwrap_or("");
            log::info!("NVIDIA driver: {first_line}");
        }
    }
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

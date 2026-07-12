//! Compositor-side background blur behind the transparent window
//! ("frosted glass" / acrylic / vibrancy).
//!
//! Applied once on state change, not every frame. Controlled by the
//! "Background blur" toggle in Theme Settings. When enabled, the window is
//! forced transparent (see [`crate::theme::AppTheme::wants_transparency`]) so
//! the compositor backdrop shows through.

use egui::Color32;

/// RGBA tint handed to the compositor backdrop (used by e.g. Windows acrylic).
type Tint = [u8; 4];

/// Drives the OS/compositor background blur. Cheap to keep around; the actual
/// platform call only happens when the desired state differs from the last
/// successfully-applied one.
pub struct Backdrop {
    enabled: bool,
    /// Last successfully-applied `(enabled, tint)`, or `None` if not applied yet.
    last: Option<(bool, Tint)>,
}

impl Backdrop {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Synchronize the compositor backdrop with the desired state. No-op unless
    /// the state or tint changed since the last successful apply. Call every
    /// frame (e.g. from `App::ui`); the cost is a single comparison when idle.
    pub fn sync(&mut self, frame: &eframe::Frame, tint: Color32) {
        let tint_arr = [tint.r(), tint.g(), tint.b(), tint.a()];
        if self.last == Some((self.enabled, tint_arr)) {
            return;
        }
        match apply(frame, self.enabled, tint_arr) {
            ApplyResult::Applied => self.last = Some((self.enabled, tint_arr)),
            ApplyResult::Retry => {
                // Window handle not ready yet (early frames); retry next frame.
            }
            ApplyResult::Unsupported(msg) => {
                log::warn!("backdrop: {msg}");
                // Avoid retrying persistent unsupported cases every frame.
                self.last = Some((self.enabled, tint_arr));
            }
        }
    }
}

enum ApplyResult {
    Applied,
    Retry,
    Unsupported(String),
}

#[allow(clippy::needless_return, unused_variables)]
fn apply(frame: &eframe::Frame, on: bool, tint: Tint) -> ApplyResult {
    #[cfg(target_os = "windows")]
    {
        return apply_windows(frame, on, tint);
    }
    #[cfg(target_os = "macos")]
    {
        return apply_macos(frame, on);
    }
    #[cfg(target_os = "linux")]
    {
        return apply_linux(frame, on);
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        ApplyResult::Unsupported("unsupported platform".into())
    }
}

#[cfg(target_os = "windows")]
fn apply_windows(frame: &eframe::Frame, on: bool, tint: Tint) -> ApplyResult {
    use window_vibrancy::{apply_acrylic, clear_acrylic};
    let color = (tint[0], tint[1], tint[2], tint[3]);
    let res = if on {
        apply_acrylic(frame, Some(color))
    } else {
        clear_acrylic(frame)
    };
    match res {
        Ok(()) => ApplyResult::Applied,
        Err(e) => ApplyResult::Unsupported(format!("acrylic: {e}")),
    }
}

#[cfg(target_os = "macos")]
fn apply_macos(frame: &eframe::Frame, on: bool) -> ApplyResult {
    use window_vibrancy::{apply_vibrancy, clear_vibrancy, NSVisualEffectMaterial};
    let res = if on {
        apply_vibrancy(
            frame,
            NSVisualEffectMaterial::UnderWindowBackground,
            None,
            None,
        )
    } else {
        clear_vibrancy(frame).map(|_| ())
    };
    match res {
        Ok(()) => ApplyResult::Applied,
        Err(e) => ApplyResult::Unsupported(format!("vibrancy: {e}")),
    }
}

#[cfg(target_os = "linux")]
fn apply_linux(frame: &eframe::Frame, on: bool) -> ApplyResult {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let raw = match frame.window_handle() {
        Ok(h) => h.as_raw(),
        Err(e) => {
            log::debug!("backdrop: window handle not ready yet: {e}");
            return ApplyResult::Retry;
        }
    };

    let window = match raw {
        RawWindowHandle::Xlib(h) => h.window as u32,
        RawWindowHandle::Xcb(h) => h.window.get(),
        // Wayland: client-side blur is compositor-controlled (KWin/Hyprland
        // auto-blur transparent windows via their own config). Nothing to do.
        _ => return ApplyResult::Applied,
    };

    match set_x11_blur(window, on) {
        Ok(()) => ApplyResult::Applied,
        Err(e) => ApplyResult::Unsupported(format!("x11 blur: {e}")),
    }
}

/// Set/clear the KDE `_KDE_NET_WM_BLUR_BEHIND_REGION` property on an X11
/// window. An empty CARDINAL list requests whole-window blur (matching
/// `KWindowSystem::enableBlurBehind(window, true, QRegion())`); deleting the
/// property disables it. Honored by KWin; harmless on compositors that ignore
/// it (e.g. Mutter).
#[cfg(target_os = "linux")]
fn set_x11_blur(window: u32, on: bool) -> Result<(), String> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode};

    const BLUR_ATOM_NAME: &[u8] = b"_KDE_NET_WM_BLUR_BEHIND_REGION";

    let (conn, _screen) = x11rb::connect(None).map_err(|e| format!("connect: {e}"))?;

    let atom = conn
        .intern_atom(false, BLUR_ATOM_NAME)
        .map_err(|e| format!("intern_atom: {e}"))?
        .reply()
        .map_err(|e| format!("intern_atom reply: {e}"))?
        .atom;

    if on {
        // Empty CARDINAL list => blur the whole window.
        conn.change_property(
            PropMode::REPLACE,
            window,
            atom,
            AtomEnum::CARDINAL,
            32,
            0,
            &[],
        )
        .map_err(|e| format!("change_property: {e}"))?;
    } else {
        conn.delete_property(window, atom)
            .map_err(|e| format!("delete_property: {e}"))?;
    }

    conn.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

use egui::Color32;
use serde::{Deserialize, Serialize};

/// The default terminal foreground color.
pub const DEFAULT_TERMINAL_FG: Color32 = Color32::from_rgb(0xd8, 0xd8, 0xd8);

/// Application-wide theme colors and fonts. Stored in `Settings` and editable
/// through the "Theme" and "Fonts" settings windows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AppTheme {
    /// Background for the menu bar, panels, window fill and terminal.
    #[serde(with = "color32_hex")]
    pub app_bg: Color32,
    /// Opacity of the application/terminal background, from 0 to 100.
    #[serde(default = "default_opacity")]
    pub app_bg_opacity: u8,
    /// Text color used in the sidebar for group/tab names.
    #[serde(with = "color32_hex")]
    pub panel_text: Color32,
    /// Text color for the selected group in the sidebar.
    #[serde(with = "color32_hex")]
    pub panel_text_selected: Color32,
    /// Text color for a hovered group in the sidebar.
    #[serde(with = "color32_hex")]
    pub panel_text_hover: Color32,
    /// Text color for active/selected tabs in the sidebar.
    #[serde(with = "color32_hex")]
    pub tab_text: Color32,
    /// Background color for the active/selected tab in the sidebar.
    #[serde(with = "color32_hex")]
    pub tab_active_bg: Color32,
    /// Terminal foreground color.
    #[serde(with = "color32_hex")]
    pub terminal_fg: Color32,
    /// Font sizes used throughout the app.
    #[serde(default)]
    pub fonts: AppFonts,
}

fn default_opacity() -> u8 {
    100
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            app_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            app_bg_opacity: 100,
            panel_text: Color32::from_rgb(0xce, 0xce, 0xce),
            panel_text_selected: Color32::from_rgb(0xff, 0xa8, 0x00),
            panel_text_hover: Color32::from_rgb(0xe0, 0x9c, 0x00),
            tab_text: Color32::from_rgb(0xd8, 0xd8, 0xd8),
            tab_active_bg: Color32::from_rgb(0x02, 0x5f, 0x99),
            terminal_fg: DEFAULT_TERMINAL_FG,
            fonts: AppFonts::default(),
        }
    }
}

impl AppTheme {
    /// Return the effective application background color with opacity applied.
    pub fn app_bg_with_opacity(&self) -> Color32 {
        with_alpha(self.app_bg, self.app_bg_opacity)
    }

    /// Apply UI colors to the current egui visuals. This gives an immediate
    /// preview while the user is editing the theme.
    pub fn apply_to_visuals(&self, ctx: &egui::Context) {
        let mut visuals = ctx.global_style().visuals.clone();
        let app_bg = self.app_bg_with_opacity();
        visuals.panel_fill = app_bg;
        visuals.window_fill = app_bg;
        visuals.widgets.inactive.bg_fill = app_bg;
        visuals.widgets.noninteractive.bg_fill = app_bg;
        visuals.override_text_color = Some(self.panel_text);
        visuals.selection.bg_fill = self.tab_active_bg;
        visuals.selection.stroke.color = self.tab_active_bg;
        ctx.set_visuals(visuals);
    }

    /// Build the terminal theme from the configured terminal colors.
    pub fn build_terminal_theme(&self) -> egui_term::TerminalTheme {
        let mut palette = egui_term::ColorPalette::default();
        palette.foreground = color_to_hex(self.terminal_fg);
        palette.background = color_to_hex(self.app_bg_with_opacity());
        egui_term::TerminalTheme::new(Box::new(palette))
    }

    /// Build the terminal font from the configured terminal font size.
    pub fn terminal_font(&self) -> egui_term::TerminalFont {
        egui_term::TerminalFont::new(egui_term::FontSettings {
            font_type: egui::FontId::monospace(self.fonts.terminal_font_size),
        })
    }
}

/// Font sizes used throughout the application.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AppFonts {
    /// General UI font size (menu, buttons, body text).
    #[serde(default = "default_ui_font_size")]
    pub ui_font_size: f32,
    /// Sidebar group name font size.
    #[serde(default = "default_group_name_font_size")]
    pub group_name_font_size: f32,
    /// Sidebar tab button font size.
    #[serde(default = "default_tab_font_size")]
    pub tab_font_size: f32,
    /// Terminal font size.
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: f32,
}

fn default_ui_font_size() -> f32 {
    15.0
}

fn default_group_name_font_size() -> f32 {
    18.0
}

fn default_tab_font_size() -> f32 {
    15.0
}

fn default_terminal_font_size() -> f32 {
    14.0
}

impl Default for AppFonts {
    fn default() -> Self {
        Self {
            ui_font_size: default_ui_font_size(),
            group_name_font_size: default_group_name_font_size(),
            tab_font_size: default_tab_font_size(),
            terminal_font_size: default_terminal_font_size(),
        }
    }
}

impl AppFonts {
    /// Apply the configured font sizes to the global egui style.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = ctx.global_style().as_ref().clone();
        let proportional = egui::FontFamily::Proportional;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::new(self.ui_font_size, proportional.clone()));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::new(self.ui_font_size, proportional.clone()));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::new(self.ui_font_size + 4.0, proportional.clone()));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::new(self.terminal_font_size, egui::FontFamily::Monospace));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::new(self.ui_font_size - 2.0, proportional));
        ctx.set_global_style(style);
    }
}

/// Render a label plus a font-size slider.
pub fn font_size_slider(ui: &mut egui::Ui, label: &str, size: &mut f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(size, 10.0..=30.0).text("px"));
    });
}

/// Apply a 0-100 opacity percentage to a `Color32`.
pub fn with_alpha(color: Color32, opacity: u8) -> Color32 {
    let alpha = opacity.clamp(0, 100) as f32 / 100.0 * 255.0;
    color_from_rgba(color.r(), color.g(), color.b(), alpha as u8)
}

/// Build a `Color32` from sRGBA bytes, replacing its alpha channel.
/// The input RGB is unmultiplied; egui's `Color32` stores premultiplied alpha.
pub fn color_from_rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Render a label, editable hex text field and a color button that opens egui's
/// color picker popup. Updates `color` when either the text or the picker
/// changes. The color button always has a 1px black border.
pub fn color_picker_button(ui: &mut egui::Ui, label: &str, color: &mut Color32) {
    ui.horizontal(|ui| {
        ui.label(label);

        let mut hex = color_to_hex(*color);
        let text_edit = egui::TextEdit::singleline(&mut hex)
            .desired_width(80.0)
            .font(egui::TextStyle::Monospace);
        let response = ui.add(text_edit);
        if response.changed() {
            *color = color_from_hex(&hex, *color);
        }

        let popup_id = ui.make_persistent_id(label);
        let button_response = color_button_with_black_border(ui, *color);

        egui::Popup::from_toggle_button_response(&button_response)
            .id(popup_id)
            .align(egui::RectAlign::BOTTOM_START)
            .layout(egui::Layout::top_down(egui::Align::Min))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(200.0);
                if egui::color_picker::color_picker_color32(
                    ui,
                    color,
                    egui::color_picker::Alpha::OnlyBlend,
                ) {
                    // Color was changed by the picker; keep in sync.
                }
            });
    });
}

/// Render a percentage slider (0-100) with a numeric label.
pub fn opacity_slider(ui: &mut egui::Ui, label: &str, opacity: &mut u8) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(opacity, 0..=100).text("%"));
    });
}

/// A small color preview button with a fixed 1px black border.
fn color_button_with_black_border(ui: &mut egui::Ui, color: Color32) -> egui::Response {
    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let stroke_width = 1.0;
        egui::color_picker::show_color_at(
            ui.painter(),
            color,
            rect.shrink(stroke_width),
        );
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(stroke_width, egui::Color32::BLACK),
            egui::StrokeKind::Inside,
        );
    }

    response
}

/// Convert a `Color32` to a `#RRGGBB` or `#RRGGBBAA` hex string.
pub fn color_to_hex(color: Color32) -> String {
    if color.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.r(),
            color.g(),
            color.b(),
            color.a()
        )
    }
}

/// Convert a `#RRGGBB` or `#RRGGBBAA` hex string to a `Color32`, returning the
/// fallback on error.
pub fn color_from_hex(hex: &str, fallback: Color32) -> Color32 {
    let is_solid = hex.len() == 7;
    let is_transparent = hex.len() == 9;
    if (!is_solid && !is_transparent) || !hex.starts_with('#') {
        return fallback;
    }
    let parse = |s: &str| u8::from_str_radix(s, 16).ok();
    match (
        parse(&hex[1..3]),
        parse(&hex[3..5]),
        parse(&hex[5..7]),
        if is_transparent { parse(&hex[7..9]) } else { Some(255) },
    ) {
        (Some(r), Some(g), Some(b), Some(a)) => Color32::from_rgba_unmultiplied(r, g, b, a),
        _ => fallback,
    }
}

mod color32_hex {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, Serializer};

    use super::color_from_hex;
    use super::color_to_hex;

    pub fn serialize<S: Serializer>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&color_to_hex(*color))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Color32, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Ok(color_from_hex(&hex, Color32::BLACK))
    }
}

//! Font setup with universal embedded fallback + optional fontconfig support
//!
//! Two fonts are embedded in the binary from `assets/fonts/`:
//! - `Ubuntu-Light` — the primary UI (proportional) font and universal
//!   outline fallback (Cyrillic etc.) in both families on every platform,
//!   so glyph coverage never depends on system fonts;
//! - `NotoEmoji-Regular` — full monochrome emoji coverage, pinned by this
//!   repository instead of depending on the egui version. (Color emoji
//!   fonts such as NotoColorEmoji are CBDT bitmaps, which epaint/ab_glyph
//!   cannot rasterize — monochrome NotoEmoji is the fullest emoji coverage
//!   that actually renders.)
//!
//! On Linux we additionally query fontconfig for extra fallback fonts (Noto,
//! DejaVu, emoji, ...), plus a CJK-capable font picked by glyph coverage so
//! Chinese/Japanese/Korean text renders instead of tofu boxes. On macOS we
//! skip fontconfig entirely: the crate `rust-fontconfig` builds its cache by
//! scanning system fonts and takes ~3 seconds on macOS while finding no
//! useful fonts (fontconfig is not the native macOS font stack).

use egui::{FontData, FontDefinitions, FontFamily, FontTweak};
#[cfg(not(target_os = "macos"))]
use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

/// Ubuntu Light shipped in this repository (Ubuntu Font License, see
/// `assets/fonts/UFL.txt`). Embedded so the same glyphs render on every
/// platform even when no system fonts are available.
const UBUNTU_LIGHT_TTF: &[u8] = include_bytes!("../assets/fonts/Ubuntu-Light.ttf");

/// Name under which the embedded Ubuntu Light is registered in egui.
const EMBEDDED_FONT_NAME: &str = "Ubuntu-Light";

/// Noto Emoji shipped in this repository (SIL OFL 1.1, see
/// `assets/fonts/OFL-NotoEmoji.txt`). Registered under the same key as
/// egui's bundled copy, replacing it — same face, but pinned by this
/// repository instead of depending on the egui version.
const NOTO_EMOJI_TTF: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// Name under which the embedded Noto Emoji is registered in egui.
const NOTO_EMOJI_FONT_NAME: &str = "NotoEmoji-Regular";

/// Initialize fonts: embedded universal fallback everywhere, plus system
/// fallback via fontconfig on platforms where it is useful. Selected system
/// fonts (when configured) are placed at the front of their families.
pub fn apply_font_definitions(
    ctx: &egui::Context,
    ui_font: Option<&str>,
    terminal_font: Option<&str>,
) {
    let mut fonts = FontDefinitions::default();

    // Universal embedded fallback: same glyph coverage on every platform.
    // This replaces egui's bundled copy (same face, but now pinned by this
    // repository instead of depending on the egui version).
    fonts.font_data.insert(
        EMBEDDED_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(UBUNTU_LIGHT_TTF)),
    );
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        let list = fonts.families.entry(family).or_default();
        if !list.iter().any(|name| name == EMBEDDED_FONT_NAME) {
            // Append, never prepend: the first family entry defines the look.
            list.push(EMBEDDED_FONT_NAME.to_owned());
        }
    }

    // Full emoji coverage pinned by this repository. egui's defaults already
    // list this name in both families, so replacing the data entry is enough.
    // Keep egui's tweak (scale 0.81): without it the emoji glyphs (✖ and
    // friends) render visibly bigger than the bundled face they replace.
    fonts.font_data.insert(
        NOTO_EMOJI_FONT_NAME.to_owned(),
        Arc::new(
            FontData::from_static(NOTO_EMOJI_TTF).tweak(FontTweak {
                scale: 0.81, // Make smaller — same as egui's default
                ..Default::default()
            }),
        ),
    );
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        let list = fonts.families.entry(family).or_default();
        if !list.iter().any(|name| name == NOTO_EMOJI_FONT_NAME) {
            list.push(NOTO_EMOJI_FONT_NAME.to_owned());
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Build the cache once and reuse it for listing and loading.
        let cache = font_cache();

        // Get system font fallback chain from fontconfig
        let fallback_fonts = get_fallback_fonts(cache);

        log::info!("Loading fallback fonts: {:?}", fallback_fonts);

        // Load fallback fonts from system
        for font_name in &fallback_fonts {
            if let Some(font_data) = load_system_font(cache, font_name) {
                log::info!("Loaded fallback font: {}", font_name);
                fonts
                    .font_data
                    .insert(font_name.clone(), Arc::new(font_data));

                // Add to monospace family as fallback (at the end of the list)
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push(font_name.clone());

                // Add to proportional family as fallback too (at the end of the list)
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .push(font_name.clone());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        log::info!("Using egui default fonts + embedded Ubuntu-Light on macOS (fontconfig fallback skipped)");
    }

    // Selected system fonts go to the front of their families so they define
    // the look of the UI / terminal.
    insert_selected_font(ctx, &mut fonts, terminal_font, FontFamily::Monospace);
    insert_selected_font(ctx, &mut fonts, ui_font, FontFamily::Proportional);

    ctx.set_fonts(fonts);
}

/// Backwards-compatible entry point: base font setup without any user
/// selection (used by tests and as the pre-settings startup default).
#[cfg(test)]
pub fn setup_fonts_with_fallback(ctx: &egui::Context) {
    apply_font_definitions(ctx, None, None);
}

/// Load the selected system font and insert it at the front of `family`.
/// No-op when no font is selected or the font cannot be loaded (logged).
fn insert_selected_font(
    _ctx: &egui::Context,
    fonts: &mut FontDefinitions,
    name: Option<&str>,
    family: FontFamily,
) {
    let Some(name) = name else {
        return;
    };

    #[cfg(not(target_os = "macos"))]
    {
        match load_system_font(font_cache(), name) {
            Some(font_data) => {
                let list = fonts.families.entry(family).or_default();
                list.retain(|existing| existing != name);
                list.insert(0, name.to_owned());
                fonts.font_data.insert(name.to_owned(), Arc::new(font_data));
            }
            None => log::warn!(
                "Could not load selected font {:?}; keeping the default",
                name
            ),
        }
    }

    #[cfg(target_os = "macos")]
    {
        log::warn!(
            "System font selection is not supported on macOS; keeping the default ({:?})",
            name
        );
    }
}

/// System font faces available for selection in the Font Settings dialog,
/// computed once and cached for the rest of the session.
pub struct SystemFonts {
    /// All installed font faces.
    pub all: Vec<String>,
    /// Only the monospace font faces (for the terminal).
    pub monospace: Vec<String>,
}

/// List system fonts for the Font Settings dialog. On macOS fontconfig is
/// skipped (see the module docs), so the lists are empty and the dialog falls
/// back to the embedded defaults.
pub fn system_fonts() -> &'static SystemFonts {
    static FONTS: OnceLock<SystemFonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        #[cfg(not(target_os = "macos"))]
        {
            let cache = font_cache();
            let mut all: Vec<String> = Vec::new();
            let mut monospace: Vec<String> = Vec::new();
            for (metadata, _font_id) in cache.list() {
                let Some(name) = &metadata.name else {
                    continue;
                };
                if !all.contains(name) {
                    all.push(name.clone());
                }
                if metadata.monospace == rust_fontconfig::PatternMatch::True
                    && !monospace.contains(name)
                {
                    monospace.push(name.clone());
                }
            }
            all.sort();
            monospace.sort();
            SystemFonts { all, monospace }
        }
        #[cfg(target_os = "macos")]
        {
            SystemFonts {
                all: Vec::new(),
                monospace: Vec::new(),
            }
        }
    })
}

/// The shared fontconfig cache. Built lazily on first use (e.g. the startup
/// font setup) and reused for fallback listing, font loading and the Font
/// Settings dialog listing.
#[cfg(not(target_os = "macos"))]
fn font_cache() -> &'static rust_fontconfig::FcFontCache {
    static CACHE: OnceLock<rust_fontconfig::FcFontCache> = OnceLock::new();
    CACHE.get_or_init(rust_fontconfig::FcFontCache::build)
}

/// Get fallback fonts from fontconfig (only for missing glyphs)
#[cfg(not(target_os = "macos"))]
fn get_fallback_fonts(cache: &rust_fontconfig::FcFontCache) -> Vec<String> {
    let mut fonts = Vec::new();
    let mut seen = HashSet::new();

    // Query monospace fonts that cover wide unicode ranges
    let all_fonts = cache.list();

    for (metadata, _font_id) in all_fonts {
        // Look for fonts with wide Unicode coverage for fallback
        if metadata.monospace == rust_fontconfig::PatternMatch::True {
            if let Some(name) = &metadata.name {
                // Skip base fonts that may already be in the system
                if name.contains("Noto") || name.contains("DejaVu") || name.contains("Symbol") {
                    if seen.insert(name.clone()) {
                        fonts.push(name.clone());
                    }
                }
            }
        }
    }

    // Priority fallback fonts (for special characters)
    let mut result = vec![
        "Noto Color Emoji",   // Emoji
        "Noto Sans Symbols",  // Mathematical symbols
        "Noto Sans Symbols2", // Additional symbols
        "DejaVu Sans",        // Fallback
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

    // CJK fallback picked by glyph coverage. The monospace scan above never
    // finds these: CJK faces are dual-width, so their `post` table reports
    // isFixedPitch=0 and rust-fontconfig marks them non-monospace — even the
    // ones named "Noto Sans Mono CJK". Without this, CJK text renders as
    // tofu boxes (□).
    if let Some(cjk) = cjk_fallback_font(cache) {
        result.push(cjk);
    }

    // Add the rest found by fontconfig
    for font in fonts {
        if !result.contains(&font) {
            result.push(font);
        }
    }

    // Limit the number of fallback fonts
    result.truncate(7);

    result
}

/// Pick the best CJK-capable fallback font by glyph coverage, preferring a
/// monospaced Simplified Chinese face (terminal cells stay double-width and
/// hanzi get SC glyph variants). Coverage is checked against the OS/2
/// unicode ranges cached by rust-fontconfig: U+4E00 (一) is present in every
/// CJK font. Returns `None` when no installed font covers CJK.
#[cfg(not(target_os = "macos"))]
fn cjk_fallback_font(cache: &rust_fontconfig::FcFontCache) -> Option<String> {
    const CJK_IDEOGRAPH: char = '\u{4E00}'; // 一

    cache
        .list()
        .iter()
        .filter(|(pattern, _)| {
            pattern
                .unicode_ranges
                .iter()
                .any(|range| range.contains(CJK_IDEOGRAPH))
        })
        .filter_map(|(pattern, _)| pattern.name.clone())
        .max_by_key(|name| cjk_fallback_score(name))
}

/// Preference score for a CJK fallback face: mono > SC variants > plain CJK,
/// Regular over Bold.
#[cfg(not(target_os = "macos"))]
fn cjk_fallback_score(name: &str) -> i32 {
    let mut score = 0;
    if name.contains("Mono") {
        score += 4; // Fixed advance for CJK cells in the terminal grid
    }
    if name.contains("SC") {
        score += 2; // Simplified Chinese glyph variants
    }
    if name.contains("CJK") {
        score += 1;
    }
    if name.contains("Bold") {
        score -= 1; // Prefer the Regular face of the same family
    }
    score
}

/// Try to load a system font by name
#[cfg(not(target_os = "macos"))]
fn load_system_font(cache: &rust_fontconfig::FcFontCache, name: &str) -> Option<FontData> {
    use rust_fontconfig::FcPattern;

    let pattern = FcPattern {
        name: Some(name.to_string()),
        ..Default::default()
    };

    let font_match = cache.query(&pattern, &mut Vec::new())?;
    let font_source = cache.get_font_by_id(&font_match.id)?;

    match font_source {
        rust_fontconfig::FontSource::Disk(font_path) => std::fs::read(&font_path.path)
            .ok()
            // fontconfig resolved a specific face of the file (e.g. "Noto Sans
            // Mono CJK SC" is one face of the 10-face NotoSansCJK .ttc);
            // without the index egui would always load face 0.
            .map(|bytes| FontData {
                index: font_path.font_index as u32,
                ..FontData::from_owned(bytes)
            }),
        rust_fontconfig::FontSource::Memory(font) => Some(FontData {
            index: font.font_index as u32,
            ..FontData::from_owned(font.bytes.clone())
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: the embedded fonts must parse and rasterize (Cyrillic +
    /// emoji + the proportional UI face) without panicking. Font loading
    /// problems in egui surface lazily at raster time, not at registration.
    #[test]
    fn embedded_fonts_rasterize_without_panicking() {
        let ctx = egui::Context::default();
        setup_fonts_with_fallback(&ctx);
        ctx.begin_pass(egui::RawInput::default());
        let ui = egui::FontId::proportional(14.0);
        let mono = egui::FontId::monospace(14.0);
        let cyrillic_ui = ctx.fonts_mut(|f| f.has_glyphs(&ui, "Привет"));
        let emoji_ui = ctx.fonts_mut(|f| f.has_glyphs(&ui, "📂⌨✓"));
        let cyrillic_mono = ctx.fonts_mut(|f| f.has_glyphs(&mono, "Привет"));
        let emoji_mono = ctx.fonts_mut(|f| f.has_glyphs(&mono, "📂✓"));
        // Rasterize once so a broken font face would panic here, not in prod.
        let _galley = ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                "Привет H2Term 📂🤖🔀🖥⌨🔍 ✓".to_string(),
                ui,
                egui::Color32::WHITE,
            )
        });
        ctx.end_pass();

        assert!(cyrillic_ui, "Cyrillic missing in the UI font");
        assert!(emoji_ui, "emoji missing in the UI font");
        assert!(cyrillic_mono, "Cyrillic missing in the monospace fallback");
        assert!(emoji_mono, "emoji missing in the monospace fallback");
    }

    /// CJK text must render, not turn into tofu boxes. The fallback is picked
    /// by glyph coverage via fontconfig (see `cjk_fallback_font`); skipped on
    /// systems without any CJK-capable font (e.g. a minimal CI container).
    #[test]
    fn cjk_fallback_provides_glyphs() {
        #[cfg(not(target_os = "macos"))]
        {
            if cjk_fallback_font(font_cache()).is_none() {
                return; // No CJK fonts installed — nothing to fall back to.
            }
        }
        let ctx = egui::Context::default();
        apply_font_definitions(&ctx, None, None);
        ctx.begin_pass(egui::RawInput::default());
        let mono = egui::FontId::monospace(14.0);
        let ui = egui::FontId::proportional(14.0);
        let cjk_mono = ctx.fonts_mut(|f| f.has_glyphs(&mono, "在提交中文"));
        let cjk_ui = ctx.fonts_mut(|f| f.has_glyphs(&ui, "在提交中文"));
        let _ = ctx.end_pass();

        assert!(cjk_mono, "CJK missing in the monospace fallback");
        assert!(cjk_ui, "CJK missing in the proportional fallback");
    }

    /// Selecting a system font must put it at the front of the target family
    /// (the first entry defines the look). Skipped when no monospace system
    /// fonts are installed (e.g. a minimal CI container).
    #[test]
    fn selected_terminal_font_goes_to_front_of_monospace_family() {
        let Some(name) = system_fonts().monospace.first().cloned() else {
            return;
        };
        let ctx = egui::Context::default();
        apply_font_definitions(&ctx, None, Some(&name));
        ctx.begin_pass(egui::RawInput::default());
        let first =
            ctx.fonts(|f| f.definitions().families[&egui::FontFamily::Monospace][0].clone());
        let _ = ctx.end_pass();
        assert_eq!(first, name);
    }
}

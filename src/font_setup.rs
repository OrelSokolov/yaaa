//! Font setup with universal embedded fallback + optional fontconfig support
//!
//! `Ubuntu-Light` is embedded in the binary from `assets/fonts/` and installed
//! into both font families on every platform, so glyph coverage (Cyrillic,
//! etc.) never depends on system fonts — this is the universal fallback.
//!
//! On Linux we additionally query fontconfig for extra fallback fonts (Noto,
//! DejaVu, emoji, ...). On macOS we skip fontconfig entirely: the crate
//! `rust-fontconfig` builds its cache by scanning system fonts and takes
//! ~3 seconds on macOS while finding no useful fonts (fontconfig is not the
//! native macOS font stack).

use egui::{FontData, FontDefinitions, FontFamily};
#[cfg(not(target_os = "macos"))]
use std::collections::HashSet;
use std::sync::Arc;

/// Ubuntu Light shipped in this repository (Ubuntu Font License, see
/// `assets/fonts/UFL.txt`). Embedded so the same glyphs render on every
/// platform even when no system fonts are available.
const UBUNTU_LIGHT_TTF: &[u8] = include_bytes!("../assets/fonts/Ubuntu-Light.ttf");

/// Name under which the embedded Ubuntu Light is registered in egui.
const EMBEDDED_FONT_NAME: &str = "Ubuntu-Light";

/// Initialize fonts: embedded universal fallback everywhere, plus system
/// fallback via fontconfig on platforms where it is useful.
pub fn setup_fonts_with_fallback(ctx: &egui::Context) {
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

    #[cfg(target_os = "macos")]
    {
        log::info!("Using egui default fonts + embedded Ubuntu-Light on macOS (fontconfig fallback skipped)");
        ctx.set_fonts(fonts);
    }

    #[cfg(not(target_os = "macos"))]
    setup_fonts_with_fontconfig(ctx, fonts);
}

#[cfg(not(target_os = "macos"))]
fn setup_fonts_with_fontconfig(ctx: &egui::Context, mut fonts: FontDefinitions) {
    use rust_fontconfig::FcFontCache;

    // Build the cache once and reuse it for both listing and loading.
    let cache = FcFontCache::build();

    // Get system font fallback chain from fontconfig
    let fallback_fonts = get_fallback_fonts(&cache);

    log::info!("Loading fallback fonts: {:?}", fallback_fonts);

    // Load fallback fonts from system
    for font_name in &fallback_fonts {
        if let Some(font_data) = load_system_font(&cache, font_name) {
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

    ctx.set_fonts(fonts);
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
    let priority_fallback = vec![
        "Noto Color Emoji",   // Emoji
        "Noto Sans Symbols",  // Mathematical symbols
        "Noto Sans Symbols2", // Additional symbols
        "DejaVu Sans",        // Fallback
    ];

    // Add priority fonts at the front
    let mut result = priority_fallback
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Add the rest found by fontconfig
    for font in fonts {
        if !result.contains(&font) {
            result.push(font);
        }
    }

    // Limit the number of fallback fonts
    result.truncate(6);

    result
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
            .map(FontData::from_owned),
        rust_fontconfig::FontSource::Memory(font) => Some(FontData::from_owned(font.bytes.clone())),
    }
}

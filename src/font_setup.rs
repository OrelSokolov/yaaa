//! Font setup with optional fontconfig fallback support
//!
//! On Linux we query fontconfig for extra fallback fonts (Noto, DejaVu, etc.).
//! On macOS we skip fontconfig entirely: the crate `rust-fontconfig` builds its
//! cache by scanning system fonts and takes ~3 seconds on macOS while finding
//! no useful fonts (fontconfig is not the native macOS font stack). egui's
//! built-in default fonts cover the UI perfectly well on their own.

use egui::{FontData, FontDefinitions, FontFamily};
use std::collections::HashSet;
use std::sync::Arc;

/// Initialize fonts with optional system fallback.
pub fn setup_fonts_with_fallback(ctx: &egui::Context) {
    #[cfg(target_os = "macos")]
    {
        log::info!("Using egui default fonts on macOS (fontconfig fallback skipped)");
        ctx.set_fonts(FontDefinitions::default());
        return;
    }

    #[cfg(not(target_os = "macos"))]
    setup_fonts_with_fontconfig(ctx);
}

#[cfg(not(target_os = "macos"))]
fn setup_fonts_with_fontconfig(ctx: &egui::Context) {
    use rust_fontconfig::FcFontCache;

    let mut fonts = FontDefinitions::default();

    // Build the cache once and reuse it for both listing and loading.
    let cache = FcFontCache::build();

    // Get system font fallback chain from fontconfig
    let fallback_fonts = get_fallback_fonts(&cache);

    log::info!("Loading fallback fonts: {:?}", fallback_fonts);

    // Load fallback fonts from system
    for font_name in &fallback_fonts {
        if let Some(font_data) = load_system_font(&cache, font_name) {
            log::info!("Loaded fallback font: {}", font_name);
            fonts.font_data.insert(font_name.clone(), Arc::new(font_data));

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
        "Noto Color Emoji",      // Emoji
        "Noto Sans Symbols",     // Mathematical symbols
        "Noto Sans Symbols2",    // Additional symbols
        "DejaVu Sans",           // Fallback
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
fn load_system_font(
    cache: &rust_fontconfig::FcFontCache,
    name: &str,
) -> Option<FontData> {
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
//! Font setup with fontconfig fallback support
//!
//! This module configures egui fonts with automatic fallback to system fonts
//! for missing Unicode characters using fontconfig.

use egui::{FontData, FontDefinitions, FontFamily, FontId};
use rust_fontconfig::{FcFontCache, FcPattern};
use std::collections::HashSet;
use std::sync::Arc;

/// Initialize fonts with fontconfig fallback
pub fn setup_fonts_with_fallback(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Get system font fallback chain from fontconfig
    let fallback_fonts = get_fallback_fonts();
    
    log::info!("Loading fallback fonts: {:?}", fallback_fonts);

    // Load fallback fonts from system
    for font_name in &fallback_fonts {
        if let Some(font_data) = load_system_font(font_name) {
            log::info!("Loaded fallback font: {}", font_name);
            fonts.font_data.insert(font_name.clone(), Arc::new(font_data));
            
            // Add to monospace family as fallback (в конец списка!)
            fonts.families
                .entry(FontFamily::Monospace)
                .or_default()
                .push(font_name.clone());
                
            // Add to proportional family as fallback too (в конец списка!)
            fonts.families
                .entry(FontFamily::Proportional)
                .or_default()
                .push(font_name.clone());
        }
    }

    ctx.set_fonts(fonts);
}

/// Get fallback fonts from fontconfig (only for missing glyphs)
fn get_fallback_fonts() -> Vec<String> {
    let cache = FcFontCache::build();
    let mut fonts = Vec::new();
    let mut seen = HashSet::new();

    // Query monospace fonts that cover wide unicode ranges
    let all_fonts = cache.list();
    
    for (metadata, _font_id) in all_fonts {
        // Ищем шрифты с широким покрытием Unicode для fallback
        if metadata.monospace == rust_fontconfig::PatternMatch::True {
            if let Some(name) = &metadata.name {
                // Пропускаем базовые шрифты которые уже могут быть в системе
                if name.contains("Noto") || name.contains("DejaVu") || name.contains("Symbol") {
                    if seen.insert(name.clone()) {
                        fonts.push(name.clone());
                    }
                }
            }
        }
    }

    // Приоритетные fallback шрифты (для специальных символов)
    let priority_fallback = vec![
        "Noto Color Emoji",      // Эмодзи
        "Noto Sans Symbols",     // Математические символы
        "Noto Sans Symbols2",    // Дополнительные символы
        "DejaVu Sans",           // Резервный
    ];

    // Добавляем приоритетные в начало
    let mut result = priority_fallback.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    
    // Добавляем остальные найденные
    for font in fonts {
        if !result.contains(&font) {
            result.push(font);
        }
    }

    // Ограничиваем количество fallback шрифтов
    result.truncate(6);

    result
}

/// Try to load a system font by name
fn load_system_font(name: &str) -> Option<FontData> {
    let cache = FcFontCache::build();
    let pattern = FcPattern {
        name: Some(name.to_string()),
        ..Default::default()
    };

    let font_match = cache.query(&pattern, &mut Vec::new())?;
    let font_source = cache.get_font_by_id(&font_match.id)?;

    match font_source {
        rust_fontconfig::FontSource::Disk(font_path) => {
            std::fs::read(&font_path.path)
                .ok()
                .map(|bytes| FontData::from_owned(bytes))
        }
        rust_fontconfig::FontSource::Memory(font) => {
            Some(FontData::from_owned(font.bytes.clone()))
        }
    }
}

/// Get terminal font
pub fn get_terminal_font() -> FontId {
    FontId::monospace(14.0)
}

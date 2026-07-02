pub fn apply_menu_style(ui: &mut egui::Ui, font_size: f32) {
    ui.set_min_width(200.0);
    ui.style_mut().spacing.button_padding = egui::vec2(12.0, 4.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(font_size));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(font_size));
}

pub fn apply_menu_style(ui: &mut egui::Ui) {
    ui.style_mut().spacing.button_padding = egui::vec2(12.0, 8.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
}

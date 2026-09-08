use super::WindowActions;
use crate::font_setup;
use crate::theme::{font_size_slider, AppFonts};

impl super::WindowManager {
    pub(super) fn show_font_settings_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
        let mut save = false;
        let mut cancel = false;
        let mut preview = false;
        let mut restore_defaults = false;

        let window_id = egui::Id::new("font_settings_window");

        if self.show_font_settings && !self.was_font_settings_open {
            ctx.memory_mut(|m| m.request_focus(window_id));
        }
        self.was_font_settings_open = self.show_font_settings;

        egui::Window::new("Font Settings")
            .id(window_id)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_font_settings)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Font Settings");
                    ui.add_space(10.0);

                    font_size_slider(ui, "UI font size", &mut self.editing_fonts.ui_font_size);
                    ui.add_space(4.0);
                    font_size_slider(
                        ui,
                        "Group name font size",
                        &mut self.editing_fonts.group_name_font_size,
                    );
                    ui.add_space(4.0);
                    font_size_slider(ui, "Tab font size", &mut self.editing_fonts.tab_font_size);
                    ui.add_space(4.0);
                    font_size_slider(
                        ui,
                        "Terminal font size",
                        &mut self.editing_fonts.terminal_font_size,
                    );

                    ui.add_space(10.0);

                    let system = font_setup::system_fonts();
                    if system.all.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "System font selection is not available on this platform.",
                            )
                            .weak(),
                        );
                    } else {
                        ui.label("UI font");
                        font_combo(
                            ui,
                            "ui_font_combo",
                            &mut self.editing_fonts.ui_font_name,
                            &system.all,
                        );
                        ui.add_space(4.0);
                        ui.label("Terminal font");
                        font_combo(
                            ui,
                            "terminal_font_combo",
                            &mut self.editing_fonts.terminal_font_name,
                            &system.monospace,
                        );
                    }

                    ui.add_space(15.0);

                    ui.label("Preview");
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("UI text").size(self.editing_fonts.ui_font_size),
                        );
                        ui.label(
                            egui::RichText::new("Group")
                                .size(self.editing_fonts.group_name_font_size),
                        );
                        ui.label(egui::RichText::new("Tab").size(self.editing_fonts.tab_font_size));
                        ui.label(
                            egui::RichText::new("Terminal").font(egui::FontId::monospace(
                                self.editing_fonts.terminal_font_size,
                            )),
                        );
                    });

                    ui.add_space(15.0);

                    if super::shortcuts_active(ctx, window_id)
                        && ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked()
                            || (super::shortcuts_active(ctx, window_id)
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            save = true;
                        }
                        if ui.button("Preview").clicked() {
                            preview = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        if ui.button("Restore Defaults").clicked() {
                            restore_defaults = true;
                        }
                    });
                });
            });

        if restore_defaults {
            self.editing_fonts = AppFonts::default();
            self.editing_fonts.apply(ctx);
        }

        if preview {
            self.editing_fonts.apply(ctx);
        }

        if save {
            actions.fonts = Some(self.editing_fonts.clone());
            actions.should_save_settings = true;
            self.show_font_settings = false;
        }
        if cancel {
            // "Preview" applied the draft fonts; the owner restores its
            // applied fonts on this signal.
            actions.fonts_discarded = true;
            self.show_font_settings = false;
        }
    }
}

/// A combo box for picking a system font face. `None` selects the embedded
/// default face.
fn font_combo(ui: &mut egui::Ui, id: &str, selected: &mut Option<String>, available: &[String]) {
    let selected_text = selected.clone().unwrap_or_else(|| "Default".to_string());
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected_text)
        .width(260.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, "Default");
            for name in available {
                ui.selectable_value(selected, Some(name.clone()), name);
            }
        });
}

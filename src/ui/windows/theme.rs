use super::WindowActions;
use crate::theme::{color_picker_button, opacity_slider, AppButtonStyle, AppTheme};

impl super::WindowManager {
    pub(super) fn show_theme_settings_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
        let mut save = false;
        let mut cancel = false;
        let mut restore_defaults = false;

        let window_id = egui::Id::new("theme_settings_window");

        if self.show_theme_settings && !self.was_theme_settings_open {
            ctx.memory_mut(|m| m.request_focus(window_id));
        }
        self.was_theme_settings_open = self.show_theme_settings;

        egui::Window::new("Theme Settings")
            .id(window_id)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_theme_settings)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Theme Settings");
                    ui.add_space(10.0);

                    egui::CollapsingHeader::new("Terminal colors")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add_space(6.0);
                            color_picker_button(
                                ui,
                                "Terminal foreground",
                                &mut self.editing_theme.terminal_fg,
                            );
                        });

                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("Background")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label("Used for the app and terminal background.");
                            ui.add_space(6.0);
                            color_picker_button(
                                ui,
                                "Background color",
                                &mut self.editing_theme.app_bg,
                            );
                            opacity_slider(
                                ui,
                                "Background opacity",
                                &mut self.editing_theme.app_bg_opacity,
                            );
                        });

                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("UI colors")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add_space(6.0);
                            color_picker_button(
                                ui,
                                "Sidebar text",
                                &mut self.editing_theme.panel_text,
                            );
                            ui.add_space(4.0);
                            color_picker_button(
                                ui,
                                "Sidebar selected text",
                                &mut self.editing_theme.panel_text_selected,
                            );
                            ui.add_space(4.0);
                            color_picker_button(
                                ui,
                                "Sidebar hover text",
                                &mut self.editing_theme.panel_text_hover,
                            );
                            ui.add_space(4.0);
                            color_picker_button(ui, "Tab text", &mut self.editing_theme.tab_text);
                            ui.add_space(4.0);
                            color_picker_button(
                                ui,
                                "Active tab background",
                                &mut self.editing_theme.tab_active_bg,
                            );

                            ui.add_space(10.0);
                            ui.label("Preview");
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Sidebar text")
                                        .color(self.editing_theme.panel_text),
                                );
                                ui.label(
                                    egui::RichText::new("Selected")
                                        .color(self.editing_theme.panel_text_selected),
                                );
                                ui.label(
                                    egui::RichText::new("Hover")
                                        .color(self.editing_theme.panel_text_hover),
                                );
                                ui.label(
                                    egui::RichText::new("Tab").color(self.editing_theme.tab_text),
                                );
                            });
                            ui.horizontal(|ui| {
                                let preview_bg = self.editing_theme.tab_active_bg;
                                ui.label(
                                    egui::RichText::new("Active tab")
                                        .color(self.editing_theme.tab_text)
                                        .background_color(preview_bg),
                                );
                            });
                        });

                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("Tab buttons")
                        .default_open(false)
                        .show(ui, |ui| {
                            button_style_group(ui, &mut self.editing_theme.tab_button, "Tab");
                        });

                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("Close buttons")
                        .default_open(false)
                        .show(ui, |ui| {
                            button_style_group(ui, &mut self.editing_theme.close_button, "✖");
                        });

                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("Agent buttons")
                        .default_open(false)
                        .show(ui, |ui| {
                            button_style_group(
                                ui,
                                &mut self.editing_theme.agent_button,
                                "➕ Agent",
                            );
                        });

                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("Terminal buttons")
                        .default_open(false)
                        .show(ui, |ui| {
                            button_style_group(
                                ui,
                                &mut self.editing_theme.terminal_button,
                                "➕ Terminal",
                            );
                        });

                    ui.add_space(15.0);

                    if super::shortcuts_active(ctx, window_id)
                        && ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("Save").min_size(egui::vec2(80.0, 32.0)))
                            .clicked()
                            || (super::shortcuts_active(ctx, window_id)
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            save = true;
                        }
                        if ui
                            .add(egui::Button::new("Cancel").min_size(egui::vec2(80.0, 32.0)))
                            .clicked()
                        {
                            cancel = true;
                        }
                        if ui
                            .add(
                                egui::Button::new("Restore Defaults")
                                    .min_size(egui::vec2(80.0, 32.0)),
                            )
                            .clicked()
                        {
                            restore_defaults = true;
                        }
                    });
                });
            });

        if restore_defaults {
            self.editing_theme = AppTheme::default();
            // Apply preview immediately so the user sees the defaults.
            self.editing_theme.apply_to_visuals(ctx);
        }

        if save {
            actions.theme = Some(self.editing_theme.clone());
            actions.should_save_settings = true;
            self.show_theme_settings = false;
        }
        if cancel {
            // The live preview already applied the draft to the egui
            // visuals; the owner restores its applied theme on this signal.
            actions.theme_discarded = true;
            self.show_theme_settings = false;
        }
    }
}

/// Render Normal + Hover color pickers and a live preview button for one
/// button style group (close, agent or terminal).
fn button_style_group(ui: &mut egui::Ui, style: &mut AppButtonStyle, preview_label: &str) {
    ui.add_space(6.0);
    ui.push_id("normal", |ui| {
        ui.label("Normal");
        ui.add_space(4.0);
        color_picker_button(ui, "Background", &mut style.bg);
        ui.add_space(4.0);
        color_picker_button(ui, "Font color", &mut style.text);
        ui.add_space(4.0);
        color_picker_button(ui, "Border color", &mut style.border);
    });

    ui.add_space(10.0);
    ui.push_id("hover", |ui| {
        ui.label("Hover");
        ui.add_space(4.0);
        color_picker_button(ui, "Background", &mut style.bg_hover);
        ui.add_space(4.0);
        color_picker_button(ui, "Font color", &mut style.text_hover);
        ui.add_space(4.0);
        color_picker_button(ui, "Border color", &mut style.border_hover);
    });

    ui.add_space(10.0);
    ui.label("Preview");
    ui.horizontal(|ui| {
        let s = *style;
        s.apply_to_visuals(ui);
        ui.add(egui::Button::new(preview_label).min_size(egui::vec2(0.0, 28.0)));
    });
}

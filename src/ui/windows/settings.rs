use super::WindowActions;

impl super::WindowManager {
    pub(super) fn show_settings_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
        let mut settings_save = false;
        let mut settings_cancel = false;

        let window_id = egui::Id::new("settings_window");

        if self.show_settings && !self.was_settings_open {
            ctx.memory_mut(|m| m.request_focus(window_id));
        }
        self.was_settings_open = self.show_settings;

        egui::Window::new("Settings")
            .id(window_id)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_settings)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("General Settings");
                    ui.add_space(10.0);

                    ui.label("Default shell cmd:");
                    ui.text_edit_singleline(&mut self.editing_default_shell_cmd);

                    ui.add_space(15.0);

                    ui.checkbox(&mut self.editing_run_as_login_shell, "Run as login shell");

                    ui.add_space(15.0);

                    ui.checkbox(
                        &mut self.editing_enable_git_status,
                        "Show Git status in sidebar",
                    );

                    ui.add_space(15.0);

                    ui.checkbox(
                        &mut self.editing_preload_tabs,
                        "Enable terminal preload",
                    );

                    ui.add_space(15.0);

                    if super::shortcuts_active(ctx, window_id)
                        && ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        settings_cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked()
                            || (super::shortcuts_active(ctx, window_id)
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            settings_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            settings_cancel = true;
                        }
                    });
                });
            });

        if settings_save {
            actions.default_shell_cmd = Some(self.editing_default_shell_cmd.trim().to_string());
            actions.run_as_login_shell = Some(self.editing_run_as_login_shell);
            actions.enable_git_status = Some(self.editing_enable_git_status);
            actions.preload_tabs = Some(self.editing_preload_tabs);
            actions.should_save_settings = true;
            self.show_settings = false;
        }
        if settings_cancel {
            // The draft is re-seeded on the next open, nothing to restore.
            self.show_settings = false;
        }
    }
}

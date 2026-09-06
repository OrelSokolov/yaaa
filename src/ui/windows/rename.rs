use super::WindowActions;

impl super::WindowManager {
    /// Open the rename dialog for a group.
    pub fn rename_group(&mut self, group_id: u64, name: String) {
        self.rename_group_id = Some(group_id);
        self.rename_group_name = name;
        self.show_rename_group = true;
    }

    /// Show a "folder not found" notice for a missing project path.
    pub fn missing_folder(&mut self, message: String) {
        self.missing_folder_message = message;
        self.show_missing_folder = true;
    }

    pub(super) fn show_rename_group_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
        let mut should_save = false;
        let mut should_close = false;

        egui::Window::new("Rename Group")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_rename_group)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Rename Group");
                    ui.text_edit_singleline(&mut self.rename_group_name);
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        should_close = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            should_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_save {
            if let Some(id) = self.rename_group_id {
                actions.rename_group = Some((id, self.rename_group_name.clone()));
                self.show_rename_group = false;
                self.rename_group_id = None;
            }
        }
        if should_close {
            self.show_rename_group = false;
            self.rename_group_id = None;
        }
    }

    pub(super) fn show_missing_folder_window(&mut self, ctx: &egui::Context) {
        let mut ok = false;

        egui::Window::new("Folder not found")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_missing_folder)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Folder not found");
                    ui.add_space(10.0);
                    ui.label(&self.missing_folder_message);
                    ui.add_space(6.0);
                    ui.label("The project has been removed from the projects list.");
                    ui.add_space(15.0);
                    if ui
                        .add(egui::Button::new("OK").min_size(egui::vec2(80.0, 32.0)))
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        ok = true;
                    }
                });
            });

        if ok {
            self.show_missing_folder = false;
        }
    }

    pub(super) fn show_close_confirmation_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
        let mut confirmed = false;
        let mut cancelled = false;

        egui::Window::new("Confirm Exit")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_close_confirmation)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Are you sure?");
                    ui.add_space(15.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("Yes").min_size(egui::vec2(80.0, 32.0)))
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui
                            .add(egui::Button::new("No").min_size(egui::vec2(80.0, 32.0)))
                            .clicked()
                        {
                            cancelled = true;
                        }
                    });
                });
            });

        if confirmed {
            actions.close_confirmed = true;
            self.show_close_confirmation = false;
        }
        if cancelled {
            self.show_close_confirmation = false;
        }
    }
}

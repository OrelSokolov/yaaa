use crate::hotkeys::get_hotkeys;

pub struct WindowManager {
    pub show_about: bool,
    pub show_hotkeys: bool,
    pub show_settings: bool,
    pub show_rename_group: bool,
    pub rename_group_id: Option<u64>,
    pub rename_group_name: String,
    pub editing_default_shell_cmd: String,
    pub editing_default_agent_cmd: String,
    pub saved_default_shell_cmd: String,
    pub saved_default_agent_cmd: String,
    pub editing_run_as_login_shell: bool,
    pub saved_run_as_login_shell: bool,
    pub was_settings_open: bool,
}

impl WindowManager {
    pub fn new(
        default_shell_cmd: String,
        default_agent_cmd: String,
        run_as_login_shell: bool,
    ) -> Self {
        let editing_default_shell_cmd = default_shell_cmd.clone();
        let editing_default_agent_cmd = default_agent_cmd.clone();
        let saved_default_shell_cmd = editing_default_shell_cmd.clone();
        let saved_default_agent_cmd = editing_default_agent_cmd.clone();
        let editing_run_as_login_shell = run_as_login_shell;
        let saved_run_as_login_shell = run_as_login_shell;

        Self {
            show_about: false,
            show_hotkeys: false,
            show_settings: false,
            show_rename_group: false,
            rename_group_id: None,
            rename_group_name: String::new(),
            editing_default_shell_cmd,
            editing_default_agent_cmd,
            saved_default_shell_cmd,
            saved_default_agent_cmd,
            editing_run_as_login_shell,
            saved_run_as_login_shell,
            was_settings_open: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> WindowActions {
        let mut actions = WindowActions::default();

        self.show_about_window(ctx);
        self.show_hotkeys_window(ctx);
        self.show_rename_group_window(ctx, &mut actions);
        self.show_settings_window(ctx, &mut actions);

        actions
    }

    fn show_about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Yet Another AI Agent");
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(10.0);
                    ui.label("Multi-agent terminal with tabs and project management");
                    ui.label("Manage multiple agent sessions across different projects");
                    ui.add_space(10.0);
                    ui.label("Author: Oleg Orlov (orelcokolov@gmail.com)");
                });
            });
    }

    fn show_hotkeys_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Hotkeys")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_hotkeys)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Keyboard Shortcuts");
                    ui.add_space(10.0);

                    egui::Grid::new("hotkeys_grid")
                        .num_columns(2)
                        .spacing([40.0, 8.0])
                        .show(ui, |ui| {
                            let hotkeys = get_hotkeys();
                            for (key, description) in hotkeys {
                                ui.label(egui::RichText::new(key).strong());
                                ui.label(description);
                                ui.end_row();
                            }
                        });
                });
            });
    }

    fn show_rename_group_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
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
            actions.rename_group = Some((
                self.rename_group_id.unwrap(),
                self.rename_group_name.clone(),
            ));
            self.show_rename_group = false;
            self.rename_group_id = None;
            actions.should_save_groups = true;
        }
        if should_close {
            self.show_rename_group = false;
            self.rename_group_id = None;
        }
    }

    fn show_settings_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
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

                    ui.add_space(5.0);

                    ui.label("Default agent cmd:");
                    ui.text_edit_singleline(&mut self.editing_default_agent_cmd);

                    ui.add_space(15.0);

                    ui.checkbox(&mut self.editing_run_as_login_shell, "Run as login shell");

                    ui.add_space(15.0);

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        settings_cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
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
            actions.default_shell_cmd = Some(self.editing_default_shell_cmd.clone());
            actions.default_agent_cmd = Some(self.editing_default_agent_cmd.clone());
            actions.run_as_login_shell = Some(self.editing_run_as_login_shell);
            self.saved_default_shell_cmd = self.editing_default_shell_cmd.clone();
            self.saved_default_agent_cmd = self.editing_default_agent_cmd.clone();
            self.saved_run_as_login_shell = self.editing_run_as_login_shell;
            actions.should_save_settings = true;
            self.show_settings = false;
        }
        if settings_cancel {
            self.editing_default_shell_cmd = self.saved_default_shell_cmd.clone();
            self.editing_default_agent_cmd = self.saved_default_agent_cmd.clone();
            self.editing_run_as_login_shell = self.saved_run_as_login_shell;
            self.show_settings = false;
        }
    }

    pub fn rename_group(&mut self, group_id: u64, name: String) {
        self.rename_group_id = Some(group_id);
        self.rename_group_name = name;
        self.show_rename_group = true;
    }
}

#[derive(Default)]
pub struct WindowActions {
    pub rename_group: Option<(u64, String)>,
    pub default_shell_cmd: Option<String>,
    pub default_agent_cmd: Option<String>,
    pub run_as_login_shell: Option<bool>,
    pub should_save_groups: bool,
    pub should_save_settings: bool,
}

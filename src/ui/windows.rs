use crate::config::settings::{AgentConfig, MAX_AGENTS};
use crate::hotkeys::get_hotkeys;
use crate::theme::{
    color_picker_button, font_size_slider, opacity_slider, AppFonts, AppTheme,
};

pub struct WindowManager {
    pub show_about: bool,
    pub show_hotkeys: bool,
    pub show_settings: bool,
    pub show_agents_settings: bool,
    pub show_theme_settings: bool,
    pub show_font_settings: bool,
    pub show_rename_group: bool,
    pub show_close_confirmation: bool,
    pub rename_group_id: Option<u64>,
    pub rename_group_name: String,
    pub editing_default_shell_cmd: String,
    pub saved_default_shell_cmd: String,
    pub editing_agents: [AgentConfig; MAX_AGENTS],
    pub saved_agents: [AgentConfig; MAX_AGENTS],
    pub editing_run_as_login_shell: bool,
    pub saved_run_as_login_shell: bool,
    pub editing_theme: AppTheme,
    pub saved_theme: AppTheme,
    pub editing_fonts: AppFonts,
    pub saved_fonts: AppFonts,
    pub was_settings_open: bool,
    pub was_agents_settings_open: bool,
    pub was_theme_settings_open: bool,
    pub was_font_settings_open: bool,
}

impl WindowManager {
    pub fn new(
        default_shell_cmd: String,
        agents: [AgentConfig; MAX_AGENTS],
        run_as_login_shell: bool,
        theme: AppTheme,
    ) -> Self {
        let editing_default_shell_cmd = default_shell_cmd.clone();
        let saved_default_shell_cmd = editing_default_shell_cmd.clone();
        let editing_agents = agents.clone();
        let saved_agents = editing_agents.clone();
        let editing_run_as_login_shell = run_as_login_shell;
        let saved_run_as_login_shell = run_as_login_shell;
        let editing_theme = theme;
        let saved_theme = editing_theme;
        let editing_fonts = editing_theme.fonts;
        let saved_fonts = editing_fonts;

        Self {
            show_about: false,
            show_hotkeys: false,
            show_settings: false,
            show_agents_settings: false,
            show_theme_settings: false,
            show_font_settings: false,
            show_rename_group: false,
            show_close_confirmation: false,
            rename_group_id: None,
            rename_group_name: String::new(),
            editing_default_shell_cmd,
            saved_default_shell_cmd,
            editing_agents,
            saved_agents,
            editing_run_as_login_shell,
            saved_run_as_login_shell,
            editing_theme,
            saved_theme,
            editing_fonts,
            saved_fonts,
            was_settings_open: false,
            was_agents_settings_open: false,
            was_theme_settings_open: false,
            was_font_settings_open: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> WindowActions {
        let mut actions = WindowActions::default();

        self.show_about_window(ctx);
        self.show_hotkeys_window(ctx);
        self.show_rename_group_window(ctx, &mut actions);
        self.show_settings_window(ctx, &mut actions);
        self.show_agents_settings_window(ctx, &mut actions);
        self.show_theme_settings_window(ctx, &mut actions);
        self.show_font_settings_window(ctx, &mut actions);
        self.show_close_confirmation_window(ctx, &mut actions);

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

    fn show_rename_group_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut WindowActions,
    ) {
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

    fn show_settings_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut WindowActions,
    ) {
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

                    ui.checkbox(&mut self.editing_run_as_login_shell,
                        "Run as login shell",
                    );

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
            actions.run_as_login_shell = Some(self.editing_run_as_login_shell);
            self.saved_default_shell_cmd = self.editing_default_shell_cmd.clone();
            self.saved_run_as_login_shell = self.editing_run_as_login_shell;
            actions.should_save_settings = true;
            self.show_settings = false;
        }
        if settings_cancel {
            self.editing_default_shell_cmd = self.saved_default_shell_cmd.clone();
            self.editing_run_as_login_shell = self.saved_run_as_login_shell;
            self.show_settings = false;
        }
    }

    fn show_agents_settings_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut WindowActions,
    ) {
        let mut save = false;
        let mut cancel = false;

        let window_id = egui::Id::new("agents_settings_window");

        if self.show_agents_settings && !self.was_agents_settings_open {
            ctx.memory_mut(|m| m.request_focus(window_id));
        }
        self.was_agents_settings_open = self.show_agents_settings;

        egui::Window::new("Agents")
            .id(window_id)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_agents_settings)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Agent Settings");
                    ui.label("Configure up to 4 agents. Enabled agents appear in the sidebar.");
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical()
                        .id_salt("agents_settings_scroll")
                        .max_height(420.0)
                        .show(ui, |ui| {
                            for (i, agent) in self.editing_agents.iter_mut().enumerate() {
                                ui.push_id(i, |ui| {
                                    ui.group(|ui| {
                                        ui.label(format!("Agent {}", i + 1));

                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut agent.enabled,
                                                "Enabled",
                                            );
                                        });

                                        ui.horizontal(|ui| {
                                            ui.label("Name:");
                                            ui.text_edit_singleline(
                                                &mut agent.name,
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Command:");
                                            ui.text_edit_singleline(
                                                &mut agent.cmd,
                                            );
                                        });
                                    });
                                });
                                ui.add_space(8.0);
                            }
                        });

                    ui.add_space(10.0);

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            });

        if save {
            actions.agents = Some(self.editing_agents.clone());
            self.saved_agents = self.editing_agents.clone();
            actions.should_save_settings = true;
            self.show_agents_settings = false;
        }
        if cancel {
            self.editing_agents = self.saved_agents.clone();
            self.show_agents_settings = false;
        }
    }

    fn show_theme_settings_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut WindowActions,
    ) {
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

                    ui.heading("Terminal colors");
                    ui.add_space(6.0);
                    color_picker_button(
                        ui,
                        "Terminal foreground",
                        &mut self.editing_theme.terminal_fg,
                    );

                    ui.add_space(15.0);

                    ui.heading("Background");
                    ui.label("Used for the app and terminal background.");
                    ui.add_space(6.0);
                    color_picker_button(ui, "Background color", &mut self.editing_theme.app_bg);
                    opacity_slider(
                        ui,
                        "Background opacity",
                        &mut self.editing_theme.app_bg_opacity,
                    );

                    ui.add_space(15.0);

                    ui.heading("UI colors");
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

                    ui.add_space(15.0);

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
                            egui::RichText::new("Tab")
                                .color(self.editing_theme.tab_text),
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

                    ui.add_space(15.0);

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            save = true;
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
            self.editing_theme = AppTheme::default();
            // Apply preview immediately so the user sees the defaults.
            self.editing_theme.apply_to_visuals(ctx);
        }

        if save {
            actions.theme = Some(self.editing_theme);
            self.saved_theme = self.editing_theme;
            actions.should_save_settings = true;
            self.show_theme_settings = false;
        }
        if cancel {
            self.editing_theme = self.saved_theme;
            self.editing_theme.apply_to_visuals(ctx);
            self.show_theme_settings = false;
        }
    }

    fn show_font_settings_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut WindowActions,
    ) {
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

                    font_size_slider(
                        ui,
                        "UI font size",
                        &mut self.editing_fonts.ui_font_size,
                    );
                    ui.add_space(4.0);
                    font_size_slider(
                        ui,
                        "Group name font size",
                        &mut self.editing_fonts.group_name_font_size,
                    );
                    ui.add_space(4.0);
                    font_size_slider(
                        ui,
                        "Tab font size",
                        &mut self.editing_fonts.tab_font_size,
                    );
                    ui.add_space(4.0);
                    font_size_slider(
                        ui,
                        "Terminal font size",
                        &mut self.editing_fonts.terminal_font_size,
                    );

                    ui.add_space(15.0);

                    ui.label("Preview");
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("UI text")
                                .size(self.editing_fonts.ui_font_size),
                        );
                        ui.label(
                            egui::RichText::new("Group")
                                .size(self.editing_fonts.group_name_font_size),
                        );
                        ui.label(
                            egui::RichText::new("Tab")
                                .size(self.editing_fonts.tab_font_size),
                        );
                        ui.label(
                            egui::RichText::new("Terminal")
                                .font(egui::FontId::monospace(
                                    self.editing_fonts.terminal_font_size,
                                )),
                        );
                    });

                    ui.add_space(15.0);

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        cancel = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
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
            actions.fonts = Some(self.editing_fonts);
            self.saved_fonts = self.editing_fonts;
            actions.should_save_settings = true;
            self.show_font_settings = false;
        }
        if cancel {
            self.editing_fonts = self.saved_fonts;
            self.saved_fonts.apply(ctx);
            self.show_font_settings = false;
        }
    }

    pub fn rename_group(&mut self, group_id: u64, name: String) {
        self.rename_group_id = Some(group_id);
        self.rename_group_name = name;
        self.show_rename_group = true;
    }

    fn show_close_confirmation_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut WindowActions,
    ) {
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

#[derive(Default)]
pub struct WindowActions {
    pub rename_group: Option<(u64, String)>,
    pub default_shell_cmd: Option<String>,
    pub agents: Option<[AgentConfig; MAX_AGENTS]>,
    pub run_as_login_shell: Option<bool>,
    pub theme: Option<AppTheme>,
    pub fonts: Option<AppFonts>,
    pub should_save_groups: bool,
    pub should_save_settings: bool,
    pub close_confirmed: bool,
}

use crate::ui::screen;

impl super::WindowManager {
    pub(super) fn show_welcome_window(&mut self, ctx: &egui::Context, actions: &mut super::WindowActions) {
        let mut close = false;

        let window_id = egui::Id::new("welcome_window");
        let window_width = screen::width(ctx) * 0.55;
        let toggle_style = self.welcome_toggle_style;

        if self.show_welcome_window && !self.was_welcome_open {
            ctx.memory_mut(|m| m.request_focus(window_id));
        }
        self.was_welcome_open = self.show_welcome_window;

        egui::Window::new("Welcome to H2Term")
            .id(window_id)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(window_width)
            .open(&mut self.show_welcome_window)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("Welcome to H2Term");
                    ui.add_space(4.0);
                    ui.label("A quick tour of the main features and where to find them.");
                    ui.add_space(12.0);

                    egui::Grid::new("welcome_features_grid")
                        .num_columns(3)
                        .spacing([16.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Feature").strong());
                            ui.label(egui::RichText::new("What it does / where it is").strong());
                            ui.label(egui::RichText::new("Enabled").strong());
                            ui.end_row();

                            ui.label("📂 Sidebar");
                            ui.label(
                                "Projects with terminal and agent tabs. Show/hide it with the button in the navbar.",
                            );
                            if feature_toggle(ui, toggle_style, &mut self.welcome_sidebar) {
                                actions.welcome_sidebar = Some(self.welcome_sidebar);
                            }
                            ui.end_row();

                            ui.label("🤖 Agent buttons");
                            ui.label(
                                "Quick-launch buttons for your own agents in the sidebar. Configure them in Settings → Agents.",
                            );
                            if ui
                                .add(egui::Button::new("Configure…").min_size(egui::vec2(96.0, 32.0)))
                                .clicked()
                            {
                                self.show_agents_settings = true;
                            }
                            ui.end_row();

                            ui.label("🔀 Git status");
                            ui.label("Branch and pending changes next to each project in the sidebar.");
                            if feature_toggle(ui, toggle_style, &mut self.welcome_git_status) {
                                actions.welcome_git_status = Some(self.welcome_git_status);
                            }
                            ui.end_row();

                            ui.label("🖥 System monitor");
                            ui.label(
                                "RAM usage and total memory used by terminal tabs, shown in the navbar.",
                            );
                            if feature_toggle(ui, toggle_style, &mut self.welcome_system_monitor) {
                                actions.welcome_system_monitor = Some(self.welcome_system_monitor);
                            }
                            ui.end_row();

                            ui.label("⌨ Hotkeys");
                            ui.label("All keyboard shortcuts: tab switching, search, project finder…");
                            if ui
                                .add(egui::Button::new("Open").min_size(egui::vec2(96.0, 32.0)))
                                .clicked()
                            {
                                self.show_hotkeys = true;
                            }
                            ui.end_row();

                            ui.label("🔍 Search & finder");
                            ui.label(
                                "Ctrl+F to search inside the terminal, Ctrl+Shift+O for the fuzzy project finder.",
                            );
                            ui.label("");
                            ui.end_row();
                        });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Show this window at startup");
                        if feature_toggle(ui, toggle_style, &mut self.welcome_show_at_startup) {
                            actions.welcome_show_at_startup = Some(self.welcome_show_at_startup);
                        }
                    });

                    ui.add_space(10.0);

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("Get Started").min_size(egui::vec2(110.0, 32.0)))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            });

        if close {
            self.show_welcome_window = false;
        }
    }
}

/// Big on/off toggle button used by the welcome window feature rows.
/// Returns true when the state changed this frame.
fn feature_toggle(ui: &mut egui::Ui, style: crate::theme::AppButtonStyle, on: &mut bool) -> bool {
    style.apply_to_visuals(ui);
    let label = if *on { "✓ On" } else { "Off" };
    let response = ui.add(
        egui::Button::new(egui::RichText::new(label).strong()).min_size(egui::vec2(80.0, 32.0)),
    );
    if response.clicked() {
        *on = !*on;
        true
    } else {
        false
    }
}

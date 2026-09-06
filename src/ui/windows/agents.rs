use super::WindowActions;

impl super::WindowManager {
    pub(super) fn show_agents_settings_window(&mut self, ctx: &egui::Context, actions: &mut WindowActions) {
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
                    ui.label(
                        egui::RichText::new("Absolute path required")
                            .color(egui::Color32::from_rgb(100, 150, 255)),
                    );
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
                                            ui.checkbox(&mut agent.enabled, "Enabled");
                                        });

                                        ui.horizontal(|ui| {
                                            ui.label("Name:");
                                            ui.text_edit_singleline(&mut agent.name);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Command:");
                                            ui.text_edit_singleline(&mut agent.cmd);
                                        });
                                    });
                                });
                                ui.add_space(8.0);
                            }
                        });

                    ui.add_space(10.0);

                    if super::shortcuts_active(ctx, window_id)
                        && ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
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
            // Whitespace around names/commands is accidental input, not
            // configuration; trim it before the draft becomes the config.
            let mut agents = self.editing_agents.clone();
            for agent in &mut agents {
                agent.name = agent.name.trim().to_string();
                agent.cmd = agent.cmd.trim().to_string();
            }
            actions.agents = Some(agents);
            actions.should_save_settings = true;
            self.show_agents_settings = false;
        }
        if cancel {
            self.show_agents_settings = false;
        }
    }
}

use crate::hotkeys::get_hotkeys;

impl super::WindowManager {
    pub(super) fn show_about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                    ui.heading("H2Term byOrlov");
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(10.0);
                    ui.label("Multi-agent terminal with tabs and project management");
                    ui.label("Manage multiple agent sessions across different projects");
                    ui.add_space(10.0);
                    ui.label("Author: Oleg Orlov (orelcokolov@gmail.com)");
                });
            });
    }

    pub(super) fn show_hotkeys_window(&mut self, ctx: &egui::Context) {
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
}

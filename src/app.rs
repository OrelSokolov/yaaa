use crate::config::{RecentProjects, Settings};
use crate::hotkeys::handle_keyboard_events;
use crate::menu::apply_menu_style;
use crate::terminal::TabManager;
use crate::ui::{
    show_central_panel, show_debug_panel, show_left_panel, show_search_panel, WindowActions,
    WindowManager,
};
use egui_term::BackendCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

pub struct App {
    _command_sender: Sender<(u64, egui_term::PtyEvent)>,
    command_receiver: Receiver<(u64, egui_term::PtyEvent)>,
    tab_manager: TabManager,
    window_manager: WindowManager,
    recent_projects: RecentProjects,
    egui_ctx: egui::Context,
    pub show_terminal_lines: bool,
    pub show_fps: bool,
    pub show_sidebar: bool,
}

/// The application's default gray, used for the menu bar and (after styling)
/// for panels and windows.
const APP_GRAY: egui::Color32 = egui::Color32::from_rgb(0x20, 0x20, 0x20);

fn app_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = APP_GRAY;
    visuals.window_fill = APP_GRAY;
    visuals
}

fn setup_visuals(ctx: &egui::Context) {
    // Set both light and dark styles to the same gray look, then lock the active
    // theme to Dark. This prevents macOS's light system theme from switching the
    // UI to white after the first frame.
    let visuals = app_visuals();
    ctx.set_visuals_of(egui::Theme::Dark, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Light, visuals);
    ctx.set_theme(egui::Theme::Dark);

    // Force the native window chrome (title bar / traffic lights) to dark mode on macOS
    // so it matches the rest of the UI instead of following the system light appearance.
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Force dark theme on all platforms so the UI stays gray (matching the
        // menu bar) regardless of the system light/dark appearance.
        setup_visuals(&cc.egui_ctx);

        // Setup fonts with optional system fallback
        crate::font_setup::setup_fonts_with_fallback(&cc.egui_ctx);

        let (command_sender, command_receiver) = mpsc::channel();
        let command_sender_clone = command_sender.clone();

        let settings = Settings::load();

        let tab_manager = TabManager::new(
            command_sender_clone,
            cc,
            settings.default_shell_cmd.clone(),
            settings.default_agent_cmd.clone(),
            settings.run_as_login_shell,
        );

        let window_manager = WindowManager::new(
            settings.default_shell_cmd.clone(),
            settings.default_agent_cmd.clone(),
            settings.run_as_login_shell,
        );

        let recent_projects = RecentProjects::load();

        Self {
            _command_sender: command_sender,
            command_receiver,
            tab_manager,
            window_manager,
            recent_projects,
            egui_ctx: cc.egui_ctx.clone(),
            show_terminal_lines: settings.show_terminal_lines,
            show_fps: settings.show_fps,
            show_sidebar: settings.show_sidebar,
        }
    }

    fn save_settings(&self) {
        let settings = Settings {
            show_terminal_lines: self.show_terminal_lines,
            show_fps: self.show_fps,
            show_sidebar: self.show_sidebar,
            run_as_login_shell: self.window_manager.editing_run_as_login_shell,
            default_shell_cmd: self.window_manager.editing_default_shell_cmd.clone(),
            default_agent_cmd: self.window_manager.editing_default_agent_cmd.clone(),
        };
        settings.save();
    }

    fn save_recent_projects(&self) {
        self.recent_projects.save();
    }

    fn handle_command_events(&mut self) {
        while let Ok((tab_id, event)) = self.command_receiver.try_recv() {
            match event {
                egui_term::PtyEvent::Exit => {
                    self.tab_manager.remove(tab_id);
                }
                egui_term::PtyEvent::Title(title) => {
                    self.tab_manager.set_title(tab_id, title);
                }
                _ => {}
            }
        }
    }

    fn handle_keyboard(
        &mut self,
        ctx: &egui::Context,
    ) -> (bool, Option<u64>, Option<u64>, Option<u64>) {
        let events = handle_keyboard_events(ctx, self.tab_manager.active_group_id.is_some());

        let mut close_tab_id = None;
        let mut add_tab_to_group = None;
        let mut add_agent_tab_to_group = None;

        if events.switch_to_next_tab {
            self.tab_manager.switch_to_next_tab();
        }

        if events.switch_to_prev_tab {
            self.tab_manager.switch_to_prev_tab();
        }

        if events.add_terminal_tab {
            if let Some(group_id) = self.tab_manager.active_group_id {
                add_tab_to_group = Some(group_id);
            }
        }

        if events.add_agent_tab {
            if let Some(group_id) = self.tab_manager.active_group_id {
                add_agent_tab_to_group = Some(group_id);
            }
        }

        if events.close_tab {
            if let Some(tab_id) = self.tab_manager.active_tab_id {
                close_tab_id = Some(tab_id);
            }
        }

        if events.scroll_to_top {
            if let Some(tab) = self.tab_manager.get_active() {
                tab.backend.scroll_to_top();
            }
        }

        if events.scroll_to_bottom {
            if let Some(tab) = self.tab_manager.get_active() {
                tab.backend.scroll_to_bottom();
            }
        }

        if events.scroll_page_up {
            if let Some(tab) = self.tab_manager.get_active() {
                tab.backend.process_command(BackendCommand::ScrollPageUp);
            }
        }

        if events.scroll_page_down {
            if let Some(tab) = self.tab_manager.get_active() {
                tab.backend.process_command(BackendCommand::ScrollPageDown);
            }
        }

        if events.toggle_search {
            if let Some(tab) = self.tab_manager.get_active() {
                tab.search_active = !tab.search_active;
                tab.backend.search_set_active(tab.search_active);
                if tab.search_active {
                    tab.search_query.clear();
                    tab.search_just_opened = true;
                }
            }
        }

        (
            events.close_tab,
            close_tab_id,
            add_tab_to_group,
            add_agent_tab_to_group,
        )
    }

    fn handle_panel_actions(&mut self, ctx: &egui::Context, actions: super::ui::panels::PanelActions) {
        if actions.add_group_clicked {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let name = crate::terminal::manager::TabGroup::name_from_path(&path);
                self.recent_projects.add_project(name.clone(), path.clone());
                self.save_recent_projects();
                self.tab_manager.add_group_with_path(
                    ctx.clone(),
                    Some(path),
                );
                self.tab_manager.save_groups();
            }
        }

        if let Some(group_id) = actions.add_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), false);
            self.tab_manager.save_groups();
        }

        if let Some(group_id) = actions.add_agent_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), true);
            self.tab_manager.save_groups();
        }

        for (group_id, action_type, data) in actions.group_actions {
            match action_type.as_str() {
                "remove_group" => {
                    if let Some(group) = self.tab_manager.groups.get(&group_id) {
                        self.recent_projects
                            .add_project(group.name.clone(), group.path.clone());
                        self.save_recent_projects();
                    }
                    self.tab_manager.remove_group(group_id);
                    self.tab_manager.save_groups();
                }
                "select_tab" => {
                    if let Some(tab_id) = data.first() {
                        self.tab_manager.set_active_tab(tab_id.0);
                    }
                }
                "remove_tab" => {
                    if let Some(tab_id) = data.first() {
                        self.tab_manager.remove(tab_id.0);
                        self.tab_manager.save_groups();
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_window_actions(&mut self, actions: WindowActions) {
        if let Some((group_id, name)) = actions.rename_group {
            self.tab_manager.rename_group(group_id, name);
            self.tab_manager.save_groups();
        }

        if actions.should_save_groups {
            self.tab_manager.save_groups();
        }

        if let Some(shell_cmd) = actions.default_shell_cmd {
            self.tab_manager.set_default_shell_cmd(shell_cmd);
        }

        if let Some(agent_cmd) = actions.default_agent_cmd {
            self.tab_manager.set_default_agent_cmd(agent_cmd);
        }

        if let Some(run_as_login_shell) = actions.run_as_login_shell {
            self.tab_manager.set_run_as_login_shell(run_as_login_shell);
        }

        if actions.should_save_settings {
            self.save_settings();
        }
    }

}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Paint the window background with the same opaque gray as the rest of the UI
        // instead of the default semi-transparent clear color.
        APP_GRAY.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        if ctx.input(|i| i.viewport().close_requested()) {
            self.window_manager.show_close_confirmation = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(0x20, 0x20, 0x20),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.style_mut()
                            .text_styles
                            .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
                        ui.style_mut()
                            .text_styles
                            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));

                        egui::MenuBar::new().ui(ui, |ui| {
                            ui.style_mut().spacing.button_padding = egui::vec2(6.0, 2.0);
                            ui.style_mut()
                                .text_styles
                                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));

                            ui.menu_button("Yet Another AI Agent", |ui| {
                                apply_menu_style(ui);

                                if ui.button("ℹ About").clicked() {
                                    self.window_manager.show_about = true;
                                    ui.close();
                                }
                            });
                            ui.menu_button("Projects", |ui| {
                                apply_menu_style(ui);

                                if ui.button("➕ Add project").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        let name =
                                            crate::terminal::manager::TabGroup::name_from_path(
                                                &path,
                                            );
                                        self.recent_projects
                                            .add_project(name.clone(), path.clone());
                                        self.save_recent_projects();
                                        self.tab_manager
                                            .add_group_with_path(ctx.clone(), Some(path));
                                        self.tab_manager.save_groups();
                                    }
                                    ui.close();
                                }

                                ui.separator();

                                let opened_paths: std::collections::HashSet<_> = self
                                    .tab_manager
                                    .groups
                                    .values()
                                    .map(|g| g.path.clone())
                                    .collect();

                                let recent_projects: Vec<_> = self
                                    .recent_projects
                                    .projects
                                    .iter()
                                    .filter(|p| !opened_paths.contains(&p.path))
                                    .collect();

                                if !recent_projects.is_empty() {
                                    for project in recent_projects {
                                        if ui.button(&project.name).clicked() {
                                            self.tab_manager.add_group_with_path(
                                                ctx.clone(),
                                                Some(project.path.clone()),
                                            );
                                            self.tab_manager.save_groups();
                                            ui.close();
                                        }
                                    }
                                } else {
                                    ui.label("No recent projects");
                                }
                            });
                            ui.menu_button("Settings", |ui| {
                                apply_menu_style(ui);

                                ui.menu_button("🔧 General", |ui| {
                                    apply_menu_style(ui);
                                    if ui.button("💻 Terminal").clicked() {
                                        self.window_manager.show_settings = true;
                                        ui.close();
                                    }
                                });

                                ui.separator();

                                ui.menu_button("🐛 Debug", |ui| {
                                    apply_menu_style(ui);

                                    if ui
                                        .button(if self.show_terminal_lines {
                                            "🚫 Hide terminal lines"
                                        } else {
                                            "📊 Show terminal lines"
                                        })
                                        .clicked()
                                    {
                                        self.show_terminal_lines = !self.show_terminal_lines;
                                        self.save_settings();
                                    }
                                    if ui
                                        .button(if self.show_fps {
                                            "🚫 Hide FPS"
                                        } else {
                                            "⚡ Show FPS"
                                        })
                                        .clicked()
                                    {
                                        self.show_fps = !self.show_fps;
                                        self.save_settings();
                                    }
                                });
                            });
                            ui.menu_button("Help", |ui| {
                                apply_menu_style(ui);
                                if ui.button("⌘ Hotkeys").clicked() {
                                    self.window_manager.show_hotkeys = true;
                                    ui.close();
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let btn_text = if self.show_sidebar {
                                        "📂 Hide Sidebar"
                                    } else {
                                        "📂 Show Sidebar"
                                    };
                                    if ui
                                        .button(btn_text)
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.show_sidebar = !self.show_sidebar;
                                        self.save_settings();
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(4.0);
                });
            });

        let window_actions = self.window_manager.show(ctx);

        let panel_actions = show_left_panel(
            ctx,
            &self.tab_manager,
            &mut self.window_manager,
            self.show_sidebar,
        );

        show_debug_panel(
            ctx,
            self.show_fps,
            self.show_terminal_lines,
            &mut self.tab_manager,
        );

        show_search_panel(ctx, &mut self.tab_manager);

        let (_close_tab, close_tab_id, add_tab_to_group, add_agent_tab_to_group) =
            self.handle_keyboard(ctx);

        self.handle_command_events();

        self.handle_panel_actions(ctx, panel_actions);

        if window_actions.close_confirmed {
            self.tab_manager.clear();
            std::process::exit(0);
        }

        self.handle_window_actions(window_actions);

        if let Some(tab_id) = close_tab_id {
            self.tab_manager.remove(tab_id);
            self.tab_manager.save_groups();
        }

        if let Some(group_id) = add_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), false);
            self.tab_manager.save_groups();
        }

        if let Some(group_id) = add_agent_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), true);
            self.tab_manager.save_groups();
        }

        show_central_panel(ctx, &mut self.tab_manager, &self.window_manager);

        // The terminal backend updates its state on a background PTY thread, but
        // alacritty_terminal does not emit events for ordinary screen output.
        // Without a pending repaint request egui/eframe on macOS goes to sleep
        // between input events, so terminal output appears to "stick" until a
        // key is pressed or the mouse moves. Schedule the next repaint so the
        // active tab stays live.
        if self.tab_manager.active_tab_id.is_some() {
            let viewport = ctx.input(|i| i.viewport().clone());
            if viewport.visible().unwrap_or(true) {
                let delay = if viewport.focused.unwrap_or(true) {
                    Duration::from_millis(16)
                } else {
                    Duration::from_millis(100)
                };
                ctx.request_repaint_after(delay);
            }
        }
    }
}

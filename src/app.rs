use crate::config::{RecentProjects, Settings};
use crate::hotkeys::handle_keyboard_events;
use crate::menu::apply_menu_style;
use crate::terminal::TabManager;
use crate::theme::AppTheme;
use crate::ui::{
    show_central_panel, show_debug_panel, show_left_panel, show_search_panel, GroupAction,
    PanelActions, WindowActions, WindowManager,
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
    theme: AppTheme,
    cached_terminal_theme: egui_term::TerminalTheme,
    cached_terminal_font: egui_term::TerminalFont,
    /// When the theme settings window is open, this holds the live-preview theme
    /// so that `clear_color` can reflect opacity changes immediately.
    preview_theme: Option<AppTheme>,
    exit_confirmed: bool,
}

fn setup_visuals(ctx: &egui::Context, theme: &AppTheme) {
    // Set both light and dark styles to the same look, then lock the active
    // theme to Dark. This prevents macOS's light system theme from switching the
    // UI to white after the first frame.
    let visuals = theme.visuals();
    ctx.set_visuals_of(egui::Theme::Dark, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Light, visuals);
    ctx.set_theme(egui::Theme::Dark);

    // Force the native window chrome (title bar / traffic lights) to dark mode on macOS
    // so it matches the rest of the UI instead of following the system light appearance.
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        let theme = settings.theme;

        // Force dark theme on all platforms so the UI stays consistent
        // regardless of the system light/dark appearance.
        setup_visuals(&cc.egui_ctx, &theme);

        // Setup fonts with optional system fallback
        crate::font_setup::setup_fonts_with_fallback(&cc.egui_ctx);

        // Apply the configured font sizes on top of the default font definitions.
        theme.fonts.apply(&cc.egui_ctx);

        let (command_sender, command_receiver) = mpsc::channel();
        let command_sender_clone = command_sender.clone();

        let tab_manager = TabManager::new(
            command_sender_clone,
            cc,
            settings.default_shell_cmd.clone(),
            settings.agents.clone(),
            settings.run_as_login_shell,
        );

        let window_manager = WindowManager::new(
            settings.default_shell_cmd.clone(),
            settings.agents.clone(),
            settings.run_as_login_shell,
            theme,
        );

        let recent_projects = RecentProjects::load();
        let cached_terminal_theme = theme.build_terminal_theme();
        let cached_terminal_font = theme.terminal_font();

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
            theme,
            cached_terminal_theme,
            cached_terminal_font,
            preview_theme: None,
            exit_confirmed: false,
        }
    }

    fn save_settings(&self) {
        let settings = Settings {
            show_terminal_lines: self.show_terminal_lines,
            show_fps: self.show_fps,
            show_sidebar: self.show_sidebar,
            run_as_login_shell: self.window_manager.editing_run_as_login_shell,
            default_shell_cmd: self.window_manager.editing_default_shell_cmd.clone(),
            agents: self.window_manager.editing_agents.clone(),
            legacy_default_agent_cmd: None,
            theme: self.theme,
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
    ) -> (Option<u64>, Option<u64>, Vec<(u64, usize)>) {
        let events = handle_keyboard_events(ctx, self.tab_manager.active_group_id.is_some());

        let mut close_tab_id = None;
        let mut add_tab_to_group = None;
        let mut add_agent_tab_to_group = Vec::new();

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

        if let Some(agent_index) = events.add_agent_tab {
            if let Some(group_id) = self.tab_manager.active_group_id {
                add_agent_tab_to_group.push((group_id, agent_index));
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
            close_tab_id,
            add_tab_to_group,
            add_agent_tab_to_group,
        )
    }

    fn handle_panel_actions(
        &mut self,
        ctx: &egui::Context,
        actions: PanelActions,
    ) {
        if actions.add_group_clicked {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let name = crate::terminal::manager::TabGroup::name_from_path(&path);
                self.recent_projects.add_project(name.clone(), path.clone());
                self.save_recent_projects();
                self.tab_manager
                    .add_group_with_path(ctx.clone(), Some(path));
                self.tab_manager.save_groups();
            }
        }

        if let Some(group_id) = actions.add_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), None);
            self.tab_manager.save_groups();
        }

        for (group_id, agent_index) in actions.add_agent_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), Some(agent_index));
            self.tab_manager.save_groups();
        }

        for (group_id, action) in actions.group_actions {
            match action {
                GroupAction::RemoveGroup => {
                    if let Some(group) = self.tab_manager.groups.get(&group_id) {
                        self.recent_projects
                            .add_project(group.name.clone(), group.path.clone());
                        self.save_recent_projects();
                    }
                    self.tab_manager.remove_group(group_id);
                    self.tab_manager.save_groups();
                }
                GroupAction::SelectTab(tab_id) => {
                    self.tab_manager.set_active_tab(tab_id);
                }
                GroupAction::RemoveTab(tab_id) => {
                    self.tab_manager.remove(tab_id);
                    self.tab_manager.save_groups();
                }
            }
        }
    }

    fn rebuild_terminal_cache(&mut self) {
        self.cached_terminal_theme = self.theme.build_terminal_theme();
        self.cached_terminal_font = self.theme.terminal_font();
    }

    fn effective_theme(&self) -> &AppTheme {
        self.preview_theme.as_ref().unwrap_or(&self.theme)
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

        if let Some(agents) = actions.agents {
            self.tab_manager.set_agents(agents);
        }

        if let Some(run_as_login_shell) = actions.run_as_login_shell {
            self.tab_manager.set_run_as_login_shell(run_as_login_shell);
        }

        if let Some(theme) = actions.theme {
            self.theme = theme;
            self.preview_theme = None;
            self.rebuild_terminal_cache();
            setup_visuals(&self.egui_ctx, &self.theme);
            self.theme.fonts.apply(&self.egui_ctx);
            self.egui_ctx.request_repaint();
            self.window_manager.last_applied_opacity = self.theme.app_bg_opacity;
        }

        if let Some(fonts) = actions.fonts {
            self.theme.fonts = fonts;
            self.rebuild_terminal_cache();
            self.theme.fonts.apply(&self.egui_ctx);
        }

        if actions.should_save_settings {
            self.save_settings();
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let color = self
            .preview_theme
            .unwrap_or(self.theme)
            .app_bg_with_opacity();
        let a = color.a() as f32 / 255.0;
        // Return straight (unmultiplied) alpha so the compositor blends the
        // background color correctly against the desktop.
        [
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
            a,
        ]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.exit_confirmed {
                return;
            }
            self.window_manager.show_close_confirmation = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // Keep the live preview in sync while the theme settings window is open.
        if self.window_manager.show_theme_settings {
            self.window_manager.editing_theme.apply_to_visuals(&ctx);
            let opacity = self.window_manager.editing_theme.app_bg_opacity;
            if opacity != self.window_manager.last_applied_opacity {
                self.window_manager.last_applied_opacity = opacity;
                self.preview_theme = Some(self.window_manager.editing_theme);
                let transparent = opacity < 100;
                ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(transparent));
                ctx.request_repaint();
            }
        } else {
            self.preview_theme = None;
        }

        let theme = *self.effective_theme();

        egui::Panel::top("menu_bar")
            .frame(egui::Frame {
                fill: theme.app_bg_with_opacity(),
                ..Default::default()
            })
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Button,
                            egui::FontId::proportional(theme.fonts.ui_font_size),
                        );
                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Body,
                            egui::FontId::proportional(theme.fonts.ui_font_size),
                        );

                        egui::MenuBar::new().ui(ui, |ui| {
                            ui.style_mut().spacing.button_padding = egui::vec2(6.0, 2.0);
                            ui.style_mut().text_styles.insert(
                                egui::TextStyle::Button,
                                egui::FontId::proportional(theme.fonts.ui_font_size),
                            );

                            ui.menu_button("Yet Another AI Agent", |ui| {
                                apply_menu_style(ui, theme.fonts.ui_font_size);

                                if ui.button("ℹ About").clicked() {
                                    self.window_manager.show_about = true;
                                    ui.close();
                                }
                            });
                            ui.menu_button("Projects", |ui| {
                                apply_menu_style(ui, theme.fonts.ui_font_size);

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
                                apply_menu_style(ui, theme.fonts.ui_font_size);

                                if ui.button("🎨 Theme").clicked() {
                                    self.window_manager.show_theme_settings = true;
                                    ui.close();
                                }

                                if ui.button("🔤 Fonts").clicked() {
                                    self.window_manager.show_font_settings = true;
                                    ui.close();
                                }

                                ui.separator();

                                if ui.button("💻 Terminal").clicked() {
                                    self.window_manager.show_settings = true;
                                    ui.close();
                                }
                                if ui.button("💬 Agents").clicked() {
                                    self.window_manager.show_agents_settings = true;
                                    ui.close();
                                }

                                ui.separator();

                                ui.menu_button("🐛 Debug", |ui| {
                                    apply_menu_style(ui, theme.fonts.ui_font_size);

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
                                apply_menu_style(ui, theme.fonts.ui_font_size);
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

        let window_actions = self.window_manager.show(&ctx);

        let panel_actions = show_left_panel(
            ui,
            &self.tab_manager,
            &mut self.window_manager,
            self.show_sidebar,
            &self.tab_manager.agents,
            &theme,
        );

        show_debug_panel(
            ui,
            self.show_fps,
            self.show_terminal_lines,
            &mut self.tab_manager,
            &theme,
        );

        show_search_panel(ui, &mut self.tab_manager, &theme);

        let (close_tab_id, add_tab_to_group, add_agent_tab_to_group) =
            self.handle_keyboard(&ctx);

        self.handle_command_events();

        self.handle_panel_actions(&ctx, panel_actions);

        if window_actions.close_confirmed {
            self.tab_manager.clear();
            self.exit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.handle_window_actions(window_actions);

        if let Some(tab_id) = close_tab_id {
            self.tab_manager.remove(tab_id);
            self.tab_manager.save_groups();
        }

        if let Some(group_id) = add_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), None);
            self.tab_manager.save_groups();
        }

        for (group_id, agent_index) in add_agent_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), Some(agent_index));
            self.tab_manager.save_groups();
        }

        show_central_panel(
            ui,
            &mut self.tab_manager,
            &self.window_manager,
            &theme,
            &self.cached_terminal_theme,
            &self.cached_terminal_font,
        );

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
                    Duration::from_millis(500)
                } else {
                    Duration::from_millis(1000)
                };
                ctx.request_repaint_after(delay);
            }
        }
    }
}

impl AppTheme {
    /// Build egui visuals from this theme. Used during startup and after
    /// restoring defaults.
    fn visuals(&self) -> egui::Visuals {
        let mut visuals = egui::Visuals::dark();
        let app_bg = self.app_bg_with_opacity();
        visuals.panel_fill = app_bg;
        visuals.window_fill = app_bg;
        visuals.widgets.inactive.bg_fill = app_bg;
        visuals.widgets.noninteractive.bg_fill = app_bg;
        visuals.override_text_color = Some(self.panel_text);
        visuals.selection.bg_fill = self.tab_active_bg;
        visuals.selection.stroke.color = self.tab_active_bg;
        visuals
    }
}

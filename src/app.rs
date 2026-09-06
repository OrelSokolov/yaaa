use crate::config::{RecentProjects, Settings, TerminalLaunchConfig};
use crate::git_status::GitStatusCache;
use crate::hotkeys::handle_keyboard_events;
use crate::system_monitor::SystemMonitor;
use crate::terminal::{TabManager, TerminalLayoutTracker};
use crate::theme::{setup_visuals, AppTheme};
use crate::ui::{
    show_central_panel, show_debug_panel, show_left_panel, show_menu_bar, show_search_panel,
    GroupAction, MenuActions, MenuBarView, PanelActions, ProjectFinder, WindowActions,
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
    project_finder: ProjectFinder,
    recent_projects: RecentProjects,
    egui_ctx: egui::Context,
    pub show_terminal_lines: bool,
    pub show_fps: bool,
    pub show_sidebar: bool,
    pub show_system_monitor: bool,
    pub show_tab_memory: bool,
    theme: AppTheme,
    cached_terminal_theme: egui_term::TerminalTheme,
    cached_terminal_font: egui_term::TerminalFont,
    git_cache: GitStatusCache,
    enable_git_status: bool,
    /// Single source of truth for how new terminals are spawned. `Settings`
    /// is its serialized form; `TabManager` and the settings dialogs hold
    /// copies synced through `update_launch_config`.
    launch_config: TerminalLaunchConfig,
    system_monitor: SystemMonitor,
    /// When the theme settings window is open, this holds the live-preview theme
    /// so that `clear_color` can reflect opacity changes immediately.
    preview_theme: Option<AppTheme>,
    exit_confirmed: bool,
    /// Tracks the persisted terminal content size and font cell metrics,
    /// debouncing writes to the settings file during resizes.
    layout_tracker: TerminalLayoutTracker,
    /// Result channel of a folder picker opened on a background thread, so the
    /// UI keeps rendering while the native dialog is shown.
    folder_pick: Option<Receiver<Option<std::path::PathBuf>>>,
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

        let cached_terminal_font = theme.terminal_font();
        // Font metrics cannot be queried yet (egui needs a Context::run() first),
        // so seed from persisted values; they are recomputed on the first frame.
        let terminal_layout_hint = settings
            .last_terminal_layout
            .map(|[w, h]| egui_term::Size::new(w, h));
        let cell_metrics_hint = settings
            .last_terminal_cell_metrics
            .map(|[w, h]| egui_term::Size::new(w, h));

        let launch_config = TerminalLaunchConfig::from_settings(&settings);

        let tab_manager = TabManager::new(
            command_sender_clone,
            cc,
            &launch_config,
            terminal_layout_hint,
            cell_metrics_hint,
        );

        let window_manager = WindowManager::new(theme);

        let recent_projects = RecentProjects::load();

        let git_cache = GitStatusCache::new(Duration::from_secs(5));

        let cached_terminal_theme = theme.build_terminal_theme();

        Self {
            _command_sender: command_sender,
            command_receiver,
            tab_manager,
            window_manager,
            project_finder: ProjectFinder::new(),
            recent_projects,
            egui_ctx: cc.egui_ctx.clone(),
            show_terminal_lines: settings.show_terminal_lines,
            show_fps: settings.show_fps,
            show_sidebar: settings.show_sidebar,
            show_system_monitor: settings.show_system_monitor,
            show_tab_memory: settings.show_tab_memory,
            theme,
            cached_terminal_theme,
            cached_terminal_font,
            git_cache,
            enable_git_status: settings.enable_git_status,
            launch_config,
            system_monitor: SystemMonitor::new(),
            preview_theme: None,
            exit_confirmed: false,
            layout_tracker: TerminalLayoutTracker::new(
                settings.last_terminal_layout,
                settings.last_terminal_cell_metrics,
            ),
            folder_pick: None,
        }
    }

    fn save_settings(&self) {
        let mut settings = Settings {
            show_terminal_lines: self.show_terminal_lines,
            show_fps: self.show_fps,
            show_sidebar: self.show_sidebar,
            show_system_monitor: self.show_system_monitor,
            show_tab_memory: self.show_tab_memory,
            legacy_default_agent_cmd: None,
            theme: self.theme,
            enable_git_status: self.enable_git_status,
            last_terminal_layout: self.layout_tracker.last_layout(),
            last_terminal_cell_metrics: self.layout_tracker.last_cell_metrics(),
            ..Default::default()
        };
        self.launch_config.apply_to_settings(&mut settings);
        settings.save();
    }

    fn save_recent_projects(&self) {
        self.recent_projects.save();
    }

    /// Open the native folder picker on a background thread so the UI thread
    /// (and the terminal PTYs) keep running while the dialog is shown.
    /// The result is polled every frame by `poll_folder_pick`.
    fn spawn_folder_pick(&mut self, ctx: &egui::Context) {
        if self.folder_pick.is_some() {
            return; // a picker is already open
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new().pick_folder();
            let _ = tx.send(picked);
            // Wake the UI thread so the result is applied immediately.
            ctx.request_repaint();
        });
        self.folder_pick = Some(rx);
    }

    fn poll_folder_pick(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.folder_pick else { return };
        match rx.try_recv() {
            Ok(Some(path)) => {
                self.folder_pick = None;
                self.add_project_from_path(ctx, path);
            }
            Ok(None) => self.folder_pick = None,
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => self.folder_pick = None,
        }
    }

    fn add_project_from_path(&mut self, ctx: &egui::Context, path: std::path::PathBuf) {
        let name = crate::terminal::manager::TabGroup::name_from_path(&path);
        self.recent_projects.add_project(name.clone(), path.clone());
        self.save_recent_projects();
        self.tab_manager
            .add_group_with_path(ctx.clone(), Some(path));
    }

    fn handle_command_events(&mut self) {
        while let Ok((tab_id, event)) = self.command_receiver.try_recv() {
            match event {
                egui_term::PtyEvent::Exit => {
                    self.tab_manager.remove(tab_id);
                    self.tab_manager.remove_preload_tab(tab_id);
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

        if events.open_project_finder {
            self.project_finder.open();
        }

        (
            close_tab_id,
            add_tab_to_group,
            add_agent_tab_to_group,
        )
    }

    fn handle_menu_actions(&mut self, ctx: &egui::Context, actions: MenuActions) {
        if actions.add_project {
            self.spawn_folder_pick(ctx);
        }

        if let Some(project) = actions.open_project {
            if project.path.exists() {
                self.tab_manager
                    .add_group_with_path(ctx.clone(), Some(project.path));
            } else {
                self.recent_projects.remove_project(&project.path);
                self.save_recent_projects();
                self.window_manager.missing_folder(format!(
                    "{}\n{}",
                    project.name,
                    project.path.display()
                ));
            }
        }

        if actions.show_about {
            self.window_manager.show_about = true;
        }
        if actions.show_theme_settings {
            self.window_manager.begin_theme_edit(&self.theme);
            self.window_manager.show_theme_settings = true;
        }
        if actions.show_font_settings {
            self.window_manager.begin_font_edit(&self.theme.fonts);
            self.window_manager.show_font_settings = true;
        }
        if actions.show_terminal_settings {
            self.window_manager
                .begin_settings_edit(&self.launch_config, self.enable_git_status);
            self.window_manager.show_settings = true;
        }
        if actions.show_agents_settings {
            self.window_manager.begin_agents_edit(&self.launch_config);
            self.window_manager.show_agents_settings = true;
        }
        if actions.show_hotkeys {
            self.window_manager.show_hotkeys = true;
        }

        if actions.toggle_git_status {
            self.enable_git_status = !self.enable_git_status;
            self.save_settings();
        }

        if actions.toggle_preload_tabs {
            self.launch_config.preload_tabs = !self.launch_config.preload_tabs;
            self.tab_manager
                .update_launch_config(&self.launch_config, ctx);
            self.save_settings();
        }

        if actions.toggle_system_monitor {
            self.show_system_monitor = !self.show_system_monitor;
            self.save_settings();
        }

        if actions.toggle_terminal_lines {
            self.show_terminal_lines = !self.show_terminal_lines;
            self.save_settings();
        }

        if actions.toggle_fps {
            self.show_fps = !self.show_fps;
            self.save_settings();
        }

        if actions.toggle_sidebar {
            self.show_sidebar = !self.show_sidebar;
            self.save_settings();
        }

        if actions.toggle_tab_memory {
            self.show_tab_memory = !self.show_tab_memory;
            self.save_settings();
        }
    }

    fn handle_panel_actions(
        &mut self,
        ctx: &egui::Context,
        actions: PanelActions,
    ) {
        if actions.add_group_clicked {
            self.spawn_folder_pick(ctx);
        }

        if let Some(group_id) = actions.add_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), None);
        }

        for (group_id, agent_index) in actions.add_agent_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), Some(agent_index));
        }

        for (group_id, action) in actions.group_actions {
            match action {
                GroupAction::RemoveGroup => {
                    if let Some(group) = self.tab_manager.group(group_id) {
                        self.recent_projects
                            .add_project(group.name.clone(), group.path.clone());
                        self.save_recent_projects();
                    }
                    self.tab_manager.remove_group(group_id);
                }
                GroupAction::SelectTab(tab_id) => {
                    self.tab_manager.set_active_tab(tab_id);
                }
                GroupAction::RemoveTab(tab_id) => {
                    self.tab_manager.remove(tab_id);
                }
                GroupAction::ToggleImportant(tab_id) => {
                    self.tab_manager.toggle_important(tab_id);
                }
            }
        }
    }

    fn rebuild_terminal_cache(&mut self, ctx: &egui::Context) {
        self.cached_terminal_theme = self.theme.build_terminal_theme();
        self.cached_terminal_font = self.theme.terminal_font();
        let metrics = self.cached_terminal_font.font_measure(ctx);
        self.tab_manager.set_cell_metrics_hint(metrics);
        self.layout_tracker.note_cell_metrics([metrics.width, metrics.height]);
    }

    fn effective_theme(&self) -> &AppTheme {
        self.preview_theme.as_ref().unwrap_or(&self.theme)
    }

    fn sync_git_paths(&mut self) {
        let paths: Vec<_> = self.tab_manager
            .iter_groups()
            .map(|g| g.path.clone())
            .collect();
        self.git_cache.retain(|p| paths.iter().any(|q| q == p));
    }

    fn handle_window_actions(&mut self, actions: WindowActions) {
        let mut config_changed = false;

        if let Some((group_id, name)) = actions.rename_group {
            self.tab_manager.rename_group(group_id, name);
        }

        if let Some(shell_cmd) = actions.default_shell_cmd {
            self.launch_config.default_shell_cmd = shell_cmd;
            config_changed = true;
        }

        if let Some(agents) = actions.agents {
            self.launch_config.agents = agents;
            config_changed = true;
        }

        if let Some(run_as_login_shell) = actions.run_as_login_shell {
            self.launch_config.run_as_login_shell = run_as_login_shell;
            config_changed = true;
        }

        if let Some(enable_git_status) = actions.enable_git_status {
            self.enable_git_status = enable_git_status;
        }

        if let Some(preload_tabs) = actions.preload_tabs {
            self.launch_config.preload_tabs = preload_tabs;
            config_changed = true;
        }

        if config_changed {
            self.tab_manager
                .update_launch_config(&self.launch_config, &self.egui_ctx);
        }

        if let Some(theme) = actions.theme {
            self.theme = theme;
            self.preview_theme = None;
            let ctx = self.egui_ctx.clone();
            self.rebuild_terminal_cache(&ctx);
            setup_visuals(&self.egui_ctx, &self.theme);
            self.theme.fonts.apply(&self.egui_ctx);
            self.egui_ctx.request_repaint();
            self.window_manager.last_applied_opacity = self.theme.app_bg_opacity;
        }

        if let Some(fonts) = actions.fonts {
            self.theme.fonts = fonts;
            let ctx = self.egui_ctx.clone();
            self.rebuild_terminal_cache(&ctx);
            self.theme.fonts.apply(&self.egui_ctx);
        }

        // The theme/font windows were closed without saving: roll the live
        // preview back to the applied theme.
        if actions.theme_discarded {
            self.preview_theme = None;
            setup_visuals(&self.egui_ctx, &self.theme);
            self.theme.fonts.apply(&self.egui_ctx);
            self.window_manager.last_applied_opacity = self.theme.app_bg_opacity;
            let transparent = self.theme.app_bg_opacity < 100;
            self.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Transparent(transparent));
            self.egui_ctx.request_repaint();
        }

        if actions.fonts_discarded {
            self.theme.fonts.apply(&self.egui_ctx);
            self.egui_ctx.request_repaint();
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

        // Sample the system monitor once per frame. `memory()` refreshes at
        // most every second; the per-tab sum walks process trees (cached, so it
        // is cheap after the first call). Both values are copied out so there
        // is no lingering borrow of `system_monitor` inside the UI closures.
        let mem_percent = self.system_monitor.memory().percent;
        let total_tabs_kb: u64 = {
            let tm = &self.tab_manager;
            let sm = &mut self.system_monitor;
            tm.iter_groups()
                .flat_map(|g| g.tabs.iter())
                .filter_map(|t| tm.get_tab(t.id))
                .map(|tab| sm.process_tree_memory_kb(tab.backend.pty_id()))
                .sum()
        };

        let open_paths: std::collections::HashSet<_> = self
            .tab_manager
            .iter_groups()
            .map(|g| g.path.clone())
            .collect();

        let menu_actions = show_menu_bar(
            ui,
            MenuBarView {
                theme: &theme,
                enable_git_status: self.enable_git_status,
                preload_tabs: self.launch_config.preload_tabs,
                show_sidebar: self.show_sidebar,
                show_system_monitor: self.show_system_monitor,
                show_terminal_lines: self.show_terminal_lines,
                show_fps: self.show_fps,
                open_paths: &open_paths,
                recent_projects: &self.recent_projects.projects,
                ram_percent: mem_percent,
                total_tabs_kb,
            },
        );

        self.handle_menu_actions(&ctx, menu_actions);

        let window_actions = self.window_manager.show(&ctx);

        // Fuzzy project finder (Ctrl+Shift+O). Same list as the Projects menu:
        // recent projects that are not currently opened.
        {
            let projects = self.recent_projects.projects.clone();
            if let Some(action) =
                self.project_finder
                    .show(&ctx, &projects, &open_paths, &theme)
            {
                if action.path.exists() {
                    self.tab_manager
                        .add_group_with_path(ctx.clone(), Some(action.path));
                } else {
                    self.recent_projects.remove_project(&action.path);
                    self.save_recent_projects();
                    self.window_manager.missing_folder(format!(
                        "{}\n{}",
                        action.name,
                        action.path.display()
                    ));
                }
            }
        }

        let panel_actions = show_left_panel(
            ui,
            &self.tab_manager,
            &mut self.window_manager,
            self.show_sidebar,
            &self.launch_config.agents,
            &theme,
            &mut self.git_cache,
            self.enable_git_status,
            self.show_tab_memory,
            &mut self.system_monitor,
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

        self.poll_folder_pick(&ctx);

        if window_actions.close_confirmed {
            self.tab_manager.clear();
            self.exit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.handle_window_actions(window_actions);

        if let Some(tab_id) = close_tab_id {
            self.tab_manager.remove(tab_id);
        }

        if let Some(group_id) = add_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), None);
        }

        for (group_id, agent_index) in add_agent_tab_to_group {
            self.tab_manager
                .add_tab_to_group(group_id, ctx.clone(), Some(agent_index));
        }

        // One flush per frame covers every mutation above; mutating methods
        // mark the session dirty themselves.
        self.tab_manager.save_groups_if_dirty();

        show_central_panel(
            ui,
            &mut self.tab_manager,
            &self.window_manager,
            self.project_finder.is_open,
            &theme,
            &self.cached_terminal_theme,
            &self.cached_terminal_font,
        );

        // Lazily compute real font cell metrics on the first frame (egui fonts
        // are not available during App::new) and seed the terminal hint from
        // them so newly created tabs boot at the correct column/row count.
        if self.layout_tracker.last_cell_metrics().is_none() {
            let metrics = self.cached_terminal_font.font_measure(&ctx);
            self.layout_tracker.note_cell_metrics([metrics.width, metrics.height]);
            self.tab_manager.set_cell_metrics_hint(metrics);
        }

        // Persist the terminal content size (debounced) so new terminals boot at
        // the correct column/row count on the next cold start.
        if let Some(hint) = self.tab_manager.terminal_layout_hint() {
            self.layout_tracker.note_layout([hint.width, hint.height]);
            if self.layout_tracker.should_flush() {
                self.save_settings();
            }
        }

        // Keep the Git watcher in sync with the current list of project groups.
        // The service compares paths internally and does nothing if nothing changed.
        self.sync_git_paths();

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

mod about;
mod agents;
mod fonts;
mod rename;
mod settings;
mod theme;

use crate::config::settings::{AgentConfig, MAX_AGENTS};
use crate::config::TerminalLaunchConfig;
use crate::theme::{AppFonts, AppTheme};

pub struct WindowManager {
    pub show_about: bool,
    pub show_hotkeys: bool,
    pub show_settings: bool,
    pub show_agents_settings: bool,
    pub show_theme_settings: bool,
    pub show_font_settings: bool,
    pub show_rename_group: bool,
    pub show_close_confirmation: bool,
    pub show_missing_folder: bool,
    pub missing_folder_message: String,
    pub rename_group_id: Option<u64>,
    pub rename_group_name: String,
    // Draft state for the settings windows. Each draft is seeded from the
    // owning state when its window opens (begin_*_edit) and reported back
    // through WindowActions on Save; Cancel just closes the window, and the
    // draft is re-seeded on the next open.
    pub editing_default_shell_cmd: String,
    pub editing_agents: [AgentConfig; MAX_AGENTS],
    pub editing_run_as_login_shell: bool,
    pub editing_enable_git_status: bool,
    pub editing_preload_tabs: bool,
    pub editing_theme: AppTheme,
    pub editing_fonts: AppFonts,
    pub was_settings_open: bool,
    pub was_agents_settings_open: bool,
    pub was_theme_settings_open: bool,
    pub was_font_settings_open: bool,
    /// Tracks the last applied opacity so we can toggle viewport transparency
    /// on the fly while the theme settings window is open.
    pub last_applied_opacity: u8,
}

impl WindowManager {
    pub fn new(theme: AppTheme) -> Self {
        let editing_fonts = theme.fonts;
        Self {
            show_about: false,
            show_hotkeys: false,
            show_settings: false,
            show_agents_settings: false,
            show_theme_settings: false,
            show_font_settings: false,
            show_rename_group: false,
            show_close_confirmation: false,
            show_missing_folder: false,
            missing_folder_message: String::new(),
            rename_group_id: None,
            rename_group_name: String::new(),
            editing_default_shell_cmd: String::new(),
            editing_agents: std::array::from_fn(|_| AgentConfig::default()),
            editing_run_as_login_shell: false,
            editing_enable_git_status: false,
            editing_preload_tabs: false,
            editing_theme: theme,
            editing_fonts,
            was_settings_open: false,
            was_agents_settings_open: false,
            was_theme_settings_open: false,
            was_font_settings_open: false,
            last_applied_opacity: theme.app_bg_opacity,
        }
    }

    /// Seed the terminal settings draft from the owning launch config.
    pub fn begin_settings_edit(
        &mut self,
        launch: &TerminalLaunchConfig,
        enable_git_status: bool,
    ) {
        self.editing_default_shell_cmd = launch.default_shell_cmd.clone();
        self.editing_run_as_login_shell = launch.run_as_login_shell;
        self.editing_enable_git_status = enable_git_status;
        self.editing_preload_tabs = launch.preload_tabs;
    }

    /// Seed the agents draft from the owning launch config.
    pub fn begin_agents_edit(&mut self, launch: &TerminalLaunchConfig) {
        self.editing_agents = launch.agents.clone();
    }

    /// Seed the theme draft from the applied theme.
    pub fn begin_theme_edit(&mut self, theme: &AppTheme) {
        self.editing_theme = *theme;
    }

    /// Seed the fonts draft from the applied theme fonts.
    pub fn begin_font_edit(&mut self, fonts: &AppFonts) {
        self.editing_fonts = *fonts;
    }

    /// Render all open windows and collect what the user did in them.
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
        self.show_missing_folder_window(ctx);

        actions
    }
}

/// Everything the user did in settings windows during this frame.
#[derive(Default)]
pub struct WindowActions {
    pub rename_group: Option<(u64, String)>,
    pub default_shell_cmd: Option<String>,
    pub agents: Option<[AgentConfig; MAX_AGENTS]>,
    pub run_as_login_shell: Option<bool>,
    pub enable_git_status: Option<bool>,
    pub preload_tabs: Option<bool>,
    pub theme: Option<AppTheme>,
    pub fonts: Option<AppFonts>,
    /// Theme window closed without saving: undo the live preview.
    pub theme_discarded: bool,
    /// Font window closed without saving: undo any applied preview.
    pub fonts_discarded: bool,
    pub should_save_settings: bool,
    pub close_confirmed: bool,
}

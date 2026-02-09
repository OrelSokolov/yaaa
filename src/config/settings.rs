use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::constants::*;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Settings {
    #[serde(default = "default_show_terminal_lines")]
    pub show_terminal_lines: bool,
    #[serde(default = "default_show_fps")]
    pub show_fps: bool,
    #[serde(default = "default_show_sidebar")]
    pub show_sidebar: bool,
    #[serde(default = "default_run_as_login_shell")]
    pub run_as_login_shell: bool,
    #[serde(default = "default_shell_cmd")]
    pub default_shell_cmd: String,
    #[serde(default = "default_agent_cmd")]
    pub default_agent_cmd: String,
}

fn default_show_terminal_lines() -> bool {
    DEFAULT_SHOW_TERMINAL_LINES
}

fn default_show_fps() -> bool {
    DEFAULT_SHOW_FPS
}

fn default_show_sidebar() -> bool {
    DEFAULT_SHOW_SIDEBAR
}

fn default_run_as_login_shell() -> bool {
    DEFAULT_RUN_AS_LOGIN_SHELL
}

fn default_shell_cmd() -> String {
    DEFAULT_SHELL_CMD.to_string()
}

fn default_agent_cmd() -> String {
    DEFAULT_AGENT_CMD.to_string()
}

impl Settings {
    pub fn get_config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("yaaa");
            let _ = std::fs::create_dir_all(&path);
            path
        })
    }

    pub fn load() -> Self {
        if let Some(config_dir) = Self::get_config_dir() {
            let settings_file = config_dir.join(SETTINGS_FILE);
            if settings_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&settings_file) {
                    if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                        return settings;
                    }
                }
            }
        }
        Settings::default()
    }

    pub fn save(&self) {
        if let Some(config_dir) = Self::get_config_dir() {
            let settings_file = config_dir.join(SETTINGS_FILE);
            if let Ok(settings_json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&settings_file, settings_json);
            }
        }
    }
}

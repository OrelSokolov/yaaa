use serde::{Deserialize, Serialize};

use crate::constants::*;
use crate::theme::AppTheme;

pub const MAX_AGENTS: usize = crate::constants::MAX_AGENTS;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AgentConfig {
    #[serde(default = "default_agent_name")]
    pub name: String,
    #[serde(default = "default_agent_cmd_field")]
    pub cmd: String,
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
}

fn default_agent_name() -> String {
    String::new()
}

fn default_agent_cmd_field() -> String {
    String::new()
}

fn default_agent_enabled() -> bool {
    false
}

impl AgentConfig {
    pub fn default_for_index(index: usize) -> Self {
        let name = match index {
            0 => "Agent",
            1 => "Agent 2",
            2 => "Agent 3",
            3 => "Agent 4",
            _ => "Agent",
        }
        .to_string();
        let cmd = if index == 0 {
            DEFAULT_AGENT_CMD.to_string()
        } else {
            String::new()
        };
        Self {
            name,
            cmd,
            enabled: index == 0,
        }
    }
}

fn default_agents() -> [AgentConfig; MAX_AGENTS] {
    [
        AgentConfig::default_for_index(0),
        AgentConfig::default_for_index(1),
        AgentConfig::default_for_index(2),
        AgentConfig::default_for_index(3),
    ]
}

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
    #[serde(default = "default_agents")]
    pub agents: [AgentConfig; MAX_AGENTS],
    /// Legacy field kept only for migrating old settings files that stored a
    /// single default agent command. It is not serialized back.
    #[serde(default, rename = "default_agent_cmd", skip_serializing)]
    pub legacy_default_agent_cmd: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: AppTheme,
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

fn default_theme() -> AppTheme {
    AppTheme::default()
}

impl Settings {
    pub fn load() -> Self {
        let mut settings = if let Some(config_dir) = super::config_dir() {
            let settings_file = config_dir.join(SETTINGS_FILE);
            if settings_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&settings_file) {
                    if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                        settings
                    } else {
                        Settings::default()
                    }
                } else {
                    Settings::default()
                }
            } else {
                Settings::default()
            }
        } else {
            Settings::default()
        };

        // Migrate legacy single-agent command into the first agent slot.
        if let Some(legacy_cmd) = settings.legacy_default_agent_cmd.take() {
            if !legacy_cmd.trim().is_empty() && settings.agents[0].cmd.trim().is_empty() {
                settings.agents[0].cmd = legacy_cmd;
            }
        }

        settings
    }

    pub fn save(&self) {
        if let Some(config_dir) = super::config_dir() {
            let settings_file = config_dir.join(SETTINGS_FILE);
            if let Ok(settings_json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&settings_file, settings_json);
            }
        }
    }
}

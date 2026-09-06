use serde::{Deserialize, Serialize};

use crate::constants::*;
use crate::theme::AppTheme;

pub const MAX_AGENTS: usize = crate::constants::MAX_AGENTS;

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
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

pub fn default_agents() -> [AgentConfig; MAX_AGENTS] {
    [
        AgentConfig::default_for_index(0),
        AgentConfig::default_for_index(1),
        AgentConfig::default_for_index(2),
        AgentConfig::default_for_index(3),
    ]
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Settings {
    #[serde(default = "default_show_terminal_lines")]
    pub show_terminal_lines: bool,
    #[serde(default = "default_show_fps")]
    pub show_fps: bool,
    #[serde(default = "default_show_sidebar")]
    pub show_sidebar: bool,
    #[serde(default = "default_show_system_monitor")]
    pub show_system_monitor: bool,
    #[serde(default = "default_show_tab_memory")]
    pub show_tab_memory: bool,
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
    #[serde(default = "default_enable_git_status")]
    pub enable_git_status: bool,
    #[serde(default = "default_preload_tabs")]
    pub preload_tabs: bool,
    /// Last known terminal content size [width, height] in pixels. Used to seed
    /// new terminals at the correct column/row count on startup so the PTY does
    /// not boot at the 80x50 default and resize on the first frame.
    #[serde(default)]
    pub last_terminal_layout: Option<[f32; 2]>,
    /// Last known terminal font cell metrics [cell_width, cell_height] in
    /// pixels. Seeded into new terminals at startup so they don't boot at the
    /// 80x50 default. Recomputed on the first frame and on font changes.
    #[serde(default)]
    pub last_terminal_cell_metrics: Option<[f32; 2]>,
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

fn default_show_system_monitor() -> bool {
    DEFAULT_SHOW_SYSTEM_MONITOR
}

fn default_show_tab_memory() -> bool {
    DEFAULT_SHOW_TAB_MEMORY
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

fn default_enable_git_status() -> bool {
    true
}

fn default_preload_tabs() -> bool {
    DEFAULT_PRELOAD_TABS
}

/// Manual `Default` that matches the serde field defaults, so that
/// `Settings::default()` (used on first launch when no settings file exists)
/// is equivalent to deserializing an empty settings object. The derived
/// `Default` used to zero every bool and disable every agent instead.
impl Default for Settings {
    fn default() -> Self {
        Self {
            show_terminal_lines: DEFAULT_SHOW_TERMINAL_LINES,
            show_fps: DEFAULT_SHOW_FPS,
            show_sidebar: DEFAULT_SHOW_SIDEBAR,
            show_system_monitor: DEFAULT_SHOW_SYSTEM_MONITOR,
            show_tab_memory: DEFAULT_SHOW_TAB_MEMORY,
            run_as_login_shell: DEFAULT_RUN_AS_LOGIN_SHELL,
            default_shell_cmd: DEFAULT_SHELL_CMD.to_string(),
            agents: default_agents(),
            legacy_default_agent_cmd: None,
            theme: AppTheme::default(),
            enable_git_status: default_enable_git_status(),
            preload_tabs: DEFAULT_PRELOAD_TABS,
            last_terminal_layout: None,
            last_terminal_cell_metrics: None,
        }
    }
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

        settings.migrate_legacy_agent();
        settings
    }

    /// Migrate the legacy single-agent command into the first agent slot.
    fn migrate_legacy_agent(&mut self) {
        if let Some(legacy_cmd) = self.legacy_default_agent_cmd.take() {
            if !legacy_cmd.trim().is_empty() && self.agents[0].cmd.trim().is_empty() {
                self.agents[0].cmd = legacy_cmd;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_constants() {
        let s = Settings::default();
        assert!(s.show_terminal_lines);
        assert!(s.show_fps);
        assert!(s.show_sidebar);
        assert!(s.show_system_monitor);
        assert!(s.show_tab_memory);
        assert!(!s.run_as_login_shell);
        assert_eq!(s.default_shell_cmd, "");
        assert_eq!(s.theme, AppTheme::default());
        assert!(s.enable_git_status);
        assert!(s.preload_tabs);
        assert_eq!(s.last_terminal_layout, None);
        assert_eq!(s.last_terminal_cell_metrics, None);
    }

    #[test]
    fn default_agents_first_enabled_rest_empty() {
        let agents = default_agents();
        assert!(agents[0].enabled);
        assert_eq!(agents[0].cmd, DEFAULT_AGENT_CMD);
        assert_eq!(agents[0].name, "Agent");
        for (i, agent) in agents.iter().enumerate().skip(1) {
            assert!(!agent.enabled, "agent {i} should be disabled");
            assert_eq!(agent.cmd, "");
        }
    }

    #[test]
    fn serde_round_trip() {
        let mut s = Settings::default();
        s.default_shell_cmd = "/bin/zsh".to_string();
        s.agents[1].enabled = true;
        s.agents[1].cmd = "claude".to_string();
        s.agents[1].name = "Claude".to_string();
        s.show_sidebar = false;
        s.last_terminal_layout = Some([640.0, 480.0]);

        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // A settings file written by an old version that only knew about a few
        // fields must still deserialize into full defaults.
        let json = r#"{"show_sidebar": false}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(!s.show_sidebar);
        assert!(s.show_fps);
        assert_eq!(s.agents[0].cmd, DEFAULT_AGENT_CMD);
    }

    #[test]
    fn legacy_agent_cmd_migrates_into_first_agent() {
        // Note: the migration only fires when agents[0].cmd is empty in the
        // file. A file without an `agents` field deserializes agent 0 with the
        // default "opencode" command, so the legacy value is skipped there.
        let json = r#"{"default_agent_cmd": "aider", "agents": [
            {"name": "", "cmd": "", "enabled": false},
            {"name": "", "cmd": "", "enabled": false},
            {"name": "", "cmd": "", "enabled": false},
            {"name": "", "cmd": "", "enabled": false}
        ]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy_agent();
        assert_eq!(s.agents[0].cmd, "aider");
        assert_eq!(s.legacy_default_agent_cmd, None);
    }

    #[test]
    fn legacy_agent_cmd_skipped_when_agents_field_absent() {
        // Characterizes current behavior: the serde default for agents[0].cmd
        // is "opencode" (non-empty), so the legacy command does not migrate.
        let json = r#"{"default_agent_cmd": "aider"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy_agent();
        assert_eq!(s.agents[0].cmd, DEFAULT_AGENT_CMD);
    }

    #[test]
    fn legacy_agent_cmd_does_not_override_explicit_agent() {
        let json = r#"{"default_agent_cmd": "aider", "agents": [
            {"name": "A", "cmd": "claude", "enabled": true},
            {"name": "", "cmd": "", "enabled": false},
            {"name": "", "cmd": "", "enabled": false},
            {"name": "", "cmd": "", "enabled": false}
        ]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy_agent();
        assert_eq!(s.agents[0].cmd, "claude");
    }

    #[test]
    fn blank_legacy_agent_cmd_is_ignored() {
        let json = r#"{"default_agent_cmd": "   "}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy_agent();
        assert_eq!(s.agents[0].cmd, DEFAULT_AGENT_CMD);
    }

    #[test]
    fn legacy_field_is_not_serialized_back() {
        let json = r#"{"default_agent_cmd": "aider"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy_agent();
        let out = serde_json::to_string(&s).unwrap();
        assert!(!out.contains("default_agent_cmd"));
    }
}

use serde::{Deserialize, Serialize};

use super::settings::{AgentConfig, Settings, MAX_AGENTS};
use crate::constants::{DEFAULT_PRELOAD_TABS, DEFAULT_RUN_AS_LOGIN_SHELL, DEFAULT_SHELL_CMD};
/// Everything that controls how new terminal tabs are spawned. A single
/// source of truth owned by `App`: `Settings` is its serialized form,
/// `TabManager` and the settings dialogs work on copies of it.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct TerminalLaunchConfig {
    /// Command used for plain terminal tabs (empty = system default shell).
    #[serde(default = "default_shell_cmd")]
    pub default_shell_cmd: String,
    /// Up to `MAX_AGENTS` agent presets offered next to terminals.
    #[serde(default = "default_agents")]
    pub agents: [AgentConfig; MAX_AGENTS],
    /// Whether plain terminal tabs run the shell as a login shell. Agents
    /// have their own per-agent `wrap_login_shell` flag instead.
    #[serde(default = "default_run_as_login_shell")]
    pub run_as_login_shell: bool,
    /// Whether spare terminals/agents are pre-spawned per group for instant open.
    #[serde(default = "default_preload_tabs")]
    pub preload_tabs: bool,
}

fn default_shell_cmd() -> String {
    DEFAULT_SHELL_CMD.to_string()
}

fn default_run_as_login_shell() -> bool {
    DEFAULT_RUN_AS_LOGIN_SHELL
}

fn default_preload_tabs() -> bool {
    DEFAULT_PRELOAD_TABS
}

fn default_agents() -> [AgentConfig; MAX_AGENTS] {
    super::settings::default_agents()
}

/// Manual `Default` matching the serde field defaults (same rationale as
/// `Settings`: the derived `Default` would zero every bool and disable every
/// agent).
impl Default for TerminalLaunchConfig {
    fn default() -> Self {
        Self {
            default_shell_cmd: default_shell_cmd(),
            agents: default_agents(),
            run_as_login_shell: default_run_as_login_shell(),
            preload_tabs: default_preload_tabs(),
        }
    }
}

impl TerminalLaunchConfig {
    /// Extract the launch config from persisted settings.
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            default_shell_cmd: settings.default_shell_cmd.clone(),
            agents: settings.agents.clone(),
            run_as_login_shell: settings.run_as_login_shell,
            preload_tabs: settings.preload_tabs,
        }
    }

    /// Write the launch config back into settings (for persistence).
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        settings.default_shell_cmd = self.default_shell_cmd.clone();
        settings.agents = self.agents.clone();
        settings.run_as_login_shell = self.run_as_login_shell;
        settings.preload_tabs = self.preload_tabs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEFAULT_AGENT_CMD;

    #[test]
    fn defaults_match_constants() {
        let config = TerminalLaunchConfig::default();
        assert_eq!(config.default_shell_cmd, DEFAULT_SHELL_CMD);
        assert!(!config.run_as_login_shell);
        assert!(config.preload_tabs);
        assert_eq!(config.agents[0].cmd, DEFAULT_AGENT_CMD);
        assert!(config.agents[0].enabled);
    }

    #[test]
    fn settings_round_trip() {
        let mut config = TerminalLaunchConfig {
            default_shell_cmd: "/bin/zsh".into(),
            run_as_login_shell: true,
            ..Default::default()
        };
        config.agents[1] = AgentConfig {
            name: "Claude".into(),
            cmd: "claude".into(),
            enabled: true,
            wrap_login_shell: true,
        };

        let mut settings = Settings::default();
        config.apply_to_settings(&mut settings);
        let back = TerminalLaunchConfig::from_settings(&settings);
        assert_eq!(back, config);
    }

    #[test]
    fn apply_to_settings_leaves_other_fields_alone() {
        let mut settings = Settings {
            show_sidebar: false,
            ..Default::default()
        };
        let config = TerminalLaunchConfig::default();
        config.apply_to_settings(&mut settings);
        assert!(!settings.show_sidebar);
    }
}

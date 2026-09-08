use alacritty_terminal::grid::Dimensions;
use egui_term::{PtyEvent, TerminalBackend, TerminalMode};
use std::{path::PathBuf, sync::mpsc::Sender};

use crate::terminal::shell_env;

pub trait TerminalBackendExt {
    fn total_lines(&self) -> usize;
    fn screen_lines(&self) -> usize;
}

impl TerminalBackendExt for TerminalBackend {
    fn total_lines(&self) -> usize {
        self.last_content().total_lines
    }

    fn screen_lines(&self) -> usize {
        self.last_content().terminal_size.screen_lines()
    }
}

#[derive(Default)]
pub struct ScrollState {
    pub last_line_count: usize,
    pub user_scrolled_up: bool,
}

impl ScrollState {
    pub fn detect_clear(&self, current_lines: usize) -> bool {
        self.last_line_count > 0 && (current_lines as f64) < (self.last_line_count as f64) * 0.1
    }
}

#[derive(Default)]
pub struct TabScrollState {
    pub normal: ScrollState,
    pub alternate: ScrollState,
}

impl TabScrollState {
    pub fn current(&mut self, is_alternate: bool) -> &mut ScrollState {
        if is_alternate {
            &mut self.alternate
        } else {
            &mut self.normal
        }
    }
}

pub struct Tab {
    pub backend: TerminalBackend,
    pub title: String,
    pub scroll_state: TabScrollState,
    pub was_alternate_last_frame: bool,
    pub just_created: bool,
    pub search_active: bool,
    pub search_query: String,
    pub search_just_opened: bool,
}

impl Tab {
    pub fn is_alternate_screen(&self) -> bool {
        self.backend
            .last_content()
            .terminal_mode
            .contains(TerminalMode::ALT_SCREEN)
    }

    fn shell_candidates(shell_cmd: &str, is_agent: bool) -> Vec<String> {
        let mut candidates: Vec<String> = Vec::new();

        if !shell_cmd.is_empty() {
            candidates.push(shell_cmd.to_string());
        }

        if is_agent {
            candidates.push("/usr/bin/bash".to_string());
            candidates.push("/bin/bash".to_string());
            candidates.push("bash".to_string());
        }

        #[cfg(unix)]
        {
            if let Ok(shell_env) = std::env::var("SHELL") {
                if !shell_env.is_empty() {
                    candidates.push(shell_env);
                }
            }
            candidates.extend(
                ["/bin/zsh", "/bin/bash", "/usr/bin/zsh", "/usr/bin/bash"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        #[cfg(windows)]
        {
            candidates.extend(["cmd.exe", "powershell.exe"].iter().map(|s| s.to_string()));
        }

        // Deduplicate while preserving order.
        let mut seen = std::collections::HashSet::new();
        candidates
            .into_iter()
            .filter(|c| seen.insert(c.clone()))
            .collect()
    }

    /// Login-shell flag for a shell program, or `None` for shells that have
    /// no login mode (the tab then starts as a plain interactive shell).
    fn login_flag(program: &str) -> Option<&'static str> {
        let name = std::path::Path::new(program)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(program);
        match name {
            "bash" | "zsh" | "sh" | "dash" | "ksh" => Some("--login"),
            "fish" => Some("-l"),
            _ => None,
        }
    }

    /// Wrap an agent command so it runs inside the user's login shell.
    /// A directly exec'd agent never goes through a shell, so user rc files
    /// (.bashrc, .profile — PATH setup from nvm and the like) never run.
    /// Returns `None` when the setting is off, or on Windows where no login
    /// shell exists and agent `.cmd`/`.exe` commands must spawn directly.
    fn login_shell_wrapper(
        enabled: bool,
        agent_cmd: &str,
    ) -> Option<(String, Vec<String>)> {
        if !enabled || !cfg!(unix) {
            return None;
        }
        Self::wrapper_shell().map(|shell| {
            (
                shell.clone(),
                Self::login_shell_wrapper_args(&shell, agent_cmd),
            )
        })
    }

    /// The shell agents are wrapped in: the user's login shell, falling back
    /// to the usual suspects. Always `None` off Unix.
    fn wrapper_shell() -> Option<String> {
        if !cfg!(unix) {
            return None;
        }
        if let Ok(shell) = std::env::var("SHELL") {
            if !shell.is_empty() {
                return Some(shell);
            }
        }
        ["/bin/bash", "/usr/bin/bash", "/bin/zsh"]
            .into_iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(str::to_string)
    }

    /// `<shell> <login flag> -i -c <agent_cmd>`: interactive so bash reads
    /// .bashrc as well as .bash_profile, login so the profile files load too.
    fn login_shell_wrapper_args(shell: &str, agent_cmd: &str) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(flag) = Self::login_flag(shell) {
            args.push(flag.to_string());
        }
        args.push("-i".to_string());
        args.push("-c".to_string());
        args.push(agent_cmd.to_string());
        args
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ctx: egui::Context,
        command_sender: Sender<(u64, PtyEvent)>,
        id: u64,
        working_dir: Option<PathBuf>,
        shell_cmd: &str,
        is_agent: bool,
        run_as_login_shell: bool,
        layout_hint: Option<egui_term::Size>,
        cell_hint: Option<egui_term::Size>,
    ) -> Result<Self, String> {
        let mut candidates = Self::shell_candidates(shell_cmd, is_agent).into_iter();

        // For agents the first candidate is the configured agent command and may
        // include arguments. For regular shells the candidate is just the shell path.
        // The candidate list is never empty: the platform fallbacks below always
        // add at least one shell.
        let Some(first) = candidates.next() else {
            return Err(format!("no shell candidates for tab {}", id));
        };
        let mut shell = first.clone();
        // For agents the first candidate is the configured agent command and
        // may include arguments; fallbacks are bare shell paths. With
        // login-shell mode on, the whole command is instead wrapped in the
        // user's login shell so rc files (.bashrc et al.) are sourced.
        let mut agent_args: Vec<String> = Vec::new();
        if is_agent {
            if let Some((wrapper, args)) = Self::login_shell_wrapper(run_as_login_shell, &first) {
                shell = wrapper;
                agent_args = args;
            } else {
                let parts: Vec<&str> = first.split_whitespace().collect();
                if parts.len() > 1 {
                    shell = parts[0].to_string();
                    agent_args = parts[1..].iter().map(|s| s.to_string()).collect();
                }
            }
        }

        let mut using_configured_command = true;
        let backend = loop {
            // Fallback shells run bare; the login flag depends on the shell
            // actually being started (fish spells it `-l`, nu has none).
            let mut args: Vec<String> = if using_configured_command {
                agent_args.clone()
            } else {
                Vec::new()
            };
            // A wrapped agent command already carries its login flag in
            // `agent_args`; everything else (plain terminals, agent fallback
            // shells) appends it here.
            if run_as_login_shell && !(is_agent && using_configured_command) {
                if let Some(flag) = Self::login_flag(&shell) {
                    args.push(flag.to_string());
                }
            }

            let result = TerminalBackend::new(
                id,
                ctx.clone(),
                command_sender.clone(),
                egui_term::BackendSettings {
                    shell: shell.clone(),
                    args: args.clone(),
                    working_directory: working_dir.clone(),
                    env: shell_env::build(),
                    initial_layout_size: layout_hint,
                    initial_cell_metrics: cell_hint,
                },
            );

            match result {
                Ok(backend) => break backend,
                Err(e) => {
                    log::warn!(
                        "Failed to create terminal backend with shell '{}': {}",
                        shell,
                        e
                    );

                    let Some(next) = candidates.next() else {
                        return Err(format!(
                            "all fallback shells failed for tab {}: {}",
                            id, e
                        ));
                    };

                    shell = next;
                    using_configured_command = false;
                    log::warn!("Retrying with fallback shell: {}", shell);
                }
            }
        };

        Ok(Self {
            backend,
            title: format!("tab: {}", id),
            scroll_state: TabScrollState::default(),
            was_alternate_last_frame: false,
            just_created: true,
            search_active: false,
            search_query: String::new(),
            search_just_opened: false,
        })
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_clear_triggers_on_line_drop() {
        let state = ScrollState {
            last_line_count: 1000,
            user_scrolled_up: false,
        };
        // Fewer than 10% of the previous lines remain.
        assert!(state.detect_clear(50));
        // 20% remain — not a clear.
        assert!(!state.detect_clear(200));
    }

    #[test]
    fn detect_clear_ignores_first_frame() {
        let state = ScrollState::default();
        assert!(!state.detect_clear(0));
    }

    #[test]
    fn login_flag_depends_on_shell() {
        assert_eq!(Tab::login_flag("/bin/zsh"), Some("--login"));
        assert_eq!(Tab::login_flag("/usr/bin/bash"), Some("--login"));
        assert_eq!(Tab::login_flag("sh"), Some("--login"));
        assert_eq!(Tab::login_flag("fish"), Some("-l"));
        assert_eq!(Tab::login_flag("nu"), None);
        assert_eq!(Tab::login_flag("/usr/local/bin/nu"), None);
    }

    #[test]
    fn login_shell_wrapper_args_bash() {
        let args = Tab::login_shell_wrapper_args("/bin/bash", "claude --model x");
        assert_eq!(args, ["--login", "-i", "-c", "claude --model x"]);
    }

    #[test]
    fn login_shell_wrapper_args_fish() {
        let args = Tab::login_shell_wrapper_args("fish", "opencode");
        assert_eq!(args, ["-l", "-i", "-c", "opencode"]);
    }

    #[test]
    fn login_shell_wrapper_args_shell_without_login_mode() {
        let args = Tab::login_shell_wrapper_args("/usr/local/bin/nu", "opencode");
        assert_eq!(args, ["-i", "-c", "opencode"]);
    }

    #[test]
    fn login_shell_wrapper_disabled_returns_none() {
        assert!(Tab::login_shell_wrapper(false, "claude").is_none());
    }

    #[test]
    fn login_shell_wrapper_enabled_finds_shell_on_unix() {
        if cfg!(unix) {
            let (shell, args) = Tab::login_shell_wrapper(true, "claude").unwrap();
            assert!(!shell.is_empty());
            assert!(args.contains(&"-i".to_string()));
            assert_eq!(args.last().map(String::as_str), Some("claude"));
        }
    }

    #[test]
    fn shell_candidates_start_with_configured_command() {
        let candidates = Tab::shell_candidates("/opt/mysh -x", false);
        assert_eq!(candidates.first().map(String::as_str), Some("/opt/mysh -x"));
        // Platform fallbacks are always present.
        assert!(candidates.len() > 1);
    }
}

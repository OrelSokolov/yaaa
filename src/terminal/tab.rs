use alacritty_terminal::grid::Dimensions;
use egui_term::{PtyEvent, TerminalBackend, TerminalMode};
use std::{path::PathBuf, sync::mpsc::Sender};

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

    pub fn command_exists(cmd: &str) -> bool {
        // Extract just the program name (first word) from the command
        let program = cmd.split_whitespace().next().unwrap_or(cmd);

        #[cfg(unix)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("which").arg(program).output() {
                output.status.success()
            } else {
                false
            }
        }
        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("where")
                .arg(program)
                .output()
                .map_or(false, |output| output.status.success())
        }
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

    pub fn resolve_shell(shell_cmd: &str, is_agent: bool) -> String {
        for candidate in Self::shell_candidates(shell_cmd, is_agent) {
            if Self::command_exists(&candidate) {
                return candidate;
            }
        }

        if !shell_cmd.is_empty() {
            return shell_cmd.to_string();
        }

        #[cfg(unix)]
        return std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        #[cfg(windows)]
        return "cmd.exe".to_string();
    }

    pub fn new(
        ctx: egui::Context,
        command_sender: Sender<(u64, PtyEvent)>,
        id: u64,
        working_dir: Option<PathBuf>,
        shell_cmd: &str,
        is_agent: bool,
        run_as_login_shell: bool,
    ) -> Self {
        let mut candidates = Self::shell_candidates(shell_cmd, is_agent).into_iter();

        // For agents the first candidate is the configured agent command and may
        // include arguments. For regular shells the candidate is just the shell path.
        let first = candidates
            .next()
            .unwrap_or_else(|| Self::resolve_shell("", false));
        let mut shell = first.clone();
        let mut args: Vec<String> = Vec::new();
        if is_agent {
            let parts: Vec<&str> = first.split_whitespace().collect();
            if parts.len() > 1 {
                shell = parts[0].to_string();
                args = parts[1..].iter().map(|s| s.to_string()).collect();
            }
        }

        // Add login shell flag if needed (only for non-agent shells)
        if run_as_login_shell && !is_agent {
            args.push("--login".to_string());
        }

        let backend = loop {
            let result = TerminalBackend::new(
                id,
                ctx.clone(),
                command_sender.clone(),
                egui_term::BackendSettings {
                    shell: shell.clone(),
                    args: args.clone(),
                    working_directory: working_dir.clone(),
                    ..Default::default()
                },
            );

            match result {
                Ok(backend) => break backend,
                Err(e) => {
                    eprintln!(
                        "Failed to create terminal backend with shell '{}': {}",
                        shell, e
                    );

                    let Some(next) = candidates.next() else {
                        panic!("All fallback shells failed. Last error: {}", e);
                    };

                    // Subsequent fallbacks are bare shell paths; clear agent args.
                    shell = next;
                    if is_agent {
                        args.clear();
                    }
                    eprintln!("Retrying with fallback shell: {}", shell);
                }
            }
        };

        Self {
            backend,
            title: format!("tab: {}", id),
            scroll_state: TabScrollState::default(),
            was_alternate_last_frame: false,
            just_created: true,
            search_active: false,
            search_query: String::new(),
            search_just_opened: false,
        }
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

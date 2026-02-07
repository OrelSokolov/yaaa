use alacritty_terminal::grid::Dimensions;
use egui_term::{PtyEvent, TerminalBackend, TerminalMode};
use std::{path::PathBuf, sync::mpsc::Sender};

pub trait TerminalBackendExt {
    fn total_lines(&self) -> usize;
    fn screen_lines(&self) -> usize;
}

impl TerminalBackendExt for TerminalBackend {
    fn total_lines(&self) -> usize {
        self.last_content().grid.total_lines()
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
    pub is_agent: bool,
}

impl Tab {
    pub fn is_alternate_screen(&self) -> bool {
        self.backend
            .last_content()
            .terminal_mode
            .contains(TerminalMode::ALT_SCREEN)
    }

    pub fn command_exists(cmd: &str) -> bool {
        #[cfg(unix)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("which").arg(cmd).output() {
                output.status.success()
            } else {
                false
            }
        }
        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("where")
                .arg(cmd)
                .output()
                .map_or(false, |output| output.status.success())
        }
    }

    pub fn resolve_shell(shell_cmd: &str, is_agent: bool) -> String {
        if shell_cmd.is_empty() {
            #[cfg(unix)]
            return std::env::var("SHELL").unwrap_or_else(|_| "/usr/bin/bash".to_string());
            #[cfg(windows)]
            return "cmd.exe".to_string();
        }

        let mut candidates = vec![shell_cmd.to_string()];

        if is_agent {
            candidates.push("/usr/bin/bash".to_string());
            candidates.push("bash".to_string());
        }

        #[cfg(unix)]
        candidates.push("/usr/bin/bash".to_string());
        #[cfg(windows)]
        candidates.push("cmd.exe".to_string());

        for candidate in candidates {
            if Self::command_exists(&candidate) {
                return candidate;
            }
        }

        #[cfg(unix)]
        return "/usr/bin/bash".to_string();
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
        let mut shell = Self::resolve_shell(shell_cmd, is_agent);

        let args = if run_as_login_shell {
            vec!["--login".to_string()]
        } else {
            vec![]
        };

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

                    let fallback = "/usr/bin/bash";
                    if shell == fallback {
                        panic!("Fallback shell '{}' also failed: {}", fallback, e);
                    }

                    shell = fallback.to_string();
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
            is_agent,
        }
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

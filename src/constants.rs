pub const GROUPS_FILE: &str = "groups.json";
pub const SETTINGS_FILE: &str = "settings.json";
pub const RECENT_PROJECTS_FILE: &str = "recent_projects.json";

pub const DEFAULT_SHOW_TERMINAL_LINES: bool = true;
pub const DEFAULT_SHOW_FPS: bool = true;
pub const DEFAULT_SHOW_SIDEBAR: bool = true;
pub const DEFAULT_SHOW_SYSTEM_MONITOR: bool = true;
pub const DEFAULT_SHOW_TAB_MEMORY: bool = true;
pub const DEFAULT_RUN_AS_LOGIN_SHELL: bool = false;
pub const DEFAULT_SHELL_CMD: &str = "";
pub const DEFAULT_AGENT_CMD: &str = "opencode";
pub const MAX_AGENTS: usize = 4;
pub const DEFAULT_PRELOAD_TABS: bool = true;
pub const DEFAULT_SHOW_WELCOME: bool = true;

/// How many recent projects to keep in the Projects menu.
pub const RECENT_PROJECTS_LIMIT: usize = 20;

/// Repaint cadence while a terminal tab is active.
pub const REPAINT_DELAY_FOCUSED_MS: u64 = 500;
pub const REPAINT_DELAY_UNFOCUSED_MS: u64 = 1000;

/// Per-tab memory severity thresholds for the sidebar: yellow above WARN,
/// red above HIGH.
pub const TAB_MEM_WARN_KB: u64 = 200 * 1024;
pub const TAB_MEM_HIGH_KB: u64 = 500 * 1024;

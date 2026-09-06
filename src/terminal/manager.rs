use crate::config::settings::{AgentConfig, MAX_AGENTS};
use crate::constants::GROUPS_FILE;
use crate::terminal::tab::Tab;
use egui_term::PtyEvent;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::mpsc::Sender,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct TabInfo {
    pub id: u64,
    pub is_agent: bool,
    #[serde(default)]
    pub agent_index: Option<usize>,
    #[serde(default)]
    pub is_important: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TabGroup {
    pub id: u64,
    pub name: String,
    pub path: PathBuf,
    pub tabs: Vec<TabInfo>,
}

impl TabGroup {
    pub fn new(id: u64, name: String, path: PathBuf) -> Self {
        Self {
            id,
            name,
            path,
            tabs: Vec::new(),
        }
    }

    pub fn name_from_path(path: &std::path::Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }
}

/// Read persisted groups from `path`. Returns `None` when the file is missing
/// or its content cannot be parsed; a corrupt file is preserved as `.bak`.
pub(crate) fn read_groups_file(path: &std::path::Path) -> Option<Vec<TabGroup>> {
    if !path.exists() {
        return None;
    }
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            log::warn!("Could not read session file {}: {}", path.display(), e);
            return None;
        }
    };
    match serde_json::from_str(&content) {
        Ok(groups) => Some(groups),
        Err(e) => {
            log::warn!(
                "Corrupt session file {}: {} — backing it up, starting fresh",
                path.display(),
                e
            );
            crate::config::backup_corrupt(path);
            None
        }
    }
}

/// Persist `groups` to `path` as pretty JSON, ordered by group id.
pub(crate) fn write_groups_file(path: &std::path::Path, groups: &BTreeMap<u64, TabGroup>) {
    if let Ok(json) = serde_json::to_string_pretty(&groups.values().collect::<Vec<_>>()) {
        if let Err(e) = crate::config::write_atomic(path, &json) {
            log::warn!("Could not save session: {}", e);
        }
    }
}

pub struct TabManager {
    command_sender: Sender<(u64, PtyEvent)>,
    groups: BTreeMap<u64, TabGroup>,
    tabs: BTreeMap<u64, Tab>,
    /// Key: (group_id, agent_index) where agent_index is None for terminal.
    preload_pool: HashMap<(u64, Option<usize>), (u64, Tab)>,
    pub active_group_id: Option<u64>,
    pub active_tab_id: Option<u64>,
    next_group_id: u64,
    next_tab_id: u64,
    /// Copy of the launch config used to spawn new tabs; kept in sync with
    /// the owner (`App::launch_config`) via `update_launch_config`.
    default_shell_cmd: String,
    agents: [AgentConfig; MAX_AGENTS],
    run_as_login_shell: bool,
    preload_enabled: bool,
    /// Set by every mutating method; flushed once per frame by
    /// `save_groups_if_dirty` so callers never need to remember to save.
    groups_dirty: bool,
    /// Last known terminal content size, used to seed new terminals at the
    /// correct column/row count so the PTY does not boot at the 80x50 default.
    terminal_layout_hint: Option<egui_term::Size>,
    /// Current font cell metrics (cell width/height) matching the hint above.
    cell_metrics_hint: Option<egui_term::Size>,
}

impl TabManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_sender: Sender<(u64, PtyEvent)>,
        cc: &eframe::CreationContext<'_>,
        launch: &crate::config::TerminalLaunchConfig,
        terminal_layout_hint: Option<egui_term::Size>,
        cell_metrics_hint: Option<egui_term::Size>,
    ) -> Self {
        let mut manager = Self {
            command_sender,
            groups: BTreeMap::new(),
            tabs: BTreeMap::new(),
            preload_pool: HashMap::new(),
            active_group_id: None,
            active_tab_id: None,
            next_group_id: 0,
            next_tab_id: 0,
            default_shell_cmd: launch.default_shell_cmd.clone(),
            agents: launch.agents.clone(),
            run_as_login_shell: launch.run_as_login_shell,
            preload_enabled: launch.preload_tabs,
            groups_dirty: false,
            terminal_layout_hint,
            cell_metrics_hint,
        };

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        if let Some(groups_data) = manager.load_groups() {
            for mut group in groups_data {
                manager.next_group_id = manager.next_group_id.max(group.id + 1);
                let mut failed_tab_ids: Vec<u64> = Vec::new();
                for tab_info in &mut group.tabs {
                    manager.next_tab_id = manager.next_tab_id.max(tab_info.id + 1);

                    let (use_agent, agent_index) = if let Some(idx) = tab_info.agent_index {
                        manager
                            .agents
                            .get(idx)
                            .map(|a| (a.enabled && !a.cmd.trim().is_empty(), Some(idx)))
                            .unwrap_or((false, None))
                    } else if tab_info.is_agent {
                        // Legacy session: agent tab with no index defaults to agent 0.
                        let idx = 0usize;
                        let enabled = manager
                            .agents
                            .get(idx)
                            .map(|a| a.enabled && !a.cmd.trim().is_empty())
                            .unwrap_or(false);
                        (enabled, Some(idx))
                    } else {
                        (false, None)
                    };

                    tab_info.is_agent = use_agent;
                    tab_info.agent_index = agent_index;

                    let shell_cmd = if use_agent {
                        agent_index
                            .and_then(|idx| manager.agents.get(idx))
                            .map(|a| a.cmd.clone())
                            .unwrap_or_default()
                    } else {
                        manager.default_shell_cmd.clone()
                    };

                    match Tab::new(
                        cc.egui_ctx.clone(),
                        manager.command_sender.clone(),
                        tab_info.id,
                        Some(group.path.clone()),
                        &shell_cmd,
                        use_agent,
                        !use_agent && manager.run_as_login_shell,
                        manager.terminal_layout_hint,
                        manager.cell_metrics_hint,
                    ) {
                        Ok(tab) => {
                            manager.tabs.insert(tab_info.id, tab);
                        }
                        Err(e) => {
                            log::warn!("Skipping tab {}: {}", tab_info.id, e);
                            failed_tab_ids.push(tab_info.id);
                        }
                    }
                }
                if !failed_tab_ids.is_empty() {
                    group.tabs.retain(|t| !failed_tab_ids.contains(&t.id));
                }
                manager.groups.insert(group.id, group);
            }
            if let Some(first_group) = manager.groups.first_key_value() {
                manager.active_group_id = Some(*first_group.0);
                manager.active_tab_id = first_group.1.tabs.first().map(|t| t.id);
            }
        }

        let current_dir_exists_in_groups = manager.groups.values().any(|g| g.path == current_dir);

        if !current_dir_exists_in_groups {
            let group_id = manager.next_group_id;
            manager.next_group_id += 1;
            let name = TabGroup::name_from_path(&current_dir);
            let group = TabGroup::new(group_id, name, current_dir);
            manager.groups.insert(group_id, group);
            manager.active_group_id = Some(group_id);

            manager.add_tab_to_group(group_id, cc.egui_ctx.clone(), None);
        }

        manager.populate_preload_pool(cc.egui_ctx.clone());

        manager
    }

    fn load_groups(&mut self) -> Option<Vec<TabGroup>> {
        let path = crate::config::config_dir().map(|d| d.join(GROUPS_FILE))?;
        read_groups_file(&path)
    }

    /// All project groups, ordered by group id.
    pub fn iter_groups(&self) -> impl Iterator<Item = &TabGroup> {
        self.groups.values()
    }

    /// Whether any project group exists.
    pub fn has_groups(&self) -> bool {
        !self.groups.is_empty()
    }

    /// Look up one project group by id.
    pub fn group(&self, id: u64) -> Option<&TabGroup> {
        self.groups.get(&id)
    }

    /// Persist the session if any group changed since the last flush.
    /// Called once per frame by `App`; mutating methods mark the session
    /// dirty automatically, so callers never need to remember to save.
    pub fn save_groups_if_dirty(&mut self) {
        if self.groups_dirty {
            self.save_groups();
            self.groups_dirty = false;
        }
    }

    fn save_groups(&self) {
        if let Some(config_dir) = crate::config::config_dir() {
            write_groups_file(&config_dir.join(GROUPS_FILE), &self.groups);
        }
    }

    fn mark_dirty(&mut self) {
        self.groups_dirty = true;
    }

    pub fn add_group_with_path(&mut self, ctx: egui::Context, path: Option<PathBuf>) {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let path =
            path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let name = TabGroup::name_from_path(&path);

        let group = TabGroup::new(group_id, name, path);
        self.groups.insert(group_id, group);
        self.active_group_id = Some(group_id);
        self.mark_dirty();

        self.add_tab_to_group(group_id, ctx.clone(), None);
        self.populate_preload_for_group(group_id, ctx);
    }

    pub fn rename_group(&mut self, group_id: u64, new_name: String) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.name = new_name;
            self.mark_dirty();
        }
    }

    /// Add a tab to a group.
    /// `agent_index` is `None` for a terminal tab, or `Some(i)` to open agent `i`.
    pub fn add_tab_to_group(
        &mut self,
        group_id: u64,
        ctx: egui::Context,
        agent_index: Option<usize>,
    ) {
        let preload_key = (group_id, agent_index);

        if self.preload_enabled {
            if let Some((tab_id, tab)) = self.preload_pool.remove(&preload_key) {
                let use_agent = agent_index
                    .and_then(|idx| self.agents.get(idx))
                    .map(|a| a.enabled && !a.cmd.trim().is_empty())
                    .unwrap_or(false);

                self.tabs.insert(tab_id, tab);

                if let Some(group) = self.groups.get_mut(&group_id) {
                    group.tabs.push(TabInfo {
                        id: tab_id,
                        is_agent: use_agent,
                        agent_index: if use_agent { agent_index } else { None },
                        is_important: false,
                    });
                }

                self.active_group_id = Some(group_id);
                self.active_tab_id = Some(tab_id);

                self.spawn_preload_tab(group_id, agent_index, ctx);
                self.mark_dirty();
                return;
            }
        }

        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let group_path = self.groups.get(&group_id).map(|g| g.path.clone());

        let (use_agent, shell_cmd) = if let Some(idx) = agent_index {
            self.agents
                .get(idx)
                .filter(|a| a.enabled && !a.cmd.trim().is_empty())
                .map(|a| (true, a.cmd.clone()))
                .unwrap_or((false, self.default_shell_cmd.clone()))
        } else {
            (false, self.default_shell_cmd.clone())
        };

        let tab = match Tab::new(
            ctx,
            self.command_sender.clone(),
            tab_id,
            group_path,
            &shell_cmd,
            use_agent,
            !use_agent && self.run_as_login_shell,
            self.terminal_layout_hint,
            self.cell_metrics_hint,
        ) {
            Ok(tab) => tab,
            Err(e) => {
                log::warn!("Failed to add tab to group {}: {}", group_id, e);
                return;
            }
        };
        self.tabs.insert(tab_id, tab);

        if let Some(group) = self.groups.get_mut(&group_id) {
            group.tabs.push(TabInfo {
                id: tab_id,
                is_agent: use_agent,
                agent_index: if use_agent { agent_index } else { None },
                is_important: false,
            });
        }

        self.active_group_id = Some(group_id);
        self.active_tab_id = Some(tab_id);
        self.mark_dirty();
    }

    pub fn remove_group(&mut self, group_id: u64) {
        if let Some(group) = self.groups.get(&group_id) {
            for tab_info in &group.tabs {
                self.tabs.remove(&tab_info.id);
            }
        }
        self.clear_preload_for_group(group_id);
        self.groups.remove(&group_id);
        self.mark_dirty();

        if self.active_group_id == Some(group_id) {
            if let Some(first_group) = self.groups.first_key_value() {
                self.active_group_id = Some(*first_group.0);
                self.active_tab_id = first_group.1.tabs.first().map(|t| t.id);
            } else {
                self.active_group_id = None;
                self.active_tab_id = None;
            }
        }
    }

    pub fn remove(&mut self, id: u64) {
        let mut group_id_to_remove = None;
        let mut affected_group_id = None;
        let mut group_tabs = None;

        for (group_id, group) in &mut self.groups {
            if group.tabs.iter().any(|t| t.id == id) {
                group.tabs.retain(|t| t.id != id);
                self.tabs.remove(&id);
                group_tabs = Some(group.tabs.clone());

                if group.tabs.is_empty() {
                    group_id_to_remove = Some(*group_id);
                } else {
                    affected_group_id = Some(*group_id);
                }
                break;
            }
        }

        if let Some(group_id) = group_id_to_remove {
            self.remove_group(group_id);
            return;
        }

        if affected_group_id.is_some() {
            self.mark_dirty();
        }

        if self.active_tab_id == Some(id) {
            if let Some(tabs) = group_tabs {
                self.active_tab_id = tabs.last().map(|t| t.id);
            }
        }
    }

    pub fn clear(&mut self) {
        self.groups.clear();
        self.tabs.clear();
        self.preload_pool.clear();
        self.active_group_id = None;
        self.active_tab_id = None;
        // Exiting the app: drop any pending changes instead of flushing an
        // empty session over the saved one.
        self.groups_dirty = false;
    }

    pub fn set_title(&mut self, id: u64, title: String) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.set_title(title);
        }
    }

    /// Toggle the "important" mark on a tab.
    pub fn toggle_important(&mut self, tab_id: u64) {
        let mut toggled = false;
        for group in self.groups.values_mut() {
            if let Some(tab_info) = group.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab_info.is_important = !tab_info.is_important;
                toggled = true;
                break;
            }
        }
        if toggled {
            self.mark_dirty();
        }
    }

    pub fn set_active_tab(&mut self, id: u64) {
        self.active_tab_id = Some(id);

        for (group_id, group) in &self.groups {
            if group.tabs.iter().any(|t| t.id == id) {
                self.active_group_id = Some(*group_id);
                break;
            }
        }

        if let Some(tab) = self.tabs.get_mut(&id) {
            let is_alternate = tab.is_alternate_screen();
            tab.scroll_state.current(is_alternate).user_scrolled_up = false;
        }
    }

    pub fn switch_to_next_tab(&mut self) {
        if let Some(group_id) = self.active_group_id {
            if let Some(group) = self.groups.get(&group_id) {
                let tabs = &group.tabs;
                if let Some(current_idx) =
                    tabs.iter().position(|t| Some(t.id) == self.active_tab_id)
                {
                    let next_idx = (current_idx + 1) % tabs.len();
                    let new_tab_id = tabs[next_idx].id;
                    self.active_tab_id = Some(new_tab_id);
                    if let Some(tab) = self.tabs.get_mut(&new_tab_id) {
                        let is_alternate = tab.is_alternate_screen();
                        tab.scroll_state.current(is_alternate).user_scrolled_up = false;
                    }
                }
            }
        }
    }

    pub fn switch_to_prev_tab(&mut self) {
        if let Some(group_id) = self.active_group_id {
            if let Some(group) = self.groups.get(&group_id) {
                let tabs = &group.tabs;
                if let Some(current_idx) =
                    tabs.iter().position(|t| Some(t.id) == self.active_tab_id)
                {
                    let prev_idx = if current_idx == 0 {
                        tabs.len() - 1
                    } else {
                        current_idx - 1
                    };
                    let new_tab_id = tabs[prev_idx].id;
                    self.active_tab_id = Some(new_tab_id);
                    if let Some(tab) = self.tabs.get_mut(&new_tab_id) {
                        let is_alternate = tab.is_alternate_screen();
                        tab.scroll_state.current(is_alternate).user_scrolled_up = false;
                    }
                }
            }
        }
    }

    fn get_tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.get_mut(&id)
    }

    pub fn get_tab(&self, id: u64) -> Option<&Tab> {
        self.tabs.get(&id)
    }

    pub fn get_active(&mut self) -> Option<&mut Tab> {
        let _group_id = self.active_group_id?;
        let tab_id = self.active_tab_id?;

        self.tabs.get_mut(&tab_id)
    }

    /// Replace the three `set_*` methods: sync this manager's copy of the
    /// launch config with the owner's. Rebuilds the preload pool when agents
    /// change and enables/disables preloading as configured.
    pub fn update_launch_config(
        &mut self,
        config: &crate::config::TerminalLaunchConfig,
        ctx: &egui::Context,
    ) {
        let agents_changed = self.agents != config.agents;
        self.default_shell_cmd = config.default_shell_cmd.clone();
        self.agents = config.agents.clone();
        self.run_as_login_shell = config.run_as_login_shell;
        // Stale preloaded tabs still run the old agent commands; drop them.
        if agents_changed && self.preload_enabled {
            self.clear_preload_pool();
        }
        self.set_preload_enabled(config.preload_tabs, ctx.clone());
    }

    /// Update the last known terminal content size. Used to seed newly created
    /// terminals so they boot at the correct column/row count.
    pub fn set_terminal_layout_hint(&mut self, size: egui_term::Size) {
        self.terminal_layout_hint = Some(size);
    }

    /// Update the current font cell metrics. Call after font/theme changes.
    pub fn set_cell_metrics_hint(&mut self, metrics: egui_term::Size) {
        self.cell_metrics_hint = Some(metrics);
    }

    /// Current terminal content size hint (last seen by the central panel).
    pub fn terminal_layout_hint(&self) -> Option<egui_term::Size> {
        self.terminal_layout_hint
    }

    // ---- Preload pool ----

    fn spawn_preload_tab(
        &mut self,
        group_id: u64,
        agent_index: Option<usize>,
        ctx: egui::Context,
    ) {
        if !self.preload_enabled {
            return;
        }

        let group_path = match self.groups.get(&group_id) {
            Some(g) => g.path.clone(),
            None => return,
        };

        let (use_agent, shell_cmd) = match agent_index {
            Some(idx) => match self.agents.get(idx) {
                Some(a) if a.enabled && !a.cmd.trim().is_empty() => (true, a.cmd.clone()),
                _ => return,
            },
            None => (false, self.default_shell_cmd.clone()),
        };

        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let tab = match Tab::new(
            ctx,
            self.command_sender.clone(),
            tab_id,
            Some(group_path),
            &shell_cmd,
            use_agent,
            !use_agent && self.run_as_login_shell,
            self.terminal_layout_hint,
            self.cell_metrics_hint,
        ) {
            Ok(tab) => tab,
            Err(e) => {
                log::warn!("Failed to preload tab for group {}: {}", group_id, e);
                return;
            }
        };

        self.preload_pool.insert((group_id, agent_index), (tab_id, tab));
    }

    pub fn populate_preload_for_group(&mut self, group_id: u64, ctx: egui::Context) {
        if !self.preload_enabled {
            return;
        }

        let key = (group_id, None);
        if !self.preload_pool.contains_key(&key) {
            self.spawn_preload_tab(group_id, None, ctx.clone());
        }

        for i in 0..MAX_AGENTS {
            let key = (group_id, Some(i));
            if !self.preload_pool.contains_key(&key) {
                if let Some(agent) = self.agents.get(i) {
                    if agent.enabled && !agent.cmd.trim().is_empty() {
                        self.spawn_preload_tab(group_id, Some(i), ctx.clone());
                    }
                }
            }
        }
    }

    pub fn populate_preload_pool(&mut self, ctx: egui::Context) {
        if !self.preload_enabled {
            return;
        }
        let group_ids: Vec<u64> = self.groups.keys().copied().collect();
        for group_id in group_ids {
            self.populate_preload_for_group(group_id, ctx.clone());
        }
    }

    fn clear_preload_for_group(&mut self, group_id: u64) {
        self.preload_pool.retain(|(gid, _), _| *gid != group_id);
    }

    pub fn clear_preload_pool(&mut self) {
        self.preload_pool.clear();
    }

    pub fn remove_preload_tab(&mut self, tab_id: u64) {
        self.preload_pool.retain(|_, (id, _)| *id != tab_id);
    }

    pub fn set_preload_enabled(&mut self, enabled: bool, ctx: egui::Context) {
        self.preload_enabled = enabled;
        if enabled {
            self.populate_preload_pool(ctx);
        } else {
            self.clear_preload_pool();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sample_groups() -> BTreeMap<u64, TabGroup> {
        let mut group = TabGroup::new(1, "proj".to_string(), PathBuf::from("/tmp/proj"));
        group.tabs.push(TabInfo {
            id: 10,
            is_agent: false,
            agent_index: None,
            is_important: false,
        });
        group.tabs.push(TabInfo {
            id: 11,
            is_agent: true,
            agent_index: Some(0),
            is_important: true,
        });
        let mut groups = BTreeMap::new();
        groups.insert(1, group);
        groups
    }

    #[test]
    fn groups_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("groups.json");
        let groups = sample_groups();

        write_groups_file(&path, &groups);
        let loaded = read_groups_file(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "proj");
        assert_eq!(loaded[0].path, PathBuf::from("/tmp/proj"));
        assert_eq!(loaded[0].tabs.len(), 2);
        assert_eq!(loaded[0].tabs[0].id, 10);
        assert!(loaded[0].tabs[1].is_important);
        assert_eq!(loaded[0].tabs[1].agent_index, Some(0));
    }

    #[test]
    fn groups_file_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");
        assert!(read_groups_file(&path).is_none());
    }

    #[test]
    fn groups_file_corrupt_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("groups.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(read_groups_file(&path).is_none());
    }

    #[test]
    fn groups_file_missing_tab_fields_use_defaults() {
        // A session file written before is_important/agent_index existed must
        // still deserialize with defaults filled in.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("groups.json");
        std::fs::write(
            &path,
            r#"[{"id":1,"name":"p","path":"/tmp/p","tabs":[{"id":3,"is_agent":false}]}]"#,
        )
        .unwrap();
        let loaded = read_groups_file(&path).unwrap();
        assert!(!loaded[0].tabs[0].is_important);
        assert_eq!(loaded[0].tabs[0].agent_index, None);
    }

    #[test]
    fn name_from_path_uses_file_name() {
        assert_eq!(
            TabGroup::name_from_path(&PathBuf::from("/home/user/my-project")),
            "my-project"
        );
    }

    #[test]
    fn name_from_path_root_falls_back_to_full_path() {
        let name = TabGroup::name_from_path(&PathBuf::from("/"));
        assert_eq!(name, "/");
    }

    #[test]
    fn name_from_path_relative_no_file_name_falls_back() {
        let name = TabGroup::name_from_path(&PathBuf::from("."));
        assert_eq!(name, ".");
    }
}

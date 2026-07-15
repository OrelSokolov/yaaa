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
    pub display_name: String,
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

    pub fn name_from_path(path: &PathBuf) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }
}

pub struct TabManager {
    command_sender: Sender<(u64, PtyEvent)>,
    pub groups: BTreeMap<u64, TabGroup>,
    tabs: BTreeMap<u64, Tab>,
    /// Key: (group_id, agent_index) where agent_index is None for terminal.
    preload_pool: HashMap<(u64, Option<usize>), (u64, Tab)>,
    pub active_group_id: Option<u64>,
    pub active_tab_id: Option<u64>,
    next_group_id: u64,
    next_tab_id: u64,
    pub default_shell_cmd: String,
    pub agents: [AgentConfig; MAX_AGENTS],
    pub run_as_login_shell: bool,
    preload_enabled: bool,
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
        default_shell_cmd: String,
        agents: [AgentConfig; MAX_AGENTS],
        run_as_login_shell: bool,
        preload_enabled: bool,
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
            default_shell_cmd,
            agents,
            run_as_login_shell,
            preload_enabled,
            terminal_layout_hint,
            cell_metrics_hint,
        };

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        if let Some(groups_data) = manager.load_groups() {
            for mut group in groups_data {
                manager.next_group_id = manager.next_group_id.max(group.id + 1);
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

                    let tab = Tab::new(
                        cc.egui_ctx.clone(),
                        manager.command_sender.clone(),
                        tab_info.id,
                        Some(group.path.clone()),
                        &shell_cmd,
                        use_agent,
                        !use_agent && manager.run_as_login_shell,
                        manager.terminal_layout_hint,
                        manager.cell_metrics_hint,
                    );
                    manager.tabs.insert(tab_info.id, tab);
                }
                manager.groups.insert(group.id, group);
            }
            if let Some(first_group) = manager.groups.first_key_value() {
                manager.active_group_id = Some(*first_group.0);
                manager.active_tab_id = first_group.1.tabs.first().map(|t| t.id);
            }

            manager.refresh_all_display_names();
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
        if let Some(config_dir) = crate::config::config_dir() {
            let groups_file = config_dir.join(GROUPS_FILE);
            if groups_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&groups_file) {
                    if let Ok(groups) = serde_json::from_str::<Vec<TabGroup>>(&content) {
                        return Some(groups);
                    }
                }
            }
        }
        None
    }

    pub fn save_groups(&self) {
        if let Some(config_dir) = crate::config::config_dir() {
            let groups_file = config_dir.join(GROUPS_FILE);
            if let Ok(groups) =
                serde_json::to_string_pretty(&self.groups.values().collect::<Vec<_>>())
            {
                let _ = std::fs::write(&groups_file, groups);
            }
        }
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

        self.add_tab_to_group(group_id, ctx.clone(), None);
        self.populate_preload_for_group(group_id, ctx);
    }

    pub fn rename_group(&mut self, group_id: u64, new_name: String) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.name = new_name;
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
                        display_name: String::new(),
                    });
                }

                self.refresh_display_names(group_id);
                self.active_group_id = Some(group_id);
                self.active_tab_id = Some(tab_id);

                self.spawn_preload_tab(group_id, agent_index, ctx);
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

        let tab = Tab::new(
            ctx,
            self.command_sender.clone(),
            tab_id,
            group_path,
            &shell_cmd,
            use_agent,
            !use_agent && self.run_as_login_shell,
            self.terminal_layout_hint,
            self.cell_metrics_hint,
        );
        self.tabs.insert(tab_id, tab);

        if let Some(group) = self.groups.get_mut(&group_id) {
            group.tabs.push(TabInfo {
                id: tab_id,
                is_agent: use_agent,
                agent_index: if use_agent { agent_index } else { None },
                display_name: String::new(),
            });
        }

        self.refresh_display_names(group_id);

        self.active_group_id = Some(group_id);
        self.active_tab_id = Some(tab_id);
    }

    pub fn remove_group(&mut self, group_id: u64) {
        if let Some(group) = self.groups.get(&group_id) {
            for tab_info in &group.tabs {
                self.tabs.remove(&tab_info.id);
            }
        }
        self.clear_preload_for_group(group_id);
        self.groups.remove(&group_id);

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

        if let Some(group_id) = affected_group_id {
            self.refresh_display_names(group_id);
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
    }

    pub fn set_title(&mut self, id: u64, title: String) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.set_title(title);
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

    fn format_tab_name(&self, tab_info: &TabInfo, index: usize) -> String {
        if let Some(idx) = tab_info.agent_index {
            let agent_name = self
                .agents
                .get(idx)
                .filter(|a| !a.name.trim().is_empty())
                .map(|a| a.name.clone())
                .unwrap_or_else(|| format!("Агент {}", idx + 1));
            format!("{}. {} 💬", index + 1, agent_name)
        } else {
            format!("{}. Terminal", index + 1)
        }
    }

    fn refresh_display_names(&mut self, group_id: u64) {
        let names: Vec<String> = if let Some(group) = self.groups.get(&group_id) {
            group
                .tabs
                .iter()
                .enumerate()
                .map(|(i, tab_info)| self.format_tab_name(tab_info, i))
                .collect()
        } else {
            return;
        };

        if let Some(group) = self.groups.get_mut(&group_id) {
            for (tab_info, name) in group.tabs.iter_mut().zip(names) {
                tab_info.display_name = name;
            }
        }
    }

    fn refresh_all_display_names(&mut self) {
        let group_ids: Vec<u64> = self.groups.keys().copied().collect();
        for group_id in group_ids {
            self.refresh_display_names(group_id);
        }
    }

    fn get_tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.get_mut(&id)
    }

    pub fn get_active(&mut self) -> Option<&mut Tab> {
        let _group_id = self.active_group_id?;
        let tab_id = self.active_tab_id?;

        self.tabs.get_mut(&tab_id)
    }

    pub fn set_default_shell_cmd(&mut self, shell_cmd: String) {
        self.default_shell_cmd = shell_cmd;
    }

    pub fn set_agents(&mut self, agents: [AgentConfig; MAX_AGENTS], ctx: egui::Context) {
        self.agents = agents;
        self.refresh_all_display_names();

        if self.preload_enabled {
            self.clear_preload_pool();
            self.populate_preload_pool(ctx);
        }
    }

    pub fn set_run_as_login_shell(&mut self, run_as_login_shell: bool) {
        self.run_as_login_shell = run_as_login_shell;
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

        let tab = Tab::new(
            ctx,
            self.command_sender.clone(),
            tab_id,
            Some(group_path),
            &shell_cmd,
            use_agent,
            !use_agent && self.run_as_login_shell,
            self.terminal_layout_hint,
            self.cell_metrics_hint,
        );

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

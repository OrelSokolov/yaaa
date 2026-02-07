use crate::constants::GROUPS_FILE;
use crate::terminal::tab::Tab;
use egui_term::PtyEvent;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::mpsc::Sender};

#[derive(Serialize, Deserialize, Clone)]
pub struct TabInfo {
    pub id: u64,
    pub is_agent: bool,
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
    pub active_group_id: Option<u64>,
    pub active_tab_id: Option<u64>,
    next_group_id: u64,
    next_tab_id: u64,
    pub default_shell_cmd: String,
    pub default_agent_cmd: String,
    pub run_as_login_shell: bool,
}

impl TabManager {
    pub fn new(
        command_sender: Sender<(u64, PtyEvent)>,
        cc: &eframe::CreationContext<'_>,
        default_shell_cmd: String,
        default_agent_cmd: String,
        run_as_login_shell: bool,
    ) -> Self {
        let mut manager = Self {
            command_sender,
            groups: BTreeMap::new(),
            tabs: BTreeMap::new(),
            active_group_id: None,
            active_tab_id: None,
            next_group_id: 0,
            next_tab_id: 0,
            default_shell_cmd,
            default_agent_cmd,
            run_as_login_shell,
        };

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        if let Some(groups_data) = manager.load_groups() {
            for mut group in groups_data {
                manager.next_group_id = manager.next_group_id.max(group.id + 1);
                for tab_info in &mut group.tabs {
                    manager.next_tab_id = manager.next_tab_id.max(tab_info.id + 1);
                    let use_agent =
                        tab_info.is_agent && Tab::command_exists(&manager.default_agent_cmd);
                    tab_info.is_agent = use_agent;
                    let shell_cmd = if use_agent {
                        &manager.default_agent_cmd
                    } else {
                        &manager.default_shell_cmd
                    };
                    let tab = Tab::new(
                        cc.egui_ctx.clone(),
                        manager.command_sender.clone(),
                        tab_info.id,
                        Some(group.path.clone()),
                        shell_cmd,
                        use_agent,
                        manager.run_as_login_shell,
                    );
                    manager.tabs.insert(tab_info.id, tab);
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

            manager.add_tab_to_group(group_id, cc.egui_ctx.clone(), false);
        }

        manager
    }

    fn get_groups_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("yaaa");
            let _ = std::fs::create_dir_all(&path);
            path
        })
    }

    fn load_groups(&mut self) -> Option<Vec<TabGroup>> {
        if let Some(config_dir) = Self::get_groups_dir() {
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
        if let Some(config_dir) = Self::get_groups_dir() {
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

        self.add_tab_to_group(group_id, ctx, false);
    }

    pub fn rename_group(&mut self, group_id: u64, new_name: String) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.name = new_name;
        }
    }

    pub fn add_tab_to_group(&mut self, group_id: u64, ctx: egui::Context, is_agent: bool) {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let group_path = self.groups.get(&group_id).map(|g| g.path.clone());
        let use_agent = is_agent && Tab::command_exists(&self.default_agent_cmd);
        let shell_cmd = if use_agent {
            &self.default_agent_cmd
        } else {
            &self.default_shell_cmd
        };
        let tab = Tab::new(
            ctx,
            self.command_sender.clone(),
            tab_id,
            group_path,
            shell_cmd,
            use_agent,
            self.run_as_login_shell,
        );
        self.tabs.insert(tab_id, tab);

        if let Some(group) = self.groups.get_mut(&group_id) {
            group.tabs.push(TabInfo {
                id: tab_id,
                is_agent: use_agent,
            });
        }

        self.active_group_id = Some(group_id);
        self.active_tab_id = Some(tab_id);
    }

    pub fn remove_group(&mut self, group_id: u64) {
        if let Some(group) = self.groups.get(&group_id) {
            for tab_info in &group.tabs {
                self.tabs.remove(&tab_info.id);
            }
        }
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
        let mut group_tabs = None;

        for (group_id, group) in &mut self.groups {
            if group.tabs.iter().any(|t| t.id == id) {
                group.tabs.retain(|t| t.id != id);
                self.tabs.remove(&id);
                group_tabs = Some(group.tabs.clone());

                if group.tabs.is_empty() {
                    group_id_to_remove = Some(*group_id);
                }
                break;
            }
        }

        if let Some(group_id) = group_id_to_remove {
            self.remove_group(group_id);
            return;
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

    pub fn get_tab_name(&self, group_id: u64, id: u64) -> String {
        let is_agent = self.tabs.get(&id).map(|t| t.is_agent).unwrap_or(false);
        if let Some(group) = self.groups.get(&group_id) {
            for (i, tab_info) in group.tabs.iter().enumerate() {
                if tab_info.id == id {
                    return if is_agent {
                        format!("{}. Agent 💬", i + 1)
                    } else {
                        format!("{}. Terminal", i + 1)
                    };
                }
            }
        }
        if is_agent {
            format!("{}. ➕ Agent", id + 1)
        } else {
            format!("{}. Terminal", id + 1)
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

    pub fn set_default_agent_cmd(&mut self, agent_cmd: String) {
        self.default_agent_cmd = agent_cmd;
    }

    pub fn set_run_as_login_shell(&mut self, run_as_login_shell: bool) {
        self.run_as_login_shell = run_as_login_shell;
    }
}

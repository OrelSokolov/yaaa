use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::constants::*;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct RecentProject {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct RecentProjects {
    pub projects: Vec<RecentProject>,
}

impl RecentProjects {
    pub fn get_config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("yaaa");
            let _ = std::fs::create_dir_all(&path);
            path
        })
    }

    pub fn load() -> Self {
        if let Some(config_dir) = Self::get_config_dir() {
            let recent_projects_file = config_dir.join(RECENT_PROJECTS_FILE);
            if recent_projects_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&recent_projects_file) {
                    if let Ok(recent_projects) = serde_json::from_str::<RecentProjects>(&content) {
                        return recent_projects;
                    }
                }
            }
        }
        RecentProjects::default()
    }

    pub fn save(&self) {
        if let Some(config_dir) = Self::get_config_dir() {
            let recent_projects_file = config_dir.join(RECENT_PROJECTS_FILE);
            if let Ok(recent_projects_json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&recent_projects_file, recent_projects_json);
            }
        }
    }

    pub fn add_project(&mut self, name: String, path: PathBuf) {
        self.projects.retain(|p| p.path != path);
        self.projects.insert(0, RecentProject { name, path });

        if self.projects.len() > 20 {
            self.projects.truncate(20);
        }
    }
}

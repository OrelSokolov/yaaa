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
    pub fn load() -> Self {
        if let Some(config_dir) = super::config_dir() {
            let recent_projects_file = config_dir.join(RECENT_PROJECTS_FILE);
            if let Ok(content) = std::fs::read_to_string(&recent_projects_file) {
                match serde_json::from_str::<RecentProjects>(&content) {
                    Ok(recent_projects) => return recent_projects,
                    Err(e) => {
                        log::warn!(
                            "Corrupt recent projects file {}: {} — backing it up, using defaults",
                            recent_projects_file.display(),
                            e
                        );
                        super::backup_corrupt(&recent_projects_file);
                    }
                }
            }
        }
        RecentProjects::default()
    }

    pub fn save(&self) {
        if let Some(config_dir) = super::config_dir() {
            let recent_projects_file = config_dir.join(RECENT_PROJECTS_FILE);
            if let Ok(recent_projects_json) = serde_json::to_string_pretty(self) {
                if let Err(e) =
                    super::write_atomic(&recent_projects_file, &recent_projects_json)
                {
                    log::warn!("Could not save recent projects: {}", e);
                }
            }
        }
    }

    pub fn add_project(&mut self, name: String, path: PathBuf) {
        self.projects.retain(|p| p.path != path);
        self.projects.insert(0, RecentProject { name, path });

        if self.projects.len() > RECENT_PROJECTS_LIMIT {
            self.projects.truncate(RECENT_PROJECTS_LIMIT);
        }
    }

    pub fn remove_project(&mut self, path: &std::path::Path) {
        self.projects.retain(|p| p.path != path);
    }
}

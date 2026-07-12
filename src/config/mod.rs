use std::path::PathBuf;

pub mod recent_projects;
pub mod settings;

pub use recent_projects::RecentProjects;
pub use settings::Settings;

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push("yaaa");
        let _ = std::fs::create_dir_all(&path);
        path
    })
}

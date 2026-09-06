use std::path::{Path, PathBuf};

pub mod launch;
pub mod recent_projects;
pub mod settings;

pub use launch::TerminalLaunchConfig;
pub use recent_projects::RecentProjects;
pub use settings::Settings;

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push("h2term");
        let _ = std::fs::create_dir_all(&path);
        path
    })
}

/// Write `contents` to `path` atomically: write to a sibling temp file, then
/// rename it over the destination. A crash mid-write can never leave a
/// truncated config file behind.
pub(crate) fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// Preserve an unreadable/corrupt file as `<name>.bak` before the app
/// overwrites it with fresh defaults, so the user can recover their data.
pub(crate) fn backup_corrupt(path: &Path) {
    let bak = path.with_extension("bak");
    let result = std::fs::rename(path, &bak)
        .or_else(|_| std::fs::copy(path, &bak).map(|_| ()));
    if let Err(e) = result {
        log::warn!(
            "Could not back up corrupt file {}: {}",
            path.display(),
            e
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_creates_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.json");

        write_atomic(&path, "one").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one");

        write_atomic(&path, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");

        // The temp file must not linger next to the target.
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(entries, vec![path]);
    }

    #[test]
    fn backup_corrupt_preserves_content_as_bak() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.json");
        std::fs::write(&path, "broken").unwrap();

        backup_corrupt(&path);

        let bak = dir.path().join("x.bak");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), "broken");
    }
}

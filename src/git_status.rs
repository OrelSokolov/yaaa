use git2::{BranchType, Repository, StatusOptions};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Synchronization state of a Git branch relative to its upstream.
/// Conflicts take precedence over divergence/pull/push because they must be
/// resolved before any sync operation is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitSyncStatus {
    /// Branch is up to date with its upstream and has no local changes.
    Clean,
    /// Working tree or index has uncommitted changes.
    Dirty,
    /// Local branch has commits that the remote does not have.
    NeedsPush,
    /// Remote branch has commits that the local branch does not have.
    NeedsPull,
    /// Local and remote have diverged (both ahead and behind).
    Diverged,
    /// There are unmerged files (merge/rebase/cherry-pick in progress).
    Conflicts,
}

impl GitSyncStatus {
    /// Visual icon shown in the UI for this sync state.
    pub fn icon(&self) -> &'static str {
        match self {
            GitSyncStatus::Clean => "✓",
            GitSyncStatus::Dirty => "⚠",
            GitSyncStatus::NeedsPush => "⬆",
            GitSyncStatus::NeedsPull => "⬇",
            GitSyncStatus::Diverged => "✖",
            GitSyncStatus::Conflicts => "✖",
        }
    }

    /// Color used when drawing the status icon in the sidebar.
    pub fn color(&self) -> egui::Color32 {
        match self {
            GitSyncStatus::Clean => egui::Color32::from_rgb(0x4c, 0xaf, 0x50),
            GitSyncStatus::Dirty => egui::Color32::from_rgb(0xff, 0x98, 0x00),
            GitSyncStatus::NeedsPush => egui::Color32::from_rgb(0xab, 0x47, 0xbc),
            GitSyncStatus::NeedsPull => egui::Color32::from_rgb(0x42, 0xa5, 0xf5),
            GitSyncStatus::Diverged | GitSyncStatus::Conflicts => {
                egui::Color32::from_rgb(0xf4, 0x43, 0x36)
            }
        }
    }

    /// Short human-readable description, useful for tooltips.
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            GitSyncStatus::Clean => "Up to date",
            GitSyncStatus::Dirty => "Uncommitted changes",
            GitSyncStatus::NeedsPush => "Needs push",
            GitSyncStatus::NeedsPull => "Needs pull",
            GitSyncStatus::Diverged => "Diverged",
            GitSyncStatus::Conflicts => "Merge conflicts",
        }
    }
}

/// Complete Git status for a repository path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    /// Current branch name, if any. `None` for detached HEAD.
    pub branch: Option<String>,
    /// How the local branch compares to its upstream.
    pub sync_status: GitSyncStatus,
}

/// Detect Git status for the directory at `path` using libgit2.
/// Returns `None` if `path` is not inside a Git repository.
pub fn compute_git_status(path: &Path) -> Option<GitStatus> {
    let repo = Repository::open(path).ok()?;
    let head = repo.head().ok()?;
    let branch = head.shorthand().map(|s| s.to_string());

    // Working tree / index status.
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false)
        .exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts)).ok()?;

    let mut dirty = false;
    let mut conflicts = false;
    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_conflicted() {
            conflicts = true;
        } else {
            dirty = true;
        }
    }

    // Ahead/behind upstream.
    let (ahead, behind) = head
        .shorthand()
        .and_then(|name| {
            let local = repo.find_branch(name, BranchType::Local).ok()?;
            let upstream = local.upstream().ok()?;
            let local_oid = head.target()?;
            let upstream_oid = upstream.get().target()?;
            repo.graph_ahead_behind(local_oid, upstream_oid).ok()
        })
        .unwrap_or((0, 0));

    let sync_status = if conflicts {
        GitSyncStatus::Conflicts
    } else if ahead > 0 && behind > 0 {
        GitSyncStatus::Diverged
    } else if ahead > 0 {
        GitSyncStatus::NeedsPush
    } else if behind > 0 {
        GitSyncStatus::NeedsPull
    } else if dirty {
        GitSyncStatus::Dirty
    } else {
        GitSyncStatus::Clean
    };

    Some(GitStatus {
        branch,
        sync_status,
    })
}

/// Background-thread Git status cache.
///
/// The UI thread never blocks: [`get_or_refresh`](GitStatusCache::get_or_refresh)
/// reads from a shared cache and registers unknown paths for the background
/// thread to compute. The thread polls all known paths every `refresh_interval`
/// using libgit2 (no external `git` process), so there are no `conhost.exe`
/// window flashes on Windows.
pub struct GitStatusCache {
    cache: Arc<Mutex<HashMap<PathBuf, Option<GitStatus>>>>,
    known_paths: Arc<Mutex<HashSet<PathBuf>>>,
    shutdown: Arc<AtomicBool>,
    _thread: Option<thread::JoinHandle<()>>,
}

impl GitStatusCache {
    /// Create a new cache and spawn the background refresh thread.
    /// `refresh_interval` controls how often the thread recomputes statuses.
    pub fn new(refresh_interval: Duration) -> Self {
        let cache: Arc<Mutex<HashMap<PathBuf, Option<GitStatus>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let known_paths: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let cache_clone = Arc::clone(&cache);
        let paths_clone = Arc::clone(&known_paths);
        let shutdown_clone = Arc::clone(&shutdown);

        let handle = thread::Builder::new()
            .name("git-status-watcher".into())
            .spawn(move || {
                while !shutdown_clone.load(Ordering::Relaxed) {
                    let paths: Vec<PathBuf> =
                        paths_clone.lock().unwrap().iter().cloned().collect();
                    for path in &paths {
                        if shutdown_clone.load(Ordering::Relaxed) {
                            break;
                        }
                        let status = compute_git_status(path);
                        cache_clone
                            .lock()
                            .unwrap()
                            .insert(path.clone(), status);
                    }
                    // Sleep in small increments so shutdown is responsive.
                    let mut elapsed = Duration::ZERO;
                    let step = Duration::from_millis(200);
                    while elapsed < refresh_interval {
                        if shutdown_clone.load(Ordering::Relaxed) {
                            return;
                        }
                        thread::sleep(step);
                        elapsed += step;
                    }
                }
            })
            .ok();

        Self {
            cache,
            known_paths,
            shutdown,
            _thread: handle,
        }
    }

    /// Return a cached status for `path`, registering it for background
    /// refresh if not already known. Returns `None` if the path has not been
    /// checked yet or is not a Git repository.
    pub fn get_or_refresh(&self, path: &Path) -> Option<GitStatus> {
        self.known_paths.lock().unwrap().insert(path.to_path_buf());
        self.cache
            .lock()
            .unwrap()
            .get(path)
            .and_then(|opt| opt.clone())
    }

    /// Remove entries for paths that no longer satisfy `predicate`.
    pub fn retain<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&Path) -> bool,
    {
        self.cache.lock().unwrap().retain(|path, _| predicate(path));
        self.known_paths
            .lock()
            .unwrap()
            .retain(|path| predicate(path));
    }
}

impl Drop for GitStatusCache {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self._thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_and_labels() {
        assert_eq!(GitSyncStatus::Clean.icon(), "✓");
        assert_eq!(GitSyncStatus::Dirty.icon(), "⚠");
        assert_eq!(GitSyncStatus::NeedsPush.icon(), "⬆");
        assert_eq!(GitSyncStatus::NeedsPull.icon(), "⬇");
        assert_eq!(GitSyncStatus::Diverged.icon(), "✖");
        assert_eq!(GitSyncStatus::Conflicts.icon(), "✖");

        assert_eq!(GitSyncStatus::Clean.label(), "Up to date");
        assert_eq!(GitSyncStatus::Dirty.label(), "Uncommitted changes");
        assert_eq!(GitSyncStatus::NeedsPush.label(), "Needs push");
        assert_eq!(GitSyncStatus::NeedsPull.label(), "Needs pull");
        assert_eq!(GitSyncStatus::Diverged.label(), "Diverged");
        assert_eq!(GitSyncStatus::Conflicts.label(), "Merge conflicts");
    }

    fn git_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git should start");
        assert!(status.success(), "git {:?} failed in {:?}", args, dir);
    }

    #[test]
    fn test_compute_git_status_clean() {
        if !git_available() {
            return;
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();

        run_git(dir, &["init", "--quiet"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test"]);

        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        run_git(dir, &["add", "file.txt"]);
        run_git(dir, &["commit", "--quiet", "-m", "initial"]);

        let status = compute_git_status(dir).expect("should detect git repo");
        // Git >= 2.28 may default to "main" instead of "master".
        assert!(
            status.branch.as_deref() == Some("master")
                || status.branch.as_deref() == Some("main"),
            "unexpected branch: {:?}",
            status.branch
        );
        assert_eq!(status.sync_status, GitSyncStatus::Clean);
    }

    #[test]
    fn test_compute_git_status_dirty() {
        if !git_available() {
            return;
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();

        run_git(dir, &["init", "--quiet"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test"]);

        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        run_git(dir, &["add", "file.txt"]);
        run_git(dir, &["commit", "--quiet", "-m", "initial"]);

        // Modify the file without committing.
        std::fs::write(dir.join("file.txt"), "modified").unwrap();

        let status = compute_git_status(dir).expect("should detect git repo");
        assert_eq!(status.sync_status, GitSyncStatus::Dirty);
    }

    #[test]
    fn test_compute_git_status_conflicts() {
        if !git_available() {
            return;
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();

        run_git(dir, &["init", "--quiet"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test"]);

        std::fs::write(dir.join("file.txt"), "base").unwrap();
        run_git(dir, &["add", "file.txt"]);
        run_git(dir, &["commit", "--quiet", "-m", "base"]);

        // Create a topic branch and change the file.
        run_git(dir, &["checkout", "--quiet", "-b", "topic"]);
        std::fs::write(dir.join("file.txt"), "topic-line").unwrap();
        run_git(dir, &["add", "file.txt"]);
        run_git(dir, &["commit", "--quiet", "-m", "topic change"]);

        // Go back to master and change the same file differently.
        run_git(dir, &["checkout", "--quiet", "master"]);
        std::fs::write(dir.join("file.txt"), "master-line").unwrap();
        run_git(dir, &["add", "file.txt"]);
        run_git(dir, &["commit", "--quiet", "-m", "master change"]);

        // Merge topic into master: this will conflict.
        let merge = std::process::Command::new("git")
            .args(["merge", "topic"])
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(!merge.success(), "merge should conflict in this test");

        let git_status = compute_git_status(dir).expect("should detect git repo");
        assert_eq!(git_status.sync_status, GitSyncStatus::Conflicts);
    }

    #[test]
    fn test_compute_git_status_not_a_repo() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(compute_git_status(tmp.path()).is_none());
    }

    #[test]
    fn test_cache_returns_status_after_refresh() {
        if !git_available() {
            return;
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();

        run_git(dir, &["init", "--quiet"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test"]);
        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        run_git(dir, &["add", "file.txt"]);
        run_git(dir, &["commit", "--quiet", "-m", "initial"]);

        let cache = GitStatusCache::new(Duration::from_millis(100));

        // First call registers the path but the background thread hasn't
        // computed the status yet.
        assert!(cache.get_or_refresh(dir).is_none());

        // Wait for the background thread to process it.
        thread::sleep(Duration::from_millis(400));

        let status = cache.get_or_refresh(dir).expect("should be cached");
        assert_eq!(status.sync_status, GitSyncStatus::Clean);
    }
}

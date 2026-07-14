use git2::Repository;
use notify_debouncer_mini::notify::{recommended_watcher, Event, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Git status displayed next to a project in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    /// Working tree is clean and the local branch matches its upstream.
    Clean,
    /// Working tree or index has modifications.
    Dirty,
    /// Index contains conflicted entries.
    Conflicts,
    /// Local branch is ahead or behind its upstream.
    Diverged,
    /// The directory is not a Git repository.
    NotARepo,
    /// An error occurred while checking the repository.
    Error,
}

impl GitStatus {
    /// Icon shown in the sidebar.
    pub fn icon(&self) -> &'static str {
        match self {
            GitStatus::Clean => "✓",
            GitStatus::Dirty => "⚠",
            GitStatus::Conflicts => "✖",
            GitStatus::Diverged => "✖",
            GitStatus::NotARepo => "-",
            GitStatus::Error => "?",
        }
    }

    /// Color for the status icon.
    pub fn color(&self) -> egui::Color32 {
        match self {
            GitStatus::Clean => egui::Color32::from_rgb(0x4c, 0xaf, 0x50),
            GitStatus::Dirty => egui::Color32::from_rgb(0xff, 0x98, 0x00),
            GitStatus::Conflicts | GitStatus::Diverged => egui::Color32::from_rgb(0xf4, 0x43, 0x36),
            GitStatus::NotARepo | GitStatus::Error => egui::Color32::from_rgb(0x80, 0x80, 0x80),
        }
    }
}

#[derive(Debug, Clone)]
struct GitStatusEntry {
    status: GitStatus,
    #[allow(dead_code)]
    updated_at: Instant,
}

#[derive(Debug)]
enum ServiceCommand {
    SetPaths(Vec<PathBuf>),
    Enable,
    Disable,
    Shutdown,
}

/// Independent Git status service.
///
/// Keeps its own cache and watcher. The UI only calls [`status_for`](GitService::status_for)
/// during repaint. The service runs on a background thread so the UI never blocks.
pub struct GitService {
    enabled: Arc<AtomicBool>,
    cache: Arc<Mutex<HashMap<PathBuf, GitStatusEntry>>>,
    last_paths: Arc<Mutex<Vec<PathBuf>>>,
    command_sender: mpsc::Sender<ServiceCommand>,
    _thread: thread::JoinHandle<()>,
}

impl GitService {
    /// Create a new service. If `enabled` is true the service is ready to watch
    /// paths, but no paths are watched until [`set_paths`](GitService::set_paths)
    /// is called.
    pub fn new(enabled: bool, repaint_context: Option<egui::Context>) -> Self {
        let enabled = Arc::new(AtomicBool::new(enabled));
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let last_paths = Arc::new(Mutex::new(Vec::new()));

        let (command_sender, command_receiver) = mpsc::channel();
        let enabled_clone = enabled.clone();
        let cache_clone = cache.clone();
        let last_paths_clone = last_paths.clone();
        let repaint_context_clone = repaint_context.clone();

        let thread = thread::spawn(move || {
            service_thread(
                enabled_clone,
                cache_clone,
                last_paths_clone,
                repaint_context_clone,
                command_receiver,
            );
        });

        Self {
            enabled,
            cache,
            last_paths,
            command_sender,
            _thread: thread,
        }
    }

    /// Turn status checking on. Watches are registered for the current paths.
    pub fn enable(&self) {
        let _ = self.command_sender.send(ServiceCommand::Enable);
    }

    /// Turn status checking off. Watches are removed and the cache is cleared.
    pub fn disable(&self) {
        let _ = self.command_sender.send(ServiceCommand::Disable);
    }

    /// Replace the list of watched project paths.
    pub fn set_paths(&self, paths: Vec<PathBuf>) {
        if let Ok(mut last) = self.last_paths.lock() {
            if *last == paths {
                return;
            }
            *last = paths.clone();
        }
        let _ = self.command_sender.send(ServiceCommand::SetPaths(paths));
    }

    /// Read the cached status for a project path. Returns `None` if the path
    /// has not been checked yet or if the service is disabled.
    pub fn status_for(&self, path: &Path) -> Option<GitStatus> {
        self.cache
            .lock()
            .ok()
            .and_then(|c| c.get(path).map(|e| e.status))
    }

    /// Whether the service is currently enabled.
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Drop for GitService {
    fn drop(&mut self) {
        let _ = self.command_sender.send(ServiceCommand::Shutdown);
    }
}

fn service_thread(
    enabled: Arc<AtomicBool>,
    cache: Arc<Mutex<HashMap<PathBuf, GitStatusEntry>>>,
    last_paths: Arc<Mutex<Vec<PathBuf>>>,
    repaint_context: Option<egui::Context>,
    command_rx: mpsc::Receiver<ServiceCommand>,
) {
    let (event_tx, event_rx) =
        mpsc::channel::<Result<Event, notify_debouncer_mini::notify::Error>>();
    let mut watcher: notify_debouncer_mini::notify::RecommendedWatcher =
        match recommended_watcher(event_tx) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create Git file watcher: {}", e);
                return;
            }
        };

    let mut watched_projects: Vec<PathBuf> = Vec::new();
    let mut pending_paths: Vec<PathBuf> = Vec::new();
    let mut buffered_paths: HashSet<PathBuf> = HashSet::new();
    let mut last_event_time: Option<Instant> = None;
    const DEBOUNCE: Duration = Duration::from_millis(500);

    loop {
        // Process commands from the UI.
        loop {
            match command_rx.try_recv() {
                Ok(ServiceCommand::Shutdown) => return,
                Ok(ServiceCommand::Enable) => {
                    if enabled.load(Ordering::Relaxed) {
                        continue;
                    }
                    enabled.store(true, Ordering::Relaxed);
                    let paths = last_paths.lock().map(|g| g.clone()).unwrap_or_default();
                    update_watches(&mut watcher, &paths);
                    watched_projects = paths;
                    pending_paths.extend(watched_projects.iter().cloned());
                }
                Ok(ServiceCommand::Disable) => {
                    enabled.store(false, Ordering::Relaxed);
                    clear_watches(&mut watcher, &watched_projects);
                    watched_projects.clear();
                    if let Ok(mut c) = cache.lock() {
                        c.clear();
                    }
                }
                Ok(ServiceCommand::SetPaths(paths)) => {
                    if enabled.load(Ordering::Relaxed) {
                        clear_watches(&mut watcher, &watched_projects);
                        update_watches(&mut watcher, &paths);
                        pending_paths.extend(paths.iter().cloned());
                    }
                    watched_projects = paths;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // Process filesystem events with a leading-edge debounce:
        // the first event after a quiet period is handled immediately,
        // subsequent events within DEBOUNCE are buffered and flushed once.
        loop {
            match event_rx.try_recv() {
                Ok(Ok(event)) => {
                    if !enabled.load(Ordering::Relaxed) {
                        continue;
                    }
                    for path in event.paths {
                        log::debug!("Git watcher event: {}", path.display());
                        let project = find_watched_project(&watched_projects, &path);
                        let Some(project) = project else { continue };

                        match last_event_time {
                            None => {
                                // First event after quiet period: immediate.
                                pending_paths.push(project);
                                last_event_time = Some(Instant::now());
                            }
                            Some(last) if last.elapsed() > DEBOUNCE => {
                                // Previous debounce window expired: immediate.
                                pending_paths.push(project);
                                last_event_time = Some(Instant::now());
                                buffered_paths.clear();
                            }
                            Some(_) => {
                                // Inside debounce window: buffer.
                                buffered_paths.insert(project);
                            }
                        }
                    }
                }
                Ok(Err(error)) => {
                    log::error!("Git watcher error: {}", error);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // Flush buffered events once the debounce window has passed.
        if let Some(last) = last_event_time {
            if last.elapsed() > DEBOUNCE && !buffered_paths.is_empty() {
                pending_paths.extend(buffered_paths.drain());
                last_event_time = None;
            }
        }

        // Recalculate status for pending paths.
        if !pending_paths.is_empty() {
            let paths: Vec<_> = pending_paths.drain(..).collect();
            let mut seen = HashSet::new();
            for path in paths {
                if !seen.insert(path.clone()) {
                    continue;
                }
                let status = check_status(&path);
                log::debug!(
                    "Recalculated Git status for {}: {:?}",
                    path.display(),
                    status
                );
                if let Ok(mut c) = cache.lock() {
                    let changed = c.get(&path).map(|e| e.status) != Some(status);
                    c.insert(
                        path,
                        GitStatusEntry {
                            status,
                            updated_at: Instant::now(),
                        },
                    );
                    if changed {
                        if let Some(ctx) = &repaint_context {
                            ctx.request_repaint();
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn find_watched_project(projects: &[PathBuf], event_path: &Path) -> Option<PathBuf> {
    // Prefer the longest matching prefix so nested projects are handled correctly.
    projects
        .iter()
        .filter(|p| event_path.starts_with(p))
        .max_by_key(|p| p.as_os_str().len())
        .cloned()
}

fn update_watches(watcher: &mut notify_debouncer_mini::notify::RecommendedWatcher, paths: &[PathBuf]) {
    for path in paths {
        if !path.exists() {
            continue;
        }

        let to_watch: Vec<PathBuf> = {
            let git_path = path.join(".git");
            if git_path.exists() {
                vec![path.to_path_buf(), git_path]
            } else {
                vec![path.to_path_buf()]
            }
        };

        for watch_path in to_watch {
            if let Err(e) = watcher.watch(watch_path.as_path(), RecursiveMode::Recursive) {
                log::debug!("Failed to watch {}: {}", watch_path.display(), e);
            }
        }
    }
}

fn clear_watches(watcher: &mut notify_debouncer_mini::notify::RecommendedWatcher, paths: &[PathBuf]) {
    for path in paths {
        let _ = watcher.unwatch(path);
        let _ = watcher.unwatch(&path.join(".git"));
    }
}

fn check_status(path: &Path) -> GitStatus {
    let repo = match Repository::open(path) {
        Ok(r) => r,
        Err(_) => return GitStatus::NotARepo,
    };

    let statuses = match repo.statuses(None) {
        Ok(s) => s,
        Err(_) => return GitStatus::Error,
    };

    let mut dirty = false;
    let mut conflicts = false;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_conflicted() {
            conflicts = true;
        } else if !status.is_ignored() {
            dirty = true;
        }
    }

    if conflicts {
        return GitStatus::Conflicts;
    }
    if dirty {
        return GitStatus::Dirty;
    }

    // Working tree is clean. Check whether the local branch diverged from upstream.
    check_divergence(&repo).unwrap_or(GitStatus::Clean)
}

fn check_divergence(repo: &Repository) -> Result<GitStatus, git2::Error> {
    let head = repo.head()?;
    let local_ref = head.resolve()?;
    let local_oid = local_ref.target().ok_or_else(|| {
        git2::Error::from_str("HEAD does not point to a valid object")
    })?;

    let branch = git2::Branch::wrap(head);
    let upstream = branch.upstream()?;
    let upstream_ref = upstream.get();
    let upstream_oid = upstream_ref.target().ok_or_else(|| {
        git2::Error::from_str("Upstream does not point to a valid object")
    })?;

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;

    if ahead > 0 || behind > 0 {
        Ok(GitStatus::Diverged)
    } else {
        Ok(GitStatus::Clean)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("git command failed; is git installed?");
        assert!(status.success(), "git {:?} failed in {}", args, dir.display());
    }

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("yaaa-git-test-{}", std::process::id()))
    }

    #[test]
    fn test_not_a_repo() {
        let dir = temp_dir().join("not-a-repo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(check_status(&dir), GitStatus::NotARepo);
    }

    #[test]
    fn test_clean_repo() {
        let dir = temp_dir().join("clean-repo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        run_git(&dir, &["init", "--quiet"]);
        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        run_git(&dir, &["config", "user.email", "test@test.com"]);
        run_git(&dir, &["config", "user.name", "Test"]);
        run_git(&dir, &["add", "."]);
        run_git(&dir, &["commit", "-m", "init", "--quiet"]);

        assert_eq!(check_status(&dir), GitStatus::Clean);
    }

    #[test]
    fn test_dirty_repo() {
        let dir = temp_dir().join("dirty-repo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        run_git(&dir, &["init", "--quiet"]);
        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        run_git(&dir, &["config", "user.email", "test@test.com"]);
        run_git(&dir, &["config", "user.name", "Test"]);
        run_git(&dir, &["add", "."]);
        run_git(&dir, &["commit", "-m", "init", "--quiet"]);

        std::fs::write(dir.join("file.txt"), "modified").unwrap();
        assert_eq!(check_status(&dir), GitStatus::Dirty);
    }

    #[test]
    fn test_diverged_repo() {
        let base = temp_dir().join("diverged-repo");
        let _ = std::fs::remove_dir_all(&base);
        let origin = base.join("origin");
        let local = base.join("local");
        std::fs::create_dir_all(&origin).unwrap();
        std::fs::create_dir_all(&local).unwrap();

        // Origin repo.
        run_git(&origin, &["init", "--quiet"]);
        run_git(&origin, &["config", "user.email", "test@test.com"]);
        run_git(&origin, &["config", "user.name", "Test"]);
        std::fs::write(origin.join("file.txt"), "origin").unwrap();
        run_git(&origin, &["add", "."]);
        run_git(&origin, &["commit", "-m", "init", "--quiet"]);

        // Clone origin.
        run_git(&local, &["clone", origin.to_str().unwrap(), ".", "--quiet"]);
        run_git(&local, &["config", "user.email", "test@test.com"]);
        run_git(&local, &["config", "user.name", "Test"]);

        // Commit in origin (so local is behind).
        std::fs::write(origin.join("file.txt"), "origin updated").unwrap();
        run_git(&origin, &["commit", "-am", "origin change", "--quiet"]);

        // Commit in local (so local is also ahead).
        std::fs::write(local.join("file.txt"), "local updated").unwrap();
        run_git(&local, &["commit", "-am", "local change", "--quiet"]);

        // Fetch so upstream reference is known locally.
        run_git(&local, &["fetch", "--quiet"]);

        assert_eq!(check_status(&local), GitStatus::Diverged);
    }


    #[test]
    fn test_watcher_updates_status() {
        let dir = temp_dir().join("watcher-repo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        run_git(&dir, &["init", "--quiet"]);
        run_git(&dir, &["config", "user.email", "test@test.com"]);
        run_git(&dir, &["config", "user.name", "Test"]);
        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        run_git(&dir, &["add", "."]);
        run_git(&dir, &["commit", "-m", "init", "--quiet"]);

        let service = GitService::new(true, None);
        service.set_paths(vec![dir.clone()]);

        wait_for_status(&service, &dir, GitStatus::Clean, Duration::from_secs(5));

        // Modify a tracked file -> Dirty.
        std::fs::write(dir.join("file.txt"), "dirty").unwrap();
        wait_for_status(&service, &dir, GitStatus::Dirty, Duration::from_secs(5));

        // Stash the change -> Clean again.
        run_git(&dir, &["stash", "--quiet"]);
        wait_for_status(&service, &dir, GitStatus::Clean, Duration::from_secs(5));
    }

    fn wait_for_status(
        service: &GitService,
        path: &Path,
        expected: GitStatus,
        timeout: Duration,
    ) {
        let start = Instant::now();
        loop {
            if let Some(status) = service.status_for(path) {
                if status == expected {
                    return;
                }
            }
            if start.elapsed() > timeout {
                panic!(
                    "Timed out waiting for {:?} for {}, last status: {:?}",
                    expected,
                    path.display(),
                    service.status_for(path)
                );
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

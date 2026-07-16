use std::path::Path;
use std::time::{Duration, Instant};

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

/// Detect Git status for the directory at `path`.
/// Returns `None` if `path` is not inside a Git repository or `git` is unavailable.
pub fn get_git_status(path: &Path) -> Option<GitStatus> {
    if !is_git_repo(path) {
        return None;
    }

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "-b", "--ignore-submodules"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_git_status(&String::from_utf8_lossy(&output.stdout))
}

fn is_git_repo(path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Parse the output of `git status --porcelain=v1 -b`.
/// Exposed so the sync-state logic can be unit-tested without running git.
pub fn parse_git_status(output: &str) -> Option<GitStatus> {
    let mut lines = output.lines();
    let branch_line = lines.next()?;
    if !branch_line.starts_with("## ") {
        return None;
    }

    let (branch, ahead, behind) = parse_branch_line(branch_line);

    let mut conflicts = false;
    let mut dirty = false;
    for line in lines {
        if line.len() < 2 {
            continue;
        }
        let status = &line[..2];
        if is_conflict_status(status) {
            conflicts = true;
        } else {
            // Any other porcelain line means the working tree or index changed.
            dirty = true;
        }
    }

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

fn parse_branch_line(line: &str) -> (Option<String>, usize, usize) {
    let rest = &line[3..];

    // Detached HEAD, e.g. "## HEAD (no branch)"
    if rest.starts_with("HEAD (") {
        return (None, 0, 0);
    }

    // Branch with upstream, e.g. "main...origin/main [ahead 2, behind 1]"
    if let Some((branch, upstream_with_status)) = rest.split_once("...") {
        let branch = Some(branch.to_string());
        let upstream = upstream_with_status.split_whitespace().next().unwrap_or("");
        let status_text = upstream_with_status.strip_prefix(upstream).unwrap_or("");
        let (ahead, behind) = parse_ahead_behind(status_text);
        return (branch, ahead, behind);
    }

    // Branch without upstream, e.g. "## feature/no-upstream"
    let branch = rest.split_whitespace().next().map(|s| s.to_string());
    (branch, 0, 0)
}

fn parse_ahead_behind(text: &str) -> (usize, usize) {
    let Some(start) = text.find('[') else {
        return (0, 0);
    };
    let Some(end) = text.find(']') else {
        return (0, 0);
    };
    let content = &text[start + 1..end];

    let mut ahead = 0;
    let mut behind = 0;

    for part in content.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.trim().parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.trim().parse().unwrap_or(0);
        }
    }

    (ahead, behind)
}

/// Conflict codes reported by `git status --porcelain=v1` for unmerged entries.
fn is_conflict_status(status: &str) -> bool {
    matches!(
        status,
        "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU"
    )
}

/// Simple time-based cache for Git statuses keyed by repository path.
#[derive(Debug, Default)]
pub struct GitStatusCache {
    entries: std::collections::HashMap<std::path::PathBuf, (GitStatus, Instant)>,
    ttl: Duration,
}

impl GitStatusCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            ttl,
        }
    }

    /// Return the cached status for `path` if it exists and has not expired.
    pub fn get(&self, path: &Path) -> Option<&GitStatus> {
        self.entries
            .get(path)
            .filter(|(_, fetched)| fetched.elapsed() < self.ttl)
            .map(|(status, _)| status)
    }

    /// Force a refresh of `path` and store the result in the cache.
    pub fn refresh(&mut self, path: &Path) -> Option<&GitStatus> {
        let status = get_git_status(path);
        if let Some(status) = status {
            self.entries.insert(path.to_path_buf(), (status, Instant::now()));
            self.entries.get(path).map(|(s, _)| s)
        } else {
            self.entries.remove(path);
            None
        }
    }

    /// Return a cached status, refreshing it only when missing or expired.
    pub fn get_or_refresh(&mut self, path: &Path) -> Option<&GitStatus> {
        if self.get(path).is_some() {
            return self.get(path);
        }
        self.refresh(path)
    }

    /// Remove stale entries for paths that are no longer tracked.
    pub fn retain<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&Path) -> bool,
    {
        self.entries.retain(|path, _| predicate(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_clean_with_upstream() {
        let output = "## main...origin/main\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, Some("main".to_string()));
        assert_eq!(status.sync_status, GitSyncStatus::Clean);
    }

    #[test]
    fn test_parse_needs_push() {
        let output = "## main...origin/main [ahead 3]\n M file.txt\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, Some("main".to_string()));
        assert_eq!(status.sync_status, GitSyncStatus::NeedsPush);
    }

    #[test]
    fn test_parse_needs_pull() {
        let output = "## main...origin/main [behind 5]\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.sync_status, GitSyncStatus::NeedsPull);
    }

    #[test]
    fn test_parse_diverged() {
        let output = "## main...origin/main [ahead 2, behind 4]\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.sync_status, GitSyncStatus::Diverged);
    }

    #[test]
    fn test_parse_conflicts_take_precedence() {
        // Even though the branch is ahead, conflicts must be reported.
        let output = "## main...origin/main [ahead 1]\nUU conflict.txt\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.sync_status, GitSyncStatus::Conflicts);
    }

    #[test]
    fn test_parse_various_conflict_codes() {
        for code in ["UU", "AU", "UD", "UA", "DU", "AA", "DD"] {
            let output = format!("## main...origin/main\n{} file.txt\n", code);
            let status = parse_git_status(&output).unwrap();
            assert_eq!(
                status.sync_status,
                GitSyncStatus::Conflicts,
                "code {} should be treated as a conflict",
                code
            );
        }
    }

    #[test]
    fn test_parse_no_upstream() {
        let output = "## feature/no-upstream\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, Some("feature/no-upstream".to_string()));
        assert_eq!(status.sync_status, GitSyncStatus::Clean);
    }

    #[test]
    fn test_parse_dirty() {
        let output = "## main...origin/main\n M file.txt\n?? untracked.txt\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, Some("main".to_string()));
        assert_eq!(status.sync_status, GitSyncStatus::Dirty);
    }

    #[test]
    fn test_parse_detached_head() {
        let output = "## HEAD (no branch)\n";
        let status = parse_git_status(output).unwrap();
        assert_eq!(status.branch, None);
        assert_eq!(status.sync_status, GitSyncStatus::Clean);
    }

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

    #[test]
    fn test_cache_refresh_and_ttl() {
        let mut cache = GitStatusCache::new(Duration::from_secs(60));
        // Empty cache returns nothing.
        assert!(cache.get(Path::new("/nonexistent")).is_none());

        // Parsing fixtures still works through the cache layer.
        let fixture = "## main...origin/main [ahead 2]\n";
        let status = parse_git_status(fixture).unwrap();
        cache
            .entries
            .insert(PathBuf::from("/fixture"), (status, Instant::now()));
        assert!(cache.get(Path::new("/fixture")).is_some());
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
    fn test_get_git_status_on_real_repo() {
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

        let status = get_git_status(dir).expect("should detect git repo");
        assert_eq!(status.branch, Some("master".to_string()));
        assert_eq!(status.sync_status, GitSyncStatus::Clean);
    }

    #[test]
    fn test_get_git_status_conflicts() {
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

        let git_status = get_git_status(dir).expect("should detect git repo");
        assert_eq!(git_status.sync_status, GitSyncStatus::Conflicts);
    }
}

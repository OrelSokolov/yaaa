use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use sysinfo::{
    MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System,
};

pub struct MemoryInfo {
    pub percent: f32,
}

pub struct SystemMonitor {
    system: System,
    last_memory_refresh: Instant,
    last_process_refresh: Instant,
    current: MemoryInfo,
    /// Parent -> children map, rebuilt lazily after each process refresh.
    children_cache: Option<HashMap<Pid, Vec<Pid>>>,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_memory(MemoryRefreshKind::everything())
                .with_processes(ProcessRefreshKind::everything()),
        );
        let current = Self::read_memory(&system);
        Self {
            system,
            last_memory_refresh: Instant::now(),
            last_process_refresh: Instant::now(),
            current,
            children_cache: None,
        }
    }

    /// Global system memory. Refreshed at most once per second.
    pub fn memory(&mut self) -> &MemoryInfo {
        let now = Instant::now();
        if now.duration_since(self.last_memory_refresh) >= Duration::from_secs(1) {
            self.system.refresh_memory();
            self.current = Self::read_memory(&self.system);
            self.last_memory_refresh = now;
        }
        &self.current
    }

    /// Resident memory of a single process in KB.
    #[allow(dead_code)]
    pub fn process_memory_kb(&mut self, pid: u32) -> u64 {
        self.refresh_processes_if_needed();
        self.system
            .process(Pid::from_u32(pid))
            .map(|p| p.memory() / 1024)
            .unwrap_or(0)
    }

    /// Resident memory of a process and all its descendants in KB.
    ///
    /// This is a better approximation for "how much RAM this tab uses" because
    /// a shell or agent process usually spawns child processes (compilers,
    /// servers, AI models, etc.).
    pub fn process_tree_memory_kb(&mut self, root_pid: u32) -> u64 {
        self.refresh_processes_if_needed();
        let root = Pid::from_u32(root_pid);

        // Build a parent -> children map from the full process list. sysinfo
        // exposes `tasks()` only on Linux, so we walk parent links ourselves to
        // make this work on macOS and Windows too. The map is cached until the
        // next process refresh, so multiple tab lookups per frame are cheap.
        if self.children_cache.is_none() {
            let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
            for (pid, process) in self.system.processes() {
                // On Linux sysinfo enumerates individual threads as separate
                // processes under /proc/[PID]/task. They share the same address
                // space as the main process, so counting them would multiply
                // the reported RSS by the number of threads.
                if process.thread_kind().is_some() {
                    continue;
                }
                if let Some(parent) = process.parent() {
                    children.entry(parent).or_default().push(*pid);
                }
            }
            self.children_cache = Some(children);
        }
        let children = self.children_cache.as_ref().unwrap();

        let mut total_bytes = 0u64;
        let mut to_visit = vec![root];
        let mut visited = HashSet::new();

        while let Some(pid) = to_visit.pop() {
            if !visited.insert(pid) {
                continue;
            }
            if let Some(p) = self.system.process(pid) {
                // Same thread-guard as above: never count thread entries.
                if p.thread_kind().is_none() {
                    total_bytes += p.memory();
                }
            }
            if let Some(kids) = children.get(&pid) {
                for child in kids {
                    to_visit.push(*child);
                }
            }
        }

        total_bytes / 1024
    }

    fn read_memory(system: &System) -> MemoryInfo {
        let total_kb = system.total_memory() / 1024;
        let available_kb = system.available_memory() / 1024;
        let used_kb = total_kb.saturating_sub(available_kb);
        let percent = if total_kb > 0 {
            (used_kb as f64 / total_kb as f64 * 100.0) as f32
        } else {
            0.0
        };
        MemoryInfo { percent }
    }

    fn refresh_processes_if_needed(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_process_refresh) >= Duration::from_secs(2) {
            self.system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
            self.children_cache = None;
            self.last_process_refresh = now;
        }
    }
}

pub fn format_kb(kb: u64) -> String {
    if kb >= 1024 * 1024 {
        format!("{:.1} GB", kb as f64 / (1024.0 * 1024.0))
    } else if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
    } else {
        format!("{} KB", kb)
    }
}

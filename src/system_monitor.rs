use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use sysinfo::{
    MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System,
};

/// How often the background thread re-enumerates all processes.
const PROCESS_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

pub struct MemoryInfo {
    pub percent: f32,
}

/// Process table snapshot shared between the refresh thread and the UI.
#[derive(Default)]
struct ProcessSnapshot {
    /// Non-thread process entries: pid -> (parent, resident bytes).
    processes: HashMap<Pid, (Option<Pid>, u64)>,
    /// Parent -> children map, precomputed by the refresh thread.
    children: HashMap<Pid, Vec<Pid>>,
}

/// Builds a snapshot from a freshly refreshed process list.
fn build_snapshot(system: &System) -> ProcessSnapshot {
    let mut processes = HashMap::new();
    let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
    for (pid, process) in system.processes() {
        // On Linux sysinfo enumerates individual threads as separate
        // processes under /proc/[PID]/task. They share the same address
        // space as the main process, so counting them would multiply the
        // reported RSS by the number of threads.
        if process.thread_kind().is_some() {
            continue;
        }
        let parent = process.parent();
        processes.insert(*pid, (parent, process.memory()));
        if let Some(parent) = parent {
            children.entry(parent).or_default().push(*pid);
        }
    }
    ProcessSnapshot { processes, children }
}

pub struct SystemMonitor {
    /// Global memory only; the process table lives on the refresh thread.
    system: System,
    last_memory_refresh: Instant,
    current: MemoryInfo,
    snapshot: Arc<Mutex<ProcessSnapshot>>,
    shutdown: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
        );
        let current = Self::read_memory(&system);

        let snapshot = Arc::new(Mutex::new(ProcessSnapshot::default()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let snapshot_clone = Arc::clone(&snapshot);
        let shutdown_clone = Arc::clone(&shutdown);
        let handle = thread::Builder::new()
            .name("system-monitor".into())
            .spawn(move || {
                // Enumerating all processes can take tens of milliseconds on
                // busy systems; doing that in the UI thread froze frames, so
                // the UI only ever reads the latest snapshot from here.
                let mut proc_system = System::new_with_specifics(
                    RefreshKind::nothing()
                        .with_processes(ProcessRefreshKind::nothing().with_memory()),
                );
                while !shutdown_clone.load(Ordering::Relaxed) {
                    proc_system.refresh_processes_specifics(
                        ProcessesToUpdate::All,
                        true,
                        ProcessRefreshKind::nothing().with_memory(),
                    );
                    let next = build_snapshot(&proc_system);
                    *lock(&snapshot_clone) = next;
                    // Sleep in small increments so shutdown is responsive.
                    let mut elapsed = Duration::ZERO;
                    let step = Duration::from_millis(200);
                    while elapsed < PROCESS_REFRESH_INTERVAL {
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
            system,
            last_memory_refresh: Instant::now(),
            current,
            snapshot,
            shutdown,
            _thread: handle,
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

    /// Resident memory of a process and all its descendants in KB.
    ///
    /// This is a better approximation for "how much RAM this tab uses" because
    /// a shell or agent process usually spawns child processes (compilers,
    /// servers, AI models, etc.). Reads the snapshot refreshed by the
    /// background thread; never blocks on process enumeration.
    pub fn process_tree_memory_kb(&self, root_pid: u32) -> u64 {
        let snapshot = lock(&self.snapshot);
        let root = Pid::from_u32(root_pid);

        let mut total_bytes = 0u64;
        let mut to_visit = vec![root];
        let mut visited = HashSet::new();

        while let Some(pid) = to_visit.pop() {
            if !visited.insert(pid) {
                continue;
            }
            // Thread entries were filtered out when the snapshot was built,
            // so they are never counted here.
            if let Some((_, bytes)) = snapshot.processes.get(&pid) {
                total_bytes += *bytes;
            }
            if let Some(kids) = snapshot.children.get(&pid) {
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
}

/// Lock a std Mutex, recovering from poisoning instead of panicking: the
/// snapshot under the lock is swapped in whole, so a guard from a panicked
/// thread still holds consistent data.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Drop for SystemMonitor {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self._thread.take() {
            let _ = handle.join();
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

# Реализация OpenCode Redis API для статусов агентов

## Обзор
Добавить поддержку чтения статусов агентов из Redis и отображения их в UI.

## Пошаговое руководство

### Шаг 1: Добавить зависимости в Cargo.toml

```toml
[dependencies]
# ... существующие зависимости
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
tokio = { version = "1", features = ["full"] }
ulid = { version = "1", features = ["serde"] }
```

### Шаг 2: Создать модуль redis_status.rs

Создать файл `src/redis_status.rs`:

```rust
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Busy,
    Done,
}

impl AgentStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" | "busy" | "waiting_for_user_input" => AgentStatus::Busy,
            "done" => AgentStatus::Done,
            _ => AgentStatus::Idle,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "IDLE",
            AgentStatus::Busy => "BUSY",
            AgentStatus::Done => "✅",
        }
    }
}

pub struct AgentStatusTracker {
    client: Option<redis::aio::ConnectionManager>,
    key_prefix: String,
    pub agent_statuses: Arc<Mutex<HashMap<String, AgentStatus>>>,
    running: Arc<Mutex<bool>>,
}

impl AgentStatusTracker {
    pub async fn new(redis_host: String, redis_port: u16, key_prefix: String) -> Result<Self, redis::RedisError> {
        info!("AgentStatusTracker::new() called with host={}, port={}, prefix={}", redis_host, redis_port, key_prefix);
        let client_url = format!("redis://{}:{}", redis_host, redis_port);
        let client = redis::Client::open(client_url)?;
        let conn = client.get_connection_manager().await?;

        let tracker = Self {
            client: Some(conn),
            key_prefix,
            agent_statuses: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(true)),
        };

        tracker.start_update_thread();
        Ok(tracker)
    }

    pub fn new_disabled() -> Self {
        info!("AgentStatusTracker::new_disabled() called");
        Self {
            client: None,
            key_prefix: String::new(),
            agent_statuses: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    fn start_update_thread(&self) {
        let client = self.client.clone();
        let key_prefix = self.key_prefix.clone();
        let agent_statuses = self.agent_statuses.clone();
        let running = self.running.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                info!("Redis update thread started");
                while *running.lock().unwrap() {
                    if let Some(ref conn) = client {
                        let pattern = format!("{}*", key_prefix);
                        let mut conn_clone = conn.clone();
                        match conn_clone.keys::<String>(&pattern).await {
                            Ok(keys) => {
                                let mut new_statuses = HashMap::new();

                                for key in keys {
                                    if let Some(status_id) = key.strip_prefix(&key_prefix) {
                                        let mut conn_clone = conn.clone();
                                        match conn_clone.hgetall::<_, HashMap<String, String>>(&key).await {
                                            Ok(fields) => {
                                                if let Some(status) = fields.get("status") {
                                                    let agent_status = AgentStatus::from_str(status);
                                                    new_statuses.insert(status_id.to_string(), agent_status);
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Redis error: {}", e);
                                            }
                                        }
                                    }
                                }

                                *agent_statuses.lock().unwrap() = new_statuses;
                            }
                            Err(e) => {
                                eprintln!("Redis error: {}", e);
                            }
                        }
                    }

                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                info!("Redis update thread stopped");
            });
        });
    }

    pub fn get_status(&self, status_id: &str) -> Option<AgentStatus> {
        self.agent_statuses.lock().unwrap().get(status_id).cloned()
    }

    pub fn is_enabled(&self) -> bool {
        self.client.is_some()
    }
}

impl Drop for AgentStatusTracker {
    fn drop(&mut self) {
        *self.running.lock().unwrap() = false;
    }
}
```

### Шаг 3: Добавить модуль в main.rs

```rust
mod app;
mod config;
mod constants;
mod hotkeys;
mod menu;
mod redis_status;
mod terminal;
mod ui;
```

### Шаг 4: Изменить TabInfo struct в terminal/manager.rs

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct TabInfo {
    pub id: u64,
    pub is_agent: bool,
    pub status_id: Option<String>,
}
```

### Шаг 5: Обновить TabManager struct

```rust
pub struct TabManager {
    // ... существующие поля
    pub opencode_redis_api: bool,
}

impl TabManager {
    pub fn new(
        // ... параметры
        opencode_redis_api: bool,
    ) -> Self {
        // ... инициализация
        Self {
            // ... поля
            opencode_redis_api,
        }
    }
}
```

### Шаг 6: Обновить конструктор Tab::new в terminal/tab.rs

```rust
pub fn new(
    // ... параметры
    status_id: &Option<String>,
    opencode_redis_api: bool,
) -> Self {
    // ... код инициализации
    
    // Добавить статус-id аргумент
    if is_agent && opencode_redis_api {
        if let Some(ref sid) = status_id {
            args.push("--status-id".to_string());
            args.push(sid.clone());
        }
    }
    
    // ... остальной код
}
```

### Шаг 7: Обновить add_tab_to_group в TabManager

```rust
pub fn add_tab_to_group(&mut self, group_id: u64, ctx: egui::Context, is_agent: bool) {
    // ... существующий код
    
    let status_id = if use_agent && self.opencode_redis_api {
        Some(ulid::Ulid::new().to_string())
    } else {
        None
    };
    
    let tab = Tab::new(
        ctx,
        self.command_sender.clone(),
        tab_id,
        group_path,
        shell_cmd,
        use_agent,
        !use_agent && self.run_as_login_shell,
        &status_id,
        self.opencode_redis_api,
    );
    
    // ... остальной код
}
```

### Шаг 8: Обновить load_groups в TabManager

```rust
for mut group in groups_data {
    manager.next_group_id = manager.next_group_id.max(group.id + 1);
    for tab_info in &mut group.tabs {
        manager.next_tab_id = manager.next_tab_id.max(tab_info.id + 1);
        
        let tab = Tab::new(
            cc.egui_ctx.clone(),
            manager.command_sender.clone(),
            tab_info.id,
            Some(group.path.clone()),
            shell_cmd,
            use_agent,
            !use_agent && manager.run_as_login_shell,
            &tab_info.status_id,
            manager.opencode_redis_api,
        );
        manager.tabs.insert(tab_info.id, tab);
    }
    manager.groups.insert(group.id, group);
}
```

### Шаг 9: Изменить Settings struct в config/settings.rs

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Settings {
    #[serde(default = "default_show_terminal_lines")]
    pub show_terminal_lines: bool,
    #[serde(default = "default_show_fps")]
    pub show_fps: bool,
    #[serde(default = "default_run_as_login_shell")]
    pub run_as_login_shell: bool,
    #[serde(default = "default_default_shell_cmd")]
    pub default_shell_cmd: String,
    #[serde(default = "default_default_agent_cmd")]
    pub default_agent_cmd: String,
    #[serde(default)]
    pub opencode_redis_api: bool,
}
```

### Шаг 10: Обновить WindowManager struct в ui/windows.rs

```rust
pub struct WindowManager {
    pub show_about: bool,
    pub show_hotkeys: bool,
    pub show_settings: bool,
    pub show_rename_group: bool,
    pub rename_group_id: Option<u64>,
    pub rename_group_name: String,
    pub editing_default_shell_cmd: String,
    pub editing_default_agent_cmd: String,
    pub saved_default_shell_cmd: String,
    pub saved_default_agent_cmd: String,
    pub editing_run_as_login_shell: bool,
    pub saved_run_as_login_shell: bool,
    pub editing_opencode_redis_api: bool,
    pub saved_opencode_redis_api: bool,
    pub was_settings_open: bool,
}

impl WindowManager {
    pub fn new(
        default_shell_cmd: String,
        default_agent_cmd: String,
        run_as_login_shell: bool,
        opencode_redis_api: bool,
    ) -> Self {
        let editing_default_shell_cmd = default_shell_cmd.clone();
        let editing_default_agent_cmd = default_agent_cmd.clone();
        let saved_default_shell_cmd = editing_default_shell_cmd.clone();
        let saved_default_agent_cmd = editing_default_agent_cmd.clone();
        let editing_run_as_login_shell = run_as_login_shell;
        let saved_run_as_login_shell = run_as_login_shell;
        let editing_opencode_redis_api = opencode_redis_api;
        let saved_opencode_redis_api = opencode_redis_api;

        Self {
            show_about: false,
            show_hotkeys: false,
            show_settings: false,
            show_rename_group: false,
            rename_group_id: None,
            rename_group_name: String::new(),
            editing_default_shell_cmd,
            editing_default_agent_cmd,
            saved_default_shell_cmd,
            saved_default_agent_cmd,
            editing_run_as_login_shell,
            saved_run_as_login_shell,
            editing_opencode_redis_api,
            saved_opencode_redis_api,
            was_settings_open: false,
        }
    }
}
```

### Шаг 11: Добавить чекбокс в UI (ui/windows.rs)

В функции show_settings добавить:
```rust
ui.add_space(15.0);
ui.checkbox(&mut self.editing_opencode_redis_api, "OpenCode Redis Api");
ui.add_space(15.0);
```

### Шаг 12: Обновить WindowActions struct

```rust
pub struct WindowActions {
    pub rename_group: Option<(u64, String)>,
    pub default_shell_cmd: Option<String>,
    pub default_agent_cmd: Option<String>,
    pub run_as_login_shell: Option<bool>,
    pub opencode_redis_api: Option<bool>,
    pub should_save_groups: bool,
    pub should_save_settings: bool,
}
```

### Шаг 13: Обновить логику сохранения/отмены настроек

В обработке window_actions:
```rust
if let Some(opencode_redis_api) = actions.opencode_redis_api {
    self.editing_opencode_redis_api = opencode_redis_api;
}
```

### Шаг 14: Интеграция в App (app.rs)

```rust
use crate::redis_status::{AgentStatus, AgentStatusTracker};

pub struct App {
    // ... существующие поля
    status_tracker: AgentStatusTracker,
    pub opencode_redis_api: bool,
    tokio_runtime: Option<tokio::runtime::Runtime>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        let opencode_redis_api = settings.opencode_redis_api;
        let tokio_runtime = tokio::runtime::Runtime::new().ok();

        let status_tracker = if opencode_redis_api {
            if let Some(ref rt) = tokio_runtime {
                rt.block_on(async {
                    AgentStatusTracker::new(
                        "localhost".to_string(),
                        6379,
                        "opencode-status-".to_string(),
                    ).await
                    .unwrap_or_else(|_| AgentStatusTracker::new_disabled())
                })
            } else {
                AgentStatusTracker::new_disabled()
            }
        } else {
            AgentStatusTracker::new_disabled()
        };

        let tab_manager = TabManager::new(
            command_sender_clone,
            cc,
            settings.default_shell_cmd.clone(),
            settings.default_agent_cmd.clone(),
            settings.run_as_login_shell,
            settings.opencode_redis_api,
        );

        // ... инициализация остальных полей

        Self {
            // ... остальные поля
            status_tracker,
            opencode_redis_api,
            tokio_runtime,
        }
    }

    fn handle_window_actions(&mut self, actions: WindowActions) {
        // ... существующая логика

        if let Some(opencode_redis_api) = actions.opencode_redis_api {
            self.opencode_redis_api = opencode_redis_api;
            self.tab_manager.opencode_redis_api = opencode_redis_api;
            
            if opencode_redis_api && self.status_tracker.is_enabled() {
                // Уже включено
            } else if opencode_redis_api {
                self.tokio_runtime = tokio::runtime::Runtime::new().ok();
                if let Some(ref rt) = self.tokio_runtime {
                    match rt.block_on(async {
                        AgentStatusTracker::new(
                            "localhost".to_string(),
                            6379,
                            "opencode-status-".to_string(),
                        ).await
                    }) {
                        Ok(tracker) => self.status_tracker = tracker,
                        Err(_) => self.status_tracker = AgentStatusTracker::new_disabled(),
                    }
                }
            } else {
                self.status_tracker = AgentStatusTracker::new_disabled();
                self.tokio_runtime = None;
            }
        }

        // ... остальная логика
    }

    pub fn get_agent_status(&self, status_id: &str) -> Option<AgentStatus> {
        self.status_tracker.get_status(status_id)
    }
}
```

### Шаг 15: Обновить UI панели (ui/panels.rs)

```rust
use crate::redis_status::AgentStatus;

pub fn show_left_panel(
    ctx: &egui::Context,
    tab_manager: &TabManager,
    window_manager: &mut super::windows::WindowManager,
    agent_statuses: &std::collections::HashMap<String, AgentStatus>,
) -> PanelActions {
    // ... существующий код

    let agent_statuses_map: std::collections::HashMap<String, AgentStatus> = tab_manager
        .groups
        .values()
        .flat_map(|g| &g.tabs)
        .filter_map(|t| {
            if t.is_agent {
                if let Some(ref sid) = t.status_id {
                    status.get(sid).map(|s| (sid.clone(), s))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let panel_actions = show_left_panel_inner(
        ctx,
        &tab_manager.groups,
        window_manager,
        &agent_statuses_map,
    );

    panel_actions
}
```

### Шаг 16: Обновить отображение статуса агента

В цикле отрисовки вкладок:
```rust
let mut display_name = tab_name.clone();
if tab_info.is_agent {
    if let Some(status_id) = &tab_info.status_id {
        if let Some(status) = agent_statuses.get(status_id) {
            match status {
                AgentStatus::Busy => {
                    ui.horizontal(|ui| {
                        ui.label(tab_name);
                        ui.label("⟳");
                    });
                }
                _ => {
                    display_name = format!("{} [{}]", tab_name, status.as_str());
                }
            }
        }
    }
}
```

## Резюме

1. **Redis подключение**: `redis://localhost:6379`
2. **Паттерн ключей**: `opencode-status-*`
3. **Извлечение status_id**: из имени ключа через `strip_prefix("opencode-status-")`
4. **Агент запускается**: с `--status-id <ulid>`
5. **Поток обновления**: отдельный поток, опрашивает каждую секунду, обновляет глобальный HashMap
6. **UI читает**: из глобального HashMap напрямую через `get_status(status_id)`
7. **Статусы**: IDLE, BUSY (со спиннером), ✅ для DONE

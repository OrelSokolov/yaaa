# План рефакторинга

> Статус: **выполнен полностью** (итерации 1–8, ветка `refactoring`).
> Коммиты: `refactor(iter1)` … `refactor(iter8)`. 56 тестов зелёные
> (было 19 на старте).

Цель: убрать неявные контракты и нарушение SRP в `app.rs` / `ui/windows.rs` /
`terminal/manager.rs`. Каждая итерация — отдельный коммит, компиляция + `cargo
test` на каждом шаге. Если итерация затрагивает код без тестов — сначала
пишутся тесты на текущее поведение (характеризационные), затем рефакторинг.

Ветка: `refactoring`.

## Диагноз (кратко)

1. **Неявная сущность «конфиг запуска терминала»** —
   `{default_shell_cmd, agents, run_as_login_shell, preload_tabs}` живёт в 4
   копиях: `Settings`, `TabManager`, `WindowManager.editing_*`,
   `WindowManager.saved_*`. `App::save_settings` читает `editing_*` как
   источник истины — инвариант «editing == saved == применено» поддерживается
   вручную в 3+ местах (например app.rs: тумблер git status мутирует 4 поля).
2. **Неявный контракт «сам вызови save_groups»** — после почти каждой мутации
   `TabManager` вызывающий обязан помнить `save_groups()` (10+ вызовов в
   app.rs).
3. **SRP**: `App` — composition root + event pump + персистентность + рендер
   меню (~220 строк в `ui()`) + тема + layout-трекер + планировщик repaint.
   `WindowManager` — 8 окон + дубликат всех настроек. `TabManager` — домен +
   персистентность + preload + форматирование имён (view-логика, хардкод
   «Агент {}»).

## Итерации

### Итерация 1 — характеризационные тесты ✅
Тестового покрытия нет для `config/settings.rs`, `theme.rs`, персистентности
сессий (`TabManager::load_groups/save_groups`), `TabGroup::name_from_path`.
Пишем тесты на текущее поведение (round-trip сериализация, дефолты,
наследование legacy-полей).

Попутно найден и исправлен баг: derive-`Default` у `Settings` давал нули
(сайдбар/FPS/агенты выключены на первом запуске) вместо serde-дефолтов.
Зафиксированы (не исправлены) квирки: легаси-миграция `default_agent_cmd`
не срабатывает без явного пустого `agents` в файле; hex-хелперы цвета
двойно премультиплицируют альфу (не проявляется на непрозрачных цветах).

### Итерация 2 — меню-бар → `src/ui/menu_bar.rs` ✅
~220 строк инлайнового UI из `App::ui` вынесены в `show_menu_bar` c
`MenuBarView` (только чтение) и `MenuActions` (command-паттерн).
`open_paths` считается один раз и переиспользуется проект-файндером.

### Итерация 3 — тема → `theme.rs` ✅
`setup_visuals` и `AppTheme::visuals` переехали из `app.rs`; `visuals()`
под тестом.

### Итерация 4 — `TerminalLayoutTracker` ✅
Дебаунс записи layout/метрик шрифта вынесен в `terminal/layout.rs`;
детекция изменения и дебаунс — под юнит-тестами.

### Итерация 5 — сущность `TerminalLaunchConfig` ✅
`{default_shell_cmd, agents, run_as_login_shell, preload_tabs}` — один
владелец (`App::launch_config`); `Settings` — сериализуемая форма
(`from_settings`/`apply_to_settings`, round-trip тест); `TabManager`
синхронизируется одним `update_launch_config` вместо трёх `set_*`;
`save_settings` читает конфиг App, а не черновик окна.

### Итерация 6 — убраны дубли `editing_*`/`saved_*` ✅
Черновик сеется при открытии окна (`begin_*_edit`), Cancel просто закрывает.
`saved_*`-зеркала и ручной инвариант удалены. Новые сигналы
`theme_discarded`/`fonts_discarded` откатывают живое превью (заодно
сбрасывается прозрачность вьюпорта, которая раньше «залипала»).

### Итерация 7 — инкапсуляция `TabManager` ✅
- `groups` → private; наружу `iter_groups()`, `group()`, `has_groups()`.
- Контракт «сам вызови save_groups» мёртв: мутации ставят dirty, App делает
  один flush за кадр (`save_groups_if_dirty`); `clear()` при выходе
  сбрасывает dirty, чтобы не затирать сохранённую сессию.
- `display_name` больше не персистится; лейблы сайдбара считаются на лету
  (`tab_display_name` в panels.rs) — переименование агента сразу видно.
- `WindowActions::should_save_groups` удалён.

### Итерация 8 — разбиение `ui/windows.rs` ✅
`ui/windows/`: mod.rs (структура, сеяние черновиков, WindowActions) + по
файлу на окно (about, rename, settings, agents, theme, fonts).

## Правила
- Одна итерация = один коммит (только файлы итерации).
- `cargo build && cargo test` зелёные перед коммитом.
- Поведение пользователя не меняется (чистый рефакторинг); изменения видимого
  поведения — только если это сама цель итерации (и указана в коммите).

---

# План рефакторинга, раунд 2

> Статус: **выполнен полностью** (итерации 1–7).
> Проверки: `cargo build` и `cargo test` зелёные на каждом шаге (72 теста,
> было 56), clippy без предупреждений в собственном коде.
> Отчёт по код-ревью: паники в рантайме, неатомарная запись настроек,
> блокировка UI-потока, мёртвый код, дубли в `app.rs`, отсутствие тестов на
> чистые функции.

## Итерации

### Итерация 1 — убрать паники в рантайме
- `terminal/tab.rs`: вместо `panic!("All fallback shells failed")` — таб
  возвращается с признаком ошибки (терминал показывает сообщение).
- `git_status.rs`: `lock().unwrap()` → устойчивость к отравленному локу
  (`unwrap_or_else(|e| e.into_inner())`).

### Итерация 2 — атомарная запись персистентности
- Общий хелпер `write_atomic(path, bytes)`: запись в `*.tmp` + `fs::rename`.
- Применить в `config/settings.rs`, `config/recent_projects.rs`,
  `terminal/manager.rs` (save_groups).
- Битый `settings.json`/сессия: перед перезаписью дефолтами сохранить
  повреждённый файл как `.bak`, записать warning в лог.

### Итерация 3 — убрать блокировки UI-потока
- `terminal/shell_env.rs`: кэшировать `build()` через `OnceLock` (окружение
  процесса не меняется между запусками PTY).
- `system_monitor.rs`: `refresh_processes_specifics` вынести в фоновый поток
  по образцу `GitStatusCache` (кэш + мьютекс + request/update).

### Итерация 4 — мёртвый код + clippy
- Удалить `command_exists`/`resolve_shell` (`tab.rs`), `process_memory_kb`
  (`system_monitor.rs`), `label()` (`git_status.rs`), если не найдётся
  применение.
- `cargo clippy --fix` + ручные правки (field_reassign_with_default и пр.).

### Итерация 5 — дедупликация `app.rs` + сигнатуры UI
- Метод `App::open_project` вместо двух копий блока «открыть проект /
  absent path → убрать из recent».
- Результаты `handle_keyboard` прогонять через тот же обработчик, что и
  panel actions (дубль добавления таба/агента).
- `show_left_panel`: 10 аргументов → view-структура по образцу `MenuBarView`.
- `format!("Агент {}")` → `agent_display_name(idx)`, единый язык лейблов.

### Итерация 6 — валидация ввода
- `default_shell_cmd`/агенты: `.trim()` при сохранении настроек.
- `--login` только для шеллов, которые его поддерживают (bash/zsh/sh/dash/ksh).
- `Escape`/`Enter` в окне настроек — только при фокусе окна, не глобально.
- Магические числа (repaint-интервалы, пороги памяти, лимит recent) →
  `constants.rs`.

### Итерация 7 — тесты на чистые функции
- `hotkeys.rs`, `Tab::detect_clear`, `agent_display_name`,
  `write_atomic` (round-trip + перезапись существующего файла).

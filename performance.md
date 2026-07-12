# Анализ производительности yaaa + egui_term

Узкие места отсортированы по степени критичности. Все оценки относятся к hot path (код, выполняемый каждый кадр рендера).

---

## КРИТИЧЕСКИЕ (hot path, каждый кадр)

### 1. `Grid::clone()` каждый кадр

**Файл:** `egui_term/src/backend/mod.rs:430`

```rust
self.last_content.grid = terminal.grid().clone();
```

Полное клонирование грида терминала (cols × lines × `sizeof(Cell)`) на **каждом кадре**. Для 80×50 — это ~4000 ячеек, с историей прокрутки — десятки тысяч. Это самое дорогое место во всём пайплайне.

### 2. Парсинг hex-строк цветов для каждой ячейки

**Файлы:** `egui_term/src/view.rs:287-288` + `egui_term/src/theme.rs:150,201`

```rust
let mut fg = self.theme.get_color(indexed.fg);
let mut bg = self.theme.get_color(indexed.bg);
```

`get_color()` для `Named` и `Indexed(<=15)` цветов вызывает `hex_to_color()`, который делает `u8::from_str_radix` по строке — **для каждой ячейки, каждый кадр**. Для 80×50 = 8000 парсов строк в кадр. Цвета нужно предвычислить один раз в `Color32` при создании темы и хранить в массиве/HashMap.

### 3. `build_terminal_theme()` каждый кадр

**Файлы:** `yaaa/src/ui/panels.rs:365` -> `yaaa/src/theme.rs:82-87`

```rust
let terminal_theme = theme.build_terminal_theme();
```

Создаёт `ColorPalette` (~30 аллокаций `String`), упаковывает в `Box`, и вызывает `get_ansi256_colors()` — строит HashMap на 256 записей. **Каждый кадр.** Нужно кэшировать и пересоздавать только при смене темы.

---

## ВЫСОКИЕ (hot path)

### 4. `BindingsLayout::new()` каждый кадр

**Файл:** `egui_term/src/view.rs:88`

```rust
bindings_layout: BindingsLayout::new(),
```

`TerminalView::new()` вызывается каждый кадр (это egui-виджет). Внутри `BindingsLayout::new()` создаётся ~150 биндингов через макрос с аллокациями. Нужно либо кэшировать через `ui.memory()`, либо сделать `Default` + переиспользование.

### 5. Клонирование всех events каждый кадр

**Файл:** `egui_term/src/view.rs:150`

```rust
let events = layout.ctx.input(|i| i.events.clone());
```

Клонирует всю очередь событий даже когда нет релевантных событий.

### 6. Клонирование всех групп каждый кадр

**Файл:** `yaaa/src/ui/panels.rs:48-52`

```rust
let groups_to_render: Vec<(u64, String, Vec<TabInfo>)> = ...
    .map(|(id, g)| (*id, g.name.clone(), g.tabs.clone()))
```

Клонирует все имена групп и векторы вкладок каждый кадр для сайдбара.

### 7. `url_regex.clone()` на каждое наведение мыши

**Файл:** `egui_term/src/backend/mod.rs:545`

```rust
self.regex_match_at(terminal, point, &mut self.url_regex.clone())
```

`RegexSearch` — тяжёлый объект. Клонируется при каждом `LinkAction::Hover` (при движении мыши с зажатым Cmd).

---

## СРЕДНИЕ

### 8. Линейный поиск по матчам для каждой ячейки

**Файл:** `egui_term/src/view.rs:277-280`

```rust
content.search_state.point_in_match(indexed.point).is_some()
// и
content.search_state.is_focused_match(indexed.point)
```

Когда поиск активен, для каждой ячейки делается `.position()` по всем матчам — O(cells × matches). Нужно построить `HashSet<Point>` при обновлении матчей.

### 9. `selectable_content()` при каждом показе контекстного меню

**Файл:** `yaaa/src/ui/panels.rs:386`

Итерирует весь грид и строит строку при каждом рендере меню, даже если выделение не изменилось.

### 10. `get_tab_name()` для каждой вкладки каждый кадр

**Файл:** `yaaa/src/ui/panels.rs:134`

Множественные lookups по BTreeMap + форматирование строк (`format!`) каждый кадр для каждой вкладки.

### 11. Рендер каждого символа отдельным `Shape::text`

**Файл:** `egui_term/src/view.rs:364-376`

Каждый символ — отдельный shape. Для текста можно батчить подряд идущие ячейки с одинаковым стилем в одну текстовую строку, сократив количество shapes в десятки раз.

---

## НИЗКИЕ

### 12. `font_measure()` каждый кадр

**Файл:** `egui_term/src/view.rs:138` (через `resize()`)

Запрос к системе шрифтов каждый кадр, хотя результат меняется только при смене шрифта.

### 13. `clear_color` с `log::debug!` каждый кадр

**Файл:** `yaaa/src/app.rs:309-311`

Макрос `log` проверяет уровень перед вычислением аргументов, но вызов `app_bg_with_opacity()` всё равно происходит каждый кадр (очень дёшево, но всё же).

### 14. `process_command` всегда берёт лок мьютекса

**Файл:** `egui_term/src/backend/mod.rs:343-344`

Даже для команд, не требующих доступа к терминалу (некоторые mouse reports).

---

## Рекомендуемый порядок исправлений

| # | Проблема | Ожидаемый эффект |
|---|----------|-----------------|
| 1 | Кэшировать `Grid` / избежать `clone()` (dirty-flag или double-buffer) | **Огромный** — убрать крупнейшую аллокацию |
| 2 | Предвычислить цвета темы в `[Color32; 256]` вместо парсинга строк | **Огромный** — убрать 8000 парсов/кадр |
| 3 | Кэшировать `TerminalTheme` (пересоздавать при смене темы) | **Большой** — убрать 256-entry HashMap + 30 аллокаций/кадр |
| 4 | Кэшировать `BindingsLayout` в `ui.memory()` | **Большой** — убрать ~150 аллокаций/кадр |
| 5 | `HashSet<Point>` для search matches | Средний — ускоряет поиск |
| 6 | Не клонировать events (фильтровать внутри closure) | Средний |
| 7 | Не клонировать groups в сайдбаре (borrow directly) | Средний |

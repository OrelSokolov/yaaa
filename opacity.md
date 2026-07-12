# План: Background Blur (матовое стекло за прозрачным окном)

## Цель
Добавить блюр рабочего стола позади прозрачного окна терминала — эффект
«матового стекла»/acrylic. Управляется отдельным чекбоксом «Background blur» в
настройках темы. Кроссплатформенно, с graceful degradation.

## Решения
- **Что размывать:** рабочий стол за окном (композиторный блюр).
- **Платформы:** все, с graceful degradation.
- **UI:** отдельный чекбокс «Background blur» в Theme Settings → Background.
- **Wayland:** пропускаем в v1 (полагаемся на авто-блюр композитора —
  Hyprland/KWin сами размывают прозрачные окна).
- **Blur ⇒ прозрачность:** включение чекбокса форсирует прозрачное окно даже
  при `opacity=100`.
- **Windows:** Acrylic (тинт = `app_bg`).

## Ключевой энаблер
Блюр — свойство окна/композитора, применяется один раз и переключается on/off.
Не относится к покадровой отрисовке egui. Механизм прозрачности окна уже есть
(`with_transparent`, `ViewportCommand::Transparent`, `clear_color` с альфой).
`eframe::Frame` реализует `HasWindowHandle + HasDisplayHandle` (epi.rs:735) —
ручку окна можно получить внутри `App::ui`.

## Изменения по файлам

### 1. `src/theme.rs`
- В `AppTheme`: `#[serde(default)] pub background_blur: bool` + `false` в
  `default()`.
- `pub fn wants_transparency(&self) -> bool { self.app_bg_opacity < 100 || self.background_blur }`.

### 2. `src/backdrop.rs` (новый)
Инкапсулирует состояние и диспетчеризацию по ОС. Применяет/снимает блюр только
при смене состояния (idempotent), не каждый кадр.
```rust
pub struct Backdrop { enabled: bool, last: Option<bool> }
impl Backdrop {
    pub fn new(enabled: bool) -> Self;
    pub fn set_enabled(&mut self, on: bool);
    pub fn sync(&mut self, frame: &eframe::Frame); // frame.window_handle(); no-op без смены состояния
}
```
Диспетчер по ОС внутри `sync`:
- **Windows:** `window_vibrancy::apply_acrylic(h, Some((r,g,b,a)))` / `clear_acrylic(h)`.
- **macOS:** `apply_vibrancy(h, UnderWindowBackground, Active, None)` / `clear_vibrancy(h)`.
- **Linux X11:** `x11rb` — атом `_KDE_NET_WM_BLUR_BEHIND_REGION`,
  `ChangeProperty`(CARDINAL, `0`)/`DeleteProperty`. Окно из
  `RawWindowHandle::Xlib`/`Xcb`.
- **Linux Wayland:** no-op (с заглушкой-логом).

### 3. `src/app.rs`
- Поле `backdrop: Backdrop` (init из `theme.background_blur`).
- `is_transparent_theme()` → `settings.theme.app_bg_opacity < 100 || settings.theme.background_blur`.
- В `App::ui`: `self.backdrop.sync(frame)`.
- В `handle_window_actions` (ветка `actions.theme`):
  `self.backdrop.set_enabled(theme.background_blur)` +
  `ViewportCommand::Transparent(theme.wants_transparency())`.
- `self.transparent = theme.wants_transparency()`.

### 4. `src/ui/windows.rs` (Theme Settings → «Background», после `opacity_slider`)
```rust
ui.checkbox(&mut self.editing_theme.background_blur, "Background blur");
```

### 5. `src/main.rs`
- Без изменений логики: `transparent = App::is_transparent_theme()` уже учтёт
  блюр.

### 6. `Cargo.toml`
```toml
[target.'cfg(windows)'.dependencies]
window-vibrancy = "0.7"
[target.'cfg(target_os = "macos")'.dependencies]
window-vibrancy = "0.7"
[target.'cfg(target_os = "linux")'.dependencies]
x11rb = "0.13"
```

## Порядок реализации
1. `theme.rs` — поле + serde default.
2. `backdrop.rs` — модуль со стабами-`cfg` под все ОС.
3. `app.rs` — проводка `Backdrop` + `wants_transparency`.
4. `windows.rs` — чекбокс.
5. `Cargo.toml` — зависимости.
6. Платформенная реализация: X11 → Windows → macOS.
7. Проверка: `cargo clippy`, `cargo build`.

## Деградация
- GNOME/Mutter, Wayland без kwin_blur → просто прозрачность (как сейчас).
- Windows < 1809 → `apply_acrylic` вернёт `Err`, лог + no-op.

use alacritty_terminal::grid::Dimensions;
use egui_term::{PtyEvent, TerminalBackend, TerminalMode, TerminalView};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

const GROUPS_FILE: &str = "groups.json";
const SETTINGS_FILE: &str = "settings.json";

trait TerminalBackendExt {
    fn total_lines(&self) -> usize;
    fn screen_lines(&self) -> usize;
}

impl TerminalBackendExt for TerminalBackend {
    fn total_lines(&self) -> usize {
        self.last_content().grid.total_lines()
    }

    fn screen_lines(&self) -> usize {
        self.last_content().terminal_size.screen_lines()
    }
}

#[derive(Default)]
struct ScrollState {
    last_line_count: usize,
    user_scrolled_up: bool,
}

impl ScrollState {
    fn detect_clear(&self, current_lines: usize) -> bool {
        self.last_line_count > 0 && (current_lines as f64) < (self.last_line_count as f64) * 0.1
    }
}

#[derive(Default)]
struct TabScrollState {
    normal: ScrollState,
    alternate: ScrollState,
}

impl TabScrollState {
    fn current(&mut self, is_alternate: bool) -> &mut ScrollState {
        if is_alternate {
            &mut self.alternate
        } else {
            &mut self.normal
        }
    }
}

pub struct App {
    _command_sender: Sender<(u64, egui_term::PtyEvent)>,
    command_receiver: Receiver<(u64, egui_term::PtyEvent)>,
    tab_manager: TabManager,
    show_about: bool,
    show_hotkeys: bool,
    show_rename_group: bool,
    rename_group_id: Option<u64>,
    rename_group_name: String,
    show_debug: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let command_sender_clone = command_sender.clone();
        let tab_manager = TabManager::new(command_sender_clone, cc);

        let settings = Self::load_settings();

        Self {
            _command_sender: command_sender,
            command_receiver,
            tab_manager,
            show_about: false,
            show_hotkeys: false,
            show_rename_group: false,
            rename_group_id: None,
            rename_group_name: String::new(),
            show_debug: settings.show_debug,
        }
    }

    fn get_config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("yaaa");
            let _ = std::fs::create_dir_all(&path);
            path
        })
    }

    fn load_settings() -> Settings {
        if let Some(config_dir) = Self::get_config_dir() {
            let settings_file = config_dir.join(SETTINGS_FILE);
            if settings_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&settings_file) {
                    if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                        return settings;
                    }
                }
            }
        }
        Settings::default()
    }

    fn save_settings(&self) {
        if let Some(config_dir) = Self::get_config_dir() {
            let settings_file = config_dir.join(SETTINGS_FILE);
            let settings = Settings {
                show_debug: self.show_debug,
            };
            if let Ok(settings_json) = serde_json::to_string_pretty(&settings) {
                let _ = std::fs::write(&settings_file, settings_json);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            self.tab_manager.clear();
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("⚙ Settings", |ui| {
                        if ui
                            .button(if self.show_debug {
                                "🚫 Hide debug"
                            } else {
                                "🐞 Show debug"
                            })
                            .clicked()
                        {
                            self.show_debug = !self.show_debug;
                            self.save_settings();
                            ui.close();
                        }
                    });
                    ui.menu_button("❓ Help", |ui| {
                        if ui.button("⌘ Hotkeys").clicked() {
                            self.show_hotkeys = true;
                            ui.close();
                        }
                        if ui.button("🛈 About").clicked() {
                            self.show_about = true;
                            ui.close();
                        }
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.show_debug {
                        let fps = ctx.input(|i| 1.0 / i.stable_dt);

                        if let Some(tab) = self.tab_manager.get_active() {
                            let content = tab.backend.last_content();
                            let total_lines = tab.backend.total_lines();
                            let display_offset = content.grid.display_offset();
                            let view_size = tab.backend.screen_lines();
                            let from_bottom = display_offset;
                            let from_top = total_lines
                                .saturating_sub(display_offset)
                                .saturating_sub(view_size);

                            ui.label(format!(
                                "📊 Lines: {} | Top: {} | Bottom: {} | View: {} | FPS: {:.1}",
                                total_lines, from_top, from_bottom, view_size, fps
                            ));
                        } else {
                            ui.label(format!("FPS: {:.1}", fps));
                        }
                    }
                });
            });
        });

        egui::Window::new("About")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                ui.heading("Yet Another AI Agent");
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(10.0);
                ui.label("Multi-agent terminal with tabs and project management");
                ui.label("Manage multiple agent sessions across different projects");
                ui.add_space(10.0);
                ui.label("Author: Oleg Orlov (orelcokolov@gmail.com)");
            });

        egui::Window::new("Hotkeys")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_hotkeys)
            .show(ctx, |ui| {
                ui.heading("Keyboard Shortcuts");
                ui.add_space(10.0);

                egui::Grid::new("hotkeys_grid")
                    .num_columns(2)
                    .spacing([40.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Ctrl + Tab").strong());
                        ui.label("Switch to next tab");
                        ui.end_row();

                        ui.label(egui::RichText::new("Ctrl + Shift + Tab").strong());
                        ui.label("Switch to previous tab");
                        ui.end_row();

                        ui.label(egui::RichText::new("Ctrl + Shift + N").strong());
                        ui.label("Add new terminal tab");
                        ui.end_row();
                    });
            });

        let mut should_save = false;
        let mut should_close = false;
        egui::Window::new("Rename Group")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_rename_group)
            .show(ctx, |ui| {
                ui.heading("Rename Group");
                ui.text_edit_singleline(&mut self.rename_group_name);
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        should_save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_save {
            if let Some(group_id) = self.rename_group_id {
                self.tab_manager
                    .rename_group(group_id, self.rename_group_name.clone());
                self.tab_manager.save_groups();
            }
            self.show_rename_group = false;
            self.rename_group_id = None;
        }
        if should_close {
            self.show_rename_group = false;
            self.rename_group_id = None;
        }

        if let Ok((tab_id, event)) = self.command_receiver.try_recv() {
            match event {
                egui_term::PtyEvent::Exit => {
                    self.tab_manager.remove(tab_id);
                }
                egui_term::PtyEvent::Title(title) => {
                    self.tab_manager.set_title(tab_id, title);
                }
                _ => {}
            }
        }

        let active_group_id = self.tab_manager.active_group_id;
        let active_tab_id = self.tab_manager.active_tab_id;
        let groups_to_render: Vec<(u64, String, Vec<u64>)> = self
            .tab_manager
            .groups
            .iter()
            .map(|(id, g)| (*id, g.name.clone(), g.tab_ids.clone()))
            .collect();

        let mut add_group_clicked = false;
        let mut add_tab_to_group: Option<u64> = None;

        let input = ctx.input(|i| i.clone());
        if input.key_pressed(egui::Key::Tab) && input.modifiers.ctrl {
            if input.modifiers.shift {
                self.tab_manager.switch_to_prev_tab();
            } else {
                self.tab_manager.switch_to_next_tab();
            }
        }
        if input.key_pressed(egui::Key::N) && input.modifiers.ctrl && input.modifiers.shift {
            if let Some(group_id) = self.tab_manager.active_group_id {
                add_tab_to_group = Some(group_id);
            }
        }
        let mut group_actions: Vec<(u64, String, Vec<(u64, bool)>)> = Vec::new();

        egui::SidePanel::left("left_panel")
            .default_width(140.0)
            .show(ctx, |ui| {
                ui.style_mut().spacing.interact_size = egui::vec2(120.0, 24.0);
                ui.style_mut()
                    .text_styles
                    .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
                if ui.button("➕ Add project").clicked() {
                    add_group_clicked = true;
                }

                ui.separator();

                for (group_id, group_name, tab_ids) in &groups_to_render {
                    let is_selected = active_group_id == Some(*group_id);

                    ui.horizontal(|ui| {
                        ui.centered_and_justified(|ui| {
                            let sense = egui::Sense::click_and_drag();
                            let response = ui.allocate_rect(ui.available_rect_before_wrap(), sense);

                            let text_color = if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                                egui::Color32::LIGHT_BLUE
                            } else if is_selected {
                                ui.visuals().selection.stroke.color
                            } else {
                                ui.visuals().text_color()
                            };

                            ui.painter().text(
                                response.rect.center(),
                                egui::Align2::CENTER_CENTER,
                                group_name,
                                egui::FontId::proportional(18.0),
                                text_color,
                            );

                            if response.clicked() {
                                self.rename_group_id = Some(*group_id);
                                self.rename_group_name = group_name.clone();
                                self.show_rename_group = true;
                            }
                        });

                        if tab_ids.is_empty() && ui.small_button("×").clicked() {
                            group_actions.push((
                                *group_id,
                                String::from("remove_group"),
                                Vec::new(),
                            ));
                        }
                    });

                    for tab_id in tab_ids {
                        let tab_name = self.tab_manager.get_tab_name(*group_id, *tab_id);
                        let is_active = active_tab_id == Some(*tab_id);

                        ui.horizontal(|ui| {
                            let width = ui.available_width() * 0.9;
                            let label = egui::Button::selectable(is_active, tab_name)
                                .min_size(egui::vec2(width, 0.0));
                            if ui.add(label).clicked() {
                                group_actions.push((
                                    *group_id,
                                    String::from("select_tab"),
                                    vec![(*tab_id, false)],
                                ));
                            }

                            if ui
                                .add(egui::Button::new("🗙").min_size(egui::vec2(30.0, 0.0)))
                                .clicked()
                            {
                                group_actions.push((
                                    *group_id,
                                    String::from("remove_tab"),
                                    vec![(*tab_id, false)],
                                ));
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new("➕ New terminal")
                                    .min_size(egui::vec2(0.0, 16.0)),
                            )
                            .clicked()
                        {
                            add_tab_to_group = Some(*group_id);
                        }
                    });

                    ui.separator();
                }
            });

        for action in group_actions {
            let (group_id, action_type, data) = action;
            match action_type.as_str() {
                "remove_group" => {
                    self.tab_manager.remove_group(group_id);
                    self.tab_manager.save_groups();
                }
                "select_tab" => {
                    if let Some(tab_id) = data.first() {
                        self.tab_manager.set_active_tab(tab_id.0);
                    }
                }
                "remove_tab" => {
                    if let Some(tab_id) = data.first() {
                        self.tab_manager.remove(tab_id.0);
                    }
                }
                _ => {}
            }
        }

        if add_group_clicked {
            if self.tab_manager.groups.is_empty() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.tab_manager
                        .add_group_with_path(ctx.clone(), Some(path));
                    self.tab_manager.save_groups();
                }
            } else {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.tab_manager
                        .add_group_with_path(ctx.clone(), Some(path));
                    self.tab_manager.save_groups();
                }
            }
        }

        if let Some(group_id) = add_tab_to_group {
            self.tab_manager.add_tab_to_group(group_id, ctx.clone());
            self.tab_manager.save_groups();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tab) = self.tab_manager.get_active() {
                let content = tab.backend.last_content();
                let is_alternate = content.terminal_mode.contains(TerminalMode::ALT_SCREEN);
                let total_lines = tab.backend.total_lines();
                let viewport_height = ui.available_height();

                let mode_switched = tab.was_alternate_last_frame != is_alternate;
                let terminal_cleared =
                    !is_alternate && tab.scroll_state.normal.detect_clear(total_lines);

                if terminal_cleared || mode_switched {
                    let state = tab.scroll_state.current(is_alternate);
                    state.last_line_count = total_lines;
                    state.user_scrolled_up = false;

                    if terminal_cleared {
                        tab.backend.scroll_to_bottom();
                        tab.backend.clear_history();
                    }
                }

                tab.scroll_state.current(is_alternate).last_line_count = total_lines;
                tab.was_alternate_last_frame = is_alternate;

                let scroll_state = tab.scroll_state.current(is_alternate);

                egui::ScrollArea::vertical()
                    .id_salt(("terminal", tab.backend.id()))
                    .max_height(viewport_height)
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.set_height(viewport_height);

                        let should_block_input = tab.just_created;
                        let terminal = TerminalView::new(ui, &mut tab.backend)
                            .set_focus(!self.show_rename_group && !should_block_input)
                            .set_size(ui.available_size());

                        ui.add(terminal);

                        if tab.just_created {
                            tab.just_created = false;
                        }

                        if !is_alternate {
                            let inner_rect = ui.min_rect();
                            let viewport_bottom = ui.max_rect().bottom();
                            let content_bottom = inner_rect.bottom();
                            let is_at_bottom = content_bottom - viewport_bottom < 10.0;
                            scroll_state.user_scrolled_up = !is_at_bottom;
                        }
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No active tab. Select a group and add a tab.");
                });
            }
        });
    }
}

struct TabManager {
    command_sender: Sender<(u64, PtyEvent)>,
    groups: BTreeMap<u64, TabGroup>,
    tabs: BTreeMap<u64, Tab>,
    active_group_id: Option<u64>,
    active_tab_id: Option<u64>,
    next_group_id: u64,
    next_tab_id: u64,
}

impl TabManager {
    fn new(command_sender: Sender<(u64, PtyEvent)>, cc: &eframe::CreationContext<'_>) -> Self {
        let mut manager = Self {
            command_sender,
            groups: BTreeMap::new(),
            tabs: BTreeMap::new(),
            active_group_id: None,
            active_tab_id: None,
            next_group_id: 0,
            next_tab_id: 0,
        };

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        if let Some(groups_data) = manager.load_groups() {
            for group in groups_data {
                manager.next_group_id = manager.next_group_id.max(group.id + 1);
                for &tab_id in &group.tab_ids {
                    manager.next_tab_id = manager.next_tab_id.max(tab_id + 1);
                    let tab = Tab::new(
                        cc.egui_ctx.clone(),
                        manager.command_sender.clone(),
                        tab_id,
                        Some(group.path.clone()),
                    );
                    manager.tabs.insert(tab_id, tab);
                }
                manager.groups.insert(group.id, group);
            }
            if let Some(first_group) = manager.groups.first_key_value() {
                manager.active_group_id = Some(*first_group.0);
                manager.active_tab_id = first_group.1.tab_ids.first().copied();
            }
        }

        let current_dir_exists_in_groups = manager.groups.values().any(|g| g.path == current_dir);

        if !current_dir_exists_in_groups {
            let group_id = manager.next_group_id;
            manager.next_group_id += 1;
            let name = TabGroup::name_from_path(&current_dir);
            let group = TabGroup::new(group_id, name, current_dir);
            manager.groups.insert(group_id, group);
            manager.active_group_id = Some(group_id);

            manager.add_tab_to_group(group_id, cc.egui_ctx.clone());
        }

        manager
    }

    fn get_groups_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("yaaa");
            let _ = std::fs::create_dir_all(&path);
            path
        })
    }

    fn load_groups(&mut self) -> Option<Vec<TabGroup>> {
        if let Some(config_dir) = Self::get_groups_dir() {
            let groups_file = config_dir.join(GROUPS_FILE);
            if groups_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&groups_file) {
                    if let Ok(groups) = serde_json::from_str::<Vec<TabGroup>>(&content) {
                        return Some(groups);
                    }
                }
            }
        }
        None
    }

    fn save_groups(&self) {
        if let Some(config_dir) = Self::get_groups_dir() {
            let groups_file = config_dir.join(GROUPS_FILE);
            if let Ok(groups) =
                serde_json::to_string_pretty(&self.groups.values().collect::<Vec<_>>())
            {
                let _ = std::fs::write(&groups_file, groups);
            }
        }
    }

    fn add_group_with_path(&mut self, ctx: egui::Context, path: Option<PathBuf>) {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let path =
            path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let name = TabGroup::name_from_path(&path);

        let group = TabGroup::new(group_id, name, path);
        self.groups.insert(group_id, group);
        self.active_group_id = Some(group_id);

        self.add_tab_to_group(group_id, ctx);
    }

    fn rename_group(&mut self, group_id: u64, new_name: String) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.name = new_name;
        }
    }

    fn add_tab_to_group(&mut self, group_id: u64, ctx: egui::Context) {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let group_path = self.groups.get(&group_id).map(|g| g.path.clone());
        let tab = Tab::new(ctx, self.command_sender.clone(), tab_id, group_path);
        self.tabs.insert(tab_id, tab);

        if let Some(group) = self.groups.get_mut(&group_id) {
            group.tab_ids.push(tab_id);
        }

        self.active_group_id = Some(group_id);
        self.active_tab_id = Some(tab_id);
    }

    fn remove_group(&mut self, group_id: u64) {
        if let Some(group) = self.groups.get(&group_id) {
            for tab_id in &group.tab_ids {
                self.tabs.remove(tab_id);
            }
        }
        self.groups.remove(&group_id);

        if self.active_group_id == Some(group_id) {
            if let Some(first_group) = self.groups.first_key_value() {
                self.active_group_id = Some(*first_group.0);
                self.active_tab_id = first_group.1.tab_ids.first().copied();
            } else {
                self.active_group_id = None;
                self.active_tab_id = None;
            }
        }
    }

    fn remove(&mut self, id: u64) {
        let mut group_id_to_remove = None;
        let mut group_tab_ids = None;

        for (group_id, group) in &mut self.groups {
            if group.tab_ids.contains(&id) {
                group.tab_ids.retain(|t| t != &id);
                self.tabs.remove(&id);
                group_tab_ids = Some(group.tab_ids.clone());

                if group.tab_ids.is_empty() {
                    group_id_to_remove = Some(*group_id);
                }
                break;
            }
        }

        if let Some(group_id) = group_id_to_remove {
            self.remove_group(group_id);
            return;
        }

        if self.active_tab_id == Some(id) {
            if let Some(tab_ids) = group_tab_ids {
                self.active_tab_id = tab_ids.first().copied();
            }
        }
    }

    fn clear(&mut self) {
        self.groups.clear();
        self.tabs.clear();
        self.active_group_id = None;
        self.active_tab_id = None;
    }

    fn set_title(&mut self, id: u64, title: String) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.set_title(title);
        }
    }

    fn set_active_tab(&mut self, id: u64) {
        self.active_tab_id = Some(id);

        for (group_id, group) in &self.groups {
            if group.tab_ids.contains(&id) {
                self.active_group_id = Some(*group_id);
                break;
            }
        }

        if let Some(tab) = self.tabs.get_mut(&id) {
            let is_alternate = tab.is_alternate_screen();
            tab.scroll_state.current(is_alternate).user_scrolled_up = false;
        }
    }

    fn switch_to_next_tab(&mut self) {
        if let Some(group_id) = self.active_group_id {
            if let Some(group) = self.groups.get(&group_id) {
                let tab_ids = &group.tab_ids;
                if let Some(current_idx) = tab_ids
                    .iter()
                    .position(|&id| Some(id) == self.active_tab_id)
                {
                    let next_idx = (current_idx + 1) % tab_ids.len();
                    let new_tab_id = tab_ids[next_idx];
                    self.active_tab_id = Some(new_tab_id);
                    if let Some(tab) = self.tabs.get_mut(&new_tab_id) {
                        let is_alternate = tab.is_alternate_screen();
                        tab.scroll_state.current(is_alternate).user_scrolled_up = false;
                    }
                }
            }
        }
    }

    fn switch_to_prev_tab(&mut self) {
        if let Some(group_id) = self.active_group_id {
            if let Some(group) = self.groups.get(&group_id) {
                let tab_ids = &group.tab_ids;
                if let Some(current_idx) = tab_ids
                    .iter()
                    .position(|&id| Some(id) == self.active_tab_id)
                {
                    let prev_idx = if current_idx == 0 {
                        tab_ids.len() - 1
                    } else {
                        current_idx - 1
                    };
                    let new_tab_id = tab_ids[prev_idx];
                    self.active_tab_id = Some(new_tab_id);
                    if let Some(tab) = self.tabs.get_mut(&new_tab_id) {
                        let is_alternate = tab.is_alternate_screen();
                        tab.scroll_state.current(is_alternate).user_scrolled_up = false;
                    }
                }
            }
        }
    }

    fn get_tab_name(&self, group_id: u64, id: u64) -> String {
        if let Some(group) = self.groups.get(&group_id) {
            for (i, tab_id) in group.tab_ids.iter().enumerate() {
                if *tab_id == id {
                    return format!("Tab{}", i + 1);
                }
            }
        }
        format!("Tab{}", id + 1)
    }

    fn get_tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.get_mut(&id)
    }

    fn get_active(&mut self) -> Option<&mut Tab> {
        let _group_id = self.active_group_id?;
        let tab_id = self.active_tab_id?;

        self.tabs.get_mut(&tab_id)
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    #[serde(default = "default_show_debug")]
    show_debug: bool,
}

fn default_show_debug() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
struct TabGroup {
    id: u64,
    name: String,
    path: PathBuf,
    tab_ids: Vec<u64>,
}

impl TabGroup {
    fn new(id: u64, name: String, path: PathBuf) -> Self {
        Self {
            id,
            name,
            path,
            tab_ids: Vec::new(),
        }
    }

    fn name_from_path(path: &PathBuf) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }
}

struct Tab {
    backend: TerminalBackend,
    title: String,
    scroll_state: TabScrollState,
    was_alternate_last_frame: bool,
    just_created: bool,
}

impl Tab {
    fn is_alternate_screen(&self) -> bool {
        self.backend
            .last_content()
            .terminal_mode
            .contains(TerminalMode::ALT_SCREEN)
    }
}

impl Tab {
    fn new(
        ctx: egui::Context,
        command_sender: Sender<(u64, PtyEvent)>,
        id: u64,
        working_dir: Option<PathBuf>,
    ) -> Self {
        #[cfg(unix)]
        let system_shell = std::env::var("SHELL").expect("SHELL variable is not defined");
        #[cfg(windows)]
        let system_shell = "cmd.exe".to_string();

        let backend = TerminalBackend::new(
            id,
            ctx,
            command_sender,
            egui_term::BackendSettings {
                shell: system_shell,
                working_directory: working_dir,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            backend,
            title: format!("tab: {}", id),
            scroll_state: TabScrollState::default(),
            was_alternate_last_frame: false,
            just_created: true,
        }
    }

    fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

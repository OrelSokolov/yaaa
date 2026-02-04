use egui_term::{PtyEvent, TerminalBackend, TerminalView};
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, Sender},
};

pub struct App {
    command_sender: Sender<(u64, egui_term::PtyEvent)>,
    command_receiver: Receiver<(u64, egui_term::PtyEvent)>,
    tab_manager: TabManager,
    show_about: bool,
    show_rename_group: bool,
    rename_group_id: Option<u64>,
    rename_group_name: String,
}

impl App {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let command_sender_clone = command_sender.clone();
        Self {
            command_sender,
            command_receiver,
            tab_manager: TabManager::new(command_sender_clone),
            show_about: false,
            show_rename_group: false,
            rename_group_id: None,
            rename_group_name: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            self.tab_manager.clear();
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.show_about = true;
                        ui.close();
                    }
                });
            });
        });

        egui::Window::new("About")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                ui.heading("Multi-Agent Terminal");
                ui.label("A terminal application for managing multiple agent sessions.");
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
                    if ui.button("Save").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
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
                },
                egui_term::PtyEvent::Title(title) => {
                    self.tab_manager.set_title(tab_id, title);
                },
                _ => {},
            }
        }

        let active_group_id = self.tab_manager.active_group_id;
        let active_tab_id = self.tab_manager.active_tab_id;
        let mut groups_to_render: Vec<(u64, String, Vec<u64>)> = self
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
        if input.key_pressed(egui::Key::N)
            && input.modifiers.ctrl
            && input.modifiers.shift
        {
            if let Some(group_id) = self.tab_manager.active_group_id {
                add_tab_to_group = Some(group_id);
            }
        }
        let mut group_actions: Vec<(u64, String, Vec<(u64, bool)>)> =
            Vec::new();

        egui::SidePanel::left("left_panel")
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::proportional(16.0),
                );
                if ui.button("+ Add group").clicked() {
                    add_group_clicked = true;
                }

                ui.separator();

                for (group_id, group_name, tab_ids) in &groups_to_render {
                    let is_selected = active_group_id == Some(*group_id);

                    if is_selected {
                        ui.style_mut().visuals.override_text_color =
                            Some(ui.visuals().selection.stroke.color);
                    }

                    ui.horizontal(|ui| {
                        ui.centered_and_justified(|ui| {
                            let response = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(group_name.clone())
                                        .strong()
                                        .size(18.0),
                                )
                                .frame(false),
                            );
                            if response.hovered() {
                                ui.ctx()
                                    .set_cursor_icon(egui::CursorIcon::Text);
                            }
                            if response.clicked() {
                                self.rename_group_id = Some(*group_id);
                                self.rename_group_name = group_name.clone();
                                self.show_rename_group = true;
                            }
                        });

                        if tab_ids.is_empty() && ui.small_button("×").clicked()
                        {
                            group_actions.push((
                                *group_id,
                                String::from("remove_group"),
                                Vec::new(),
                            ));
                        }
                    });

                    if is_selected {
                        ui.style_mut().visuals.override_text_color = None;
                    }

                    for tab_id in tab_ids {
                        let tab_name = self.tab_manager.get_tab_name(*tab_id);
                        let is_active = active_tab_id == Some(*tab_id);

                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            if ui
                                .selectable_label(is_active, tab_name)
                                .clicked()
                            {
                                group_actions.push((
                                    *group_id,
                                    String::from("select_tab"),
                                    vec![(*tab_id, false)],
                                ));
                            }

                            if ui.small_button("×").clicked() {
                                group_actions.push((
                                    *group_id,
                                    String::from("remove_tab"),
                                    vec![(*tab_id, false)],
                                ));
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        if ui.small_button("+ Add").clicked() {
                            add_tab_to_group = Some(*group_id);
                        }
                    });

                    ui.separator();
                }
            });

        for action in group_actions {
            let (group_id, action_type, data) = action;
            match action_type.as_str() {
                "remove_group" => self.tab_manager.remove_group(group_id),
                "select_tab" => {
                    if let Some(tab_id) = data.first() {
                        self.tab_manager.set_active_tab(tab_id.0);
                    }
                },
                "remove_tab" => {
                    if let Some(tab_id) = data.first() {
                        self.tab_manager.remove(tab_id.0);
                    }
                },
                _ => {},
            }
        }

        if add_group_clicked {
            self.tab_manager.add_group(ctx.clone());
        }

        if let Some(group_id) = add_tab_to_group {
            self.tab_manager.add_tab_to_group(group_id, ctx.clone());
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tab) = self.tab_manager.get_active() {
                let terminal = TerminalView::new(ui, &mut tab.backend)
                    .set_focus(!self.show_rename_group)
                    .set_size(ui.available_size());

                ui.add(terminal);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No active tab. Select a group and add a tab.");
                });
            }
        });
    }
}

struct TabManager {
    command_sender: Sender<(u64, egui_term::PtyEvent)>,
    groups: BTreeMap<u64, TabGroup>,
    active_group_id: Option<u64>,
    active_tab_id: Option<u64>,
    next_group_id: u64,
    next_tab_id: u64,
}

impl TabManager {
    fn new(command_sender: Sender<(u64, PtyEvent)>) -> Self {
        let mut manager = Self {
            command_sender,
            groups: BTreeMap::new(),
            active_group_id: None,
            active_tab_id: None,
            next_group_id: 0,
            next_tab_id: 0,
        };

        manager.add_group(Default::default());
        manager
    }

    fn add_group(&mut self, ctx: egui::Context) {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let group = TabGroup::new(format!("Group {}", group_id + 1));
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

        let tab = Tab::new(ctx, self.command_sender.clone(), tab_id);
        self.groups
            .get_mut(&group_id)
            .unwrap()
            .tabs
            .insert(tab_id, tab);
        self.groups.get_mut(&group_id).unwrap().tab_ids.push(tab_id);

        self.active_group_id = Some(group_id);
        self.active_tab_id = Some(tab_id);
    }

    fn remove_group(&mut self, group_id: u64) {
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

        for (group_id, group) in &mut self.groups {
            if group.tab_ids.contains(&id) {
                group.tab_ids.retain(|t| t != &id);
                group.tabs.remove(&id);

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
            self.active_tab_id = self
                .groups
                .get(&self.active_group_id.unwrap())
                .and_then(|g| g.tab_ids.first().copied());
        }
    }

    fn clear(&mut self) {
        self.groups.clear();
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
                    self.active_tab_id = Some(tab_ids[next_idx]);
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
                    self.active_tab_id = Some(tab_ids[prev_idx]);
                }
            }
        }
    }

    fn get_tab_name(&self, id: u64) -> String {
        let all_ids: Vec<u64> = self
            .groups
            .values()
            .flat_map(|g| g.tab_ids.iter())
            .copied()
            .collect();

        for (i, tab_id) in all_ids.iter().enumerate() {
            if *tab_id == id {
                return format!("Tab{}", i + 1);
            }
        }

        format!("Tab{}", id + 1)
    }

    fn get_tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        for group in self.groups.values_mut() {
            if let Some(tab) = group.tabs.get_mut(&id) {
                return Some(tab);
            }
        }
        None
    }

    fn get_active(&mut self) -> Option<&mut Tab> {
        let group_id = self.active_group_id?;
        let tab_id = self.active_tab_id?;

        self.groups.get_mut(&group_id)?.tabs.get_mut(&tab_id)
    }
}

struct TabGroup {
    name: String,
    tab_ids: Vec<u64>,
    tabs: BTreeMap<u64, Tab>,
}

impl TabGroup {
    fn new(name: String) -> Self {
        Self {
            name,
            tab_ids: Vec::new(),
            tabs: BTreeMap::new(),
        }
    }
}

struct Tab {
    backend: TerminalBackend,
    title: String,
}

impl Tab {
    fn new(
        ctx: egui::Context,
        command_sender: Sender<(u64, PtyEvent)>,
        id: u64,
    ) -> Self {
        #[cfg(unix)]
        let system_shell =
            std::env::var("SHELL").expect("SHELL variable is not defined");
        #[cfg(windows)]
        let system_shell = "cmd.exe".to_string();

        let backend = TerminalBackend::new(
            id,
            ctx,
            command_sender,
            egui_term::BackendSettings {
                shell: system_shell,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            backend,
            title: format!("tab: {}", id),
        }
    }

    fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

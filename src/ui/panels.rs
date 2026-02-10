use crate::menu::apply_menu_style;
use crate::terminal::{TabInfo, TabManager, TerminalBackendExt};

fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

fn paste_from_clipboard() -> Option<String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .ok()
}

pub struct PanelActions {
    pub add_group_clicked: bool,
    pub add_tab_to_group: Option<u64>,
    pub add_agent_tab_to_group: Option<u64>,
    pub group_actions: Vec<(u64, String, Vec<(u64, bool)>)>,
}

impl Default for PanelActions {
    fn default() -> Self {
        Self {
            add_group_clicked: false,
            add_tab_to_group: None,
            add_agent_tab_to_group: None,
            group_actions: Vec::new(),
        }
    }
}

pub fn show_left_panel(
    ctx: &egui::Context,
    tab_manager: &TabManager,
    window_manager: &mut super::windows::WindowManager,
    show_sidebar: bool,
) -> PanelActions {
    let mut actions = PanelActions::default();

    let active_group_id = tab_manager.active_group_id;
    let active_tab_id = tab_manager.active_tab_id;
    let groups_to_render: Vec<(u64, String, Vec<TabInfo>)> = tab_manager
        .groups
        .iter()
        .map(|(id, g)| (*id, g.name.clone(), g.tabs.clone()))
        .collect();

    if show_sidebar {
        egui::SidePanel::left("left_panel")
            .default_width(100.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().spacing.interact_size = egui::vec2(120.0, 24.0);
                        ui.style_mut()
                            .text_styles
                            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
                        ui.add_space(8.0);
                        if groups_to_render.is_empty() {
                            let add_project_btn = ui
                                .button("➕ Add project")
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if add_project_btn.clicked() {
                                actions.add_group_clicked = true;
                            }
                        } else {
                            ui.label("My projects");
                        }
                        ui.add_space(8.0);

                        ui.separator();

                        for (group_id, group_name, tabs) in &groups_to_render {
                            let is_selected = active_group_id == Some(*group_id);

                            ui.horizontal(|ui| {
                                ui.centered_and_justified(|ui| {
                                    let sense = egui::Sense::click_and_drag();
                                    let response =
                                        ui.allocate_rect(ui.available_rect_before_wrap(), sense);

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
                                        window_manager.rename_group(*group_id, group_name.clone());
                                    }
                                });

                                if tabs.is_empty()
                                    && ui
                                        .small_button("×")
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                {
                                    actions.group_actions.push((
                                        *group_id,
                                        String::from("remove_group"),
                                        Vec::new(),
                                    ));
                                }
                            });

                            ui.add_space(10.0);

                            for tab_info in tabs {
                                let tab_id = tab_info.id;
                                let tab_name = tab_manager.get_tab_name(*group_id, tab_id);
                                let is_active = active_tab_id == Some(tab_id);

                                ui.horizontal(|ui| {
                                    let width = ui.available_width() * 0.9;
                                    let label = egui::Button::selectable(is_active, tab_name)
                                        .min_size(egui::vec2(width, 0.0));
                                    let response = ui
                                        .add(label)
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if response.clicked() {
                                        actions.group_actions.push((
                                            *group_id,
                                            String::from("select_tab"),
                                            vec![(tab_id, false)],
                                        ));
                                    }

                                    let close_btn = ui
                                        .add(egui::Button::new("✖").min_size(egui::vec2(30.0, 0.0)))
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if close_btn.clicked() {
                                        actions.group_actions.push((
                                            *group_id,
                                            String::from("remove_tab"),
                                            vec![(tab_id, false)],
                                        ));
                                    }
                                });
                            }

                            ui.horizontal(|ui| {
                                let terminal_btn = ui
                                    .add(
                                        egui::Button::new("➕ Terminal")
                                            .min_size(egui::vec2(0.0, 16.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if terminal_btn.clicked() {
                                    actions.add_tab_to_group = Some(*group_id);
                                }
                                let agent_btn = ui
                                    .add(
                                        egui::Button::new("➕ Agent")
                                            .min_size(egui::vec2(0.0, 16.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if agent_btn.clicked() {
                                    actions.add_agent_tab_to_group = Some(*group_id);
                                }
                            });

                            ui.separator();
                        }
                    });
            });
    }

    actions
}

pub fn show_central_panel(
    ctx: &egui::Context,
    tab_manager: &mut TabManager,
    window_manager: &super::windows::WindowManager,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(tab) = tab_manager.get_active() {
            let content = tab.backend.last_content();
            let is_alternate = content
                .terminal_mode
                .contains(egui_term::TerminalMode::ALT_SCREEN);
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
                    let terminal = egui_term::TerminalView::new(ui, &mut tab.backend)
                        .set_focus(
                            !window_manager.show_rename_group
                                && !window_manager.show_settings
                                && !should_block_input,
                        )
                        .set_size(ui.available_size());

                    let response = ui.add(terminal);

                    let selected_text = tab.backend.selectable_content();

                    response.context_menu(|ui| {
                        apply_menu_style(ui);

                        if !selected_text.is_empty() {
                            if ui.button("📋 Copy").clicked() {
                                copy_to_clipboard(&selected_text);
                                ui.close();
                            }
                        }
                        if ui.button("📝 Paste").clicked() {
                            if let Some(text) = paste_from_clipboard() {
                                tab.backend
                                    .process_command(egui_term::BackendCommand::Write(
                                        text.into_bytes(),
                                    ));
                            }
                            ui.close();
                        }
                    });

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

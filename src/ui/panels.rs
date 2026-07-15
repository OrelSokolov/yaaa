use crate::config::settings::{AgentConfig, MAX_AGENTS};
use crate::menu::apply_menu_style;
use crate::git_status::GitService;
use crate::terminal::{TabManager, TerminalBackendExt};
use crate::theme::AppTheme;

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

pub enum GroupAction {
    RemoveGroup,
    SelectTab(u64),
    RemoveTab(u64),
}

#[derive(Default)]
pub struct PanelActions {
    pub add_group_clicked: bool,
    pub add_tab_to_group: Option<u64>,
    pub add_agent_tab_to_group: Vec<(u64, usize)>,
    pub group_actions: Vec<(u64, GroupAction)>,
}

pub fn show_left_panel(
    ui: &mut egui::Ui,
    tab_manager: &TabManager,
    window_manager: &mut super::windows::WindowManager,
    show_sidebar: bool,
    agents: &[AgentConfig; MAX_AGENTS],
    theme: &AppTheme,
    git_service: &GitService,
) -> PanelActions {
    let mut actions = PanelActions::default();

    let active_group_id = tab_manager.active_group_id;
    let active_tab_id = tab_manager.active_tab_id;

    if show_sidebar {
        egui::Panel::left("left_panel")
            .default_size(100.0)
            .frame(egui::Frame {
                fill: theme.app_bg_with_opacity(),
                inner_margin: egui::Margin::symmetric(6, 0),
                ..Default::default()
            })
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().spacing.interact_size = egui::vec2(120.0, 24.0);
                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Body,
                            egui::FontId::proportional(theme.fonts.ui_font_size),
                        );
                        ui.add_space(8.0);
                        if tab_manager.groups.is_empty() {
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

                        for (group_id, group) in &tab_manager.groups {
                            let is_selected = active_group_id == Some(*group_id);

                            ui.horizontal(|ui| {
                                let centered = ui.centered_and_justified(|ui| {
                                    let sense = egui::Sense::click_and_drag();
                                    let response =
                                        ui.allocate_rect(ui.available_rect_before_wrap(), sense);

                                    let text_color = if response.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                                        theme.panel_text_hover
                                    } else if is_selected {
                                        theme.panel_text_selected
                                    } else {
                                        theme.panel_text
                                    };

                                    ui.painter().text(
                                        response.rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &group.name,
                                        egui::FontId::proportional(
                                            theme.fonts.group_name_font_size,
                                        ),
                                        text_color,
                                    );

                                    if response.clicked() {
                                        window_manager
                                            .rename_group(*group_id, group.name.clone());
                                    }

                                    response
                                });

                                // Git status icon on the right side of the centered group name
                                // area, drawn on top of the empty right part so the panel does
                                // not get widened. Skipped entirely when the service is disabled.
                                if git_service.enabled() {
                                    let (icon, icon_color) =
                                        match git_service.status_for(&group.path) {
                                            Some(status) => (status.icon(), status.color()),
                                            None => ("…", theme.panel_text),
                                        };

                                    let icon_size = theme.fonts.group_name_font_size;
                                    let icon_pos = egui::pos2(
                                        centered.response.rect.right() - 4.0,
                                        centered.response.rect.center().y,
                                    );
                                    ui.painter().text(
                                        icon_pos,
                                        egui::Align2::RIGHT_CENTER,
                                        icon,
                                        egui::FontId::proportional(icon_size),
                                        icon_color,
                                    );
                                }

                                if group.tabs.is_empty()
                                    && ui
                                        .small_button("×")
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                {
                                    actions.group_actions.push((
                                        *group_id,
                                        GroupAction::RemoveGroup,
                                    ));
                                }
                            });

                            ui.add_space(10.0);

                            for tab_info in &group.tabs {
                                let tab_id = tab_info.id;
                                let tab_name = tab_info.display_name.clone();
                                let is_active = active_tab_id == Some(tab_id);

                                ui.horizontal(|ui| {
                                    let width = ui.available_width() * 0.9;
                                    theme.tab_button.apply_to_visuals(ui);
                                    ui.visuals_mut().selection.bg_fill = theme.tab_active_bg;
                                    ui.visuals_mut().selection.stroke.color = theme.tab_button.text;
                                    ui.style_mut().text_styles.insert(
                                        egui::TextStyle::Button,
                                        egui::FontId::proportional(theme.fonts.tab_font_size),
                                    );
                                    let label = egui::Button::selectable(is_active, tab_name)
                                        .min_size(egui::vec2(width, 30.0));
                                    // Add extra vertical padding inside the tab button so the
                                    // tabs feel roomier and easier to hit.
                                    let old_padding = ui.style().spacing.button_padding;
                                    ui.style_mut().spacing.button_padding = egui::vec2(4.0, 4.0);

                                    // Center the tab label inside the button. The button is
                                    // placed inside a fixed-size child layout so it doesn't
                                    // widen the sidebar.
                                    let response = ui
                                        .allocate_ui_with_layout(
                                            egui::vec2(width, 30.0),
                                            egui::Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,
                                            ),
                                            |ui| ui.add(label),
                                        )
                                        .inner
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                                    ui.style_mut().spacing.button_padding = old_padding;
                                    if response.clicked() {
                                        actions
                                            .group_actions
                                            .push((*group_id, GroupAction::SelectTab(tab_id)));
                                    }

                                    theme.close_button.apply_to_visuals(ui);
                                    let close_btn = ui
                                        .add(egui::Button::new("✖").min_size(egui::vec2(30.0, 0.0)))
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if close_btn.clicked() {
                                        actions
                                            .group_actions
                                            .push((*group_id, GroupAction::RemoveTab(tab_id)));
                                    }
                                });
                            }

                            ui.horizontal(|ui| {
                                theme.terminal_button.apply_to_visuals(ui);
                                let terminal_btn = ui
                                    .add(
                                        egui::Button::new("➕ Terminal")
                                            .min_size(egui::vec2(0.0, 28.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if terminal_btn.clicked() {
                                    actions.add_tab_to_group = Some(*group_id);
                                }

                                for (idx, agent) in agents.iter().enumerate() {
                                    if !agent.enabled {
                                        continue;
                                    }
                                    let name = if agent.name.trim().is_empty() {
                                        format!("Агент {}", idx + 1)
                                    } else {
                                        agent.name.clone()
                                    };
                                    let has_cmd = !agent.cmd.trim().is_empty();
                                    theme.agent_button.apply_to_visuals(ui);
                                    let button = egui::Button::new(format!("➕ {}", name))
                                        .min_size(egui::vec2(0.0, 28.0));
                                    let response = if has_cmd {
                                        ui.add(button)
                                    } else {
                                        ui.add_enabled(false, button)
                                    };
                                    let response = response.on_hover_cursor(if has_cmd {
                                        egui::CursorIcon::PointingHand
                                    } else {
                                        egui::CursorIcon::NotAllowed
                                    });
                                    if !has_cmd {
                                        response.on_hover_text(
                                            "Configure a command for this agent in Agents settings",
                                        );
                                    } else if response.clicked() {
                                        actions.add_agent_tab_to_group.push((*group_id, idx));
                                    }
                                }
                            });

                            ui.separator();
                        }
                    });
            });
    }

    actions
}

pub fn show_search_panel(ui: &mut egui::Ui, tab_manager: &mut TabManager, theme: &AppTheme) {
    let Some(tab) = tab_manager.get_active() else {
        return;
    };

    if !tab.search_active {
        return;
    }

    let backend_id = tab.backend.id();
    let search_no_match = tab.backend.last_content().search_state.no_match;
    let search_textedit_id = egui::Id::new("search_input").with(backend_id);

    if tab.search_just_opened {
        ui.ctx().memory_mut(|m| m.request_focus(search_textedit_id));
        tab.search_just_opened = false;
    }

    egui::Panel::bottom("search_panel")
        .resizable(false)
        .default_size(40.0)
        .frame(egui::Frame {
            fill: theme.app_bg_with_opacity(),
            ..Default::default()
        })
        .show_inside(ui, |ui| {
            egui::Frame::NONE
                .inner_margin(egui::vec2(8.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let query_response = ui.add(
                            egui::TextEdit::singleline(&mut tab.search_query)
                                .id(search_textedit_id)
                                .desired_width(250.0)
                                .hint_text("Search...")
                                .min_size(egui::vec2(0.0, 24.0))
                                .margin(egui::vec2(4.0, 2.0)),
                        );

                        if query_response.changed() {
                            tab.backend.search_set_query(&tab.search_query);
                            if let Some(point) = tab.backend.search_current_match() {
                                tab.backend.scroll_to_point(point);
                            }
                        }

                        if search_no_match && !tab.search_query.is_empty() {
                            ui.label(
                                egui::RichText::new("Not found").color(ui.visuals().text_color()),
                            );
                        }

                        if ui
                            .add(egui::Button::new("⏶").min_size(egui::vec2(24.0, 24.0)))
                            .clicked()
                        {
                            if let Some(point) = tab.backend.search_prev() {
                                tab.backend.scroll_to_point(point);
                            }
                        }

                        if ui
                            .add(egui::Button::new("⏷").min_size(egui::vec2(24.0, 24.0)))
                            .clicked()
                        {
                            if let Some(point) = tab.backend.search_next() {
                                tab.backend.scroll_to_point(point);
                            }
                        }

                        if ui
                            .scope(|ui| {
                                ui.style_mut().spacing.button_padding = egui::vec2(12.0, 1.0);
                                ui.button("Search").clicked()
                            })
                            .inner
                        {
                            tab.backend.search_set_query(&tab.search_query);
                            if let Some(point) = tab.backend.search_current_match() {
                                tab.backend.scroll_to_point(point);
                            }
                        }
                    });
                });
        });
}

pub fn show_central_panel(
    ui: &mut egui::Ui,
    tab_manager: &mut TabManager,
    window_manager: &super::windows::WindowManager,
    theme: &AppTheme,
    terminal_theme: &egui_term::TerminalTheme,
    terminal_font: &egui_term::TerminalFont,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame {
            fill: theme.app_bg_with_opacity(),
            inner_margin: egui::Margin::same(4),
            ..Default::default()
        })
        .show_inside(ui, |ui| {
            let mut terminal_layout: Option<egui::Vec2> = None;
            if let Some(tab) = tab_manager.get_active() {
                let content = tab.backend.last_content();
                let is_alternate = content
                    .terminal_mode
                    .contains(egui_term::TerminalMode::ALT_SCREEN);
                let total_lines = tab.backend.total_lines();
                let viewport_height = ui.available_height();
                let backend_id = tab.backend.id();

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
                    .id_salt(("terminal", backend_id))
                    .max_height(viewport_height)
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.set_height(viewport_height);

                        let should_block_input = tab.just_created;
                        let layout_size = ui.available_size();
                        terminal_layout = Some(layout_size);
                        let terminal = egui_term::TerminalView::new(ui, &mut tab.backend)
                            .set_theme(terminal_theme.clone())
                            .set_font(terminal_font.clone())
                            .set_focus(
                                !window_manager.show_rename_group
                                    && !window_manager.show_settings
                                    && !window_manager.show_agents_settings
                                    && !window_manager.show_theme_settings
                                    && !window_manager.show_font_settings
                                    && !should_block_input
                                    && !tab.search_active,
                            )
                            .set_size(layout_size);

                        let response = ui.add(terminal);

                        response.context_menu(|ui| {
                            apply_menu_style(ui, theme.fonts.ui_font_size);

                            let has_selection =
                                tab.backend.last_content().selectable_range.is_some();

                            if has_selection {
                                if ui.button("📋 Copy").clicked() {
                                    let selected_text = tab.backend.selectable_content();
                                    let stripped_text: String = selected_text
                                        .split('\n')
                                        .map(|line| line.trim_end())
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    copy_to_clipboard(&stripped_text);
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

            if let Some(layout) = terminal_layout {
                tab_manager.set_terminal_layout_hint(egui_term::Size::from(layout));
            }
        });
}

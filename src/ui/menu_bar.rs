use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::recent_projects::RecentProject;
use crate::menu::apply_menu_style;
use crate::system_monitor::format_kb;
use crate::theme::AppTheme;

/// Read-only state the menu bar renders from. The menu itself never mutates
/// app state; everything the user clicks is reported back through
/// `MenuActions` and applied by `App`.
pub struct MenuBarView<'a> {
    pub theme: &'a AppTheme,
    pub enable_git_status: bool,
    pub preload_tabs: bool,
    pub show_sidebar: bool,
    pub show_system_monitor: bool,
    pub show_terminal_lines: bool,
    pub show_fps: bool,
    /// Paths of currently opened project groups (to filter the recent list).
    pub open_paths: &'a HashSet<PathBuf>,
    /// Recent projects, most recent first.
    pub recent_projects: &'a [RecentProject],
    /// System RAM usage percentage for the status area.
    pub ram_percent: f32,
    /// Summed memory of all terminal tabs, in KB.
    pub total_tabs_kb: u64,
}

/// Everything the user triggered in the menu bar during this frame.
#[derive(Default)]
pub struct MenuActions {
    pub add_project: bool,
    /// Open a recent project; the path may no longer exist, the handler
    /// must check and drop it from the recent list if missing.
    pub open_project: Option<RecentProject>,
    pub show_about: bool,
    pub show_theme_settings: bool,
    pub show_font_settings: bool,
    pub show_terminal_settings: bool,
    pub show_agents_settings: bool,
    pub show_hotkeys: bool,
    pub toggle_git_status: bool,
    pub toggle_preload_tabs: bool,
    pub toggle_system_monitor: bool,
    pub toggle_terminal_lines: bool,
    pub toggle_fps: bool,
    pub toggle_sidebar: bool,
    pub toggle_tab_memory: bool,
}

pub fn show_menu_bar(ui: &mut egui::Ui, view: MenuBarView<'_>) -> MenuActions {
    let mut actions = MenuActions::default();
    let theme = *view.theme;

    egui::Panel::top("menu_bar")
        .frame(egui::Frame {
            fill: theme.app_bg_with_opacity(),
            ..Default::default()
        })
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Button,
                        egui::FontId::proportional(theme.fonts.ui_font_size),
                    );
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Body,
                        egui::FontId::proportional(theme.fonts.ui_font_size),
                    );

                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.style_mut().spacing.button_padding = egui::vec2(6.0, 2.0);
                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Button,
                            egui::FontId::proportional(theme.fonts.ui_font_size),
                        );

                        ui.menu_button("H2Term byOrlov", |ui| {
                            apply_menu_style(ui, theme.fonts.ui_font_size);

                            if ui.button("ℹ About").clicked() {
                                actions.show_about = true;
                                ui.close();
                            }
                        });
                        ui.menu_button("Projects", |ui| {
                            apply_menu_style(ui, theme.fonts.ui_font_size);

                            if ui.button("➕ Add project").clicked() {
                                actions.add_project = true;
                                ui.close();
                            }

                            ui.separator();

                            let recent_projects: Vec<&RecentProject> = view
                                .recent_projects
                                .iter()
                                .filter(|p| !view.open_paths.contains(&p.path))
                                .collect();

                            if !recent_projects.is_empty() {
                                for project in recent_projects {
                                    if ui.button(&project.name).clicked() {
                                        actions.open_project = Some((*project).clone());
                                        ui.close();
                                    }
                                }
                            } else {
                                ui.label("No recent projects");
                            }
                        });
                        ui.menu_button("Settings", |ui| {
                            apply_menu_style(ui, theme.fonts.ui_font_size);

                            if ui.button("🎨 Theme").clicked() {
                                actions.show_theme_settings = true;
                                ui.close();
                            }

                            if ui.button("🔤 Fonts").clicked() {
                                actions.show_font_settings = true;
                                ui.close();
                            }

                            ui.separator();

                            let git_status_label = if view.enable_git_status {
                                "🔀 Hide git status"
                            } else {
                                "🔀 Show git status"
                            };
                            if ui.button(git_status_label).clicked() {
                                actions.toggle_git_status = true;
                                ui.close();
                            }

                            let preload_label = if view.preload_tabs {
                                "⚡ Disable terminal preload"
                            } else {
                                "⚡ Enable terminal preload"
                            };
                            if ui.button(preload_label).clicked() {
                                actions.toggle_preload_tabs = true;
                                ui.close();
                            }

                            let sysmon_label = if view.show_system_monitor {
                                "🖥 Hide system monitor"
                            } else {
                                "🖥 Show system monitor"
                            };
                            if ui.button(sysmon_label).clicked() {
                                actions.toggle_system_monitor = true;
                                ui.close();
                            }

                            if ui.button("💻 Terminal").clicked() {
                                actions.show_terminal_settings = true;
                                ui.close();
                            }
                            if ui.button("💬 Agents").clicked() {
                                actions.show_agents_settings = true;
                                ui.close();
                            }

                            ui.separator();

                            ui.menu_button("🐛 Debug", |ui| {
                                apply_menu_style(ui, theme.fonts.ui_font_size);

                                if ui
                                    .button(if view.show_terminal_lines {
                                        "🚫 Hide terminal lines"
                                    } else {
                                        "📊 Show terminal lines"
                                    })
                                    .clicked()
                                {
                                    actions.toggle_terminal_lines = true;
                                }
                                if ui
                                    .button(if view.show_fps {
                                        "🚫 Hide FPS"
                                    } else {
                                        "⚡ Show FPS"
                                    })
                                    .clicked()
                                {
                                    actions.toggle_fps = true;
                                }
                            });
                        });
                        ui.menu_button("Help", |ui| {
                            apply_menu_style(ui, theme.fonts.ui_font_size);
                            if ui.button("⌘ Hotkeys").clicked() {
                                actions.show_hotkeys = true;
                                ui.close();
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_text = if view.show_sidebar {
                                "📂 Hide Sidebar"
                            } else {
                                "📂 Show Sidebar"
                            };
                            if ui
                                .button(btn_text)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                actions.toggle_sidebar = true;
                            }

                            if view.show_system_monitor {
                                ui.add_space(16.0);
                                ui.label(format!("Total: {}", format_kb(view.total_tabs_kb)));

                                let ram_label = format!("RAM: {:.0}%", view.ram_percent);
                                if ui
                                    .button(ram_label)
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text(
                                        "Click to toggle per-tab memory next to tab names",
                                    )
                                    .clicked()
                                {
                                    actions.toggle_tab_memory = true;
                                }
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            });
        });

    actions
}

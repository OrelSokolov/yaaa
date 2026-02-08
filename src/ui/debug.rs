use crate::terminal::tab::TerminalBackendExt;

pub fn show_debug_panel(
    ctx: &egui::Context,
    show_fps: bool,
    show_terminal_lines: bool,
    tab_manager: &mut crate::terminal::TabManager,
) {
    if !show_fps && !show_terminal_lines {
        return;
    }

    egui::TopBottomPanel::bottom("debug_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if show_terminal_lines {
                if let Some(tab) = tab_manager.get_active() {
                    if ui
                        .button("Up")
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        tab.backend.scroll_to_top();
                    }
                    if ui
                        .button("Down")
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        tab.backend.scroll_to_bottom();
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut debug_parts = Vec::new();

                if show_terminal_lines {
                    if let Some(tab) = tab_manager.get_active() {
                        let content = tab.backend.last_content();
                        let total_lines = tab.backend.total_lines();
                        let display_offset = content.grid.display_offset();
                        let view_size = tab.backend.screen_lines();
                        let from_bottom = display_offset;
                        let from_top = total_lines
                            .saturating_sub(display_offset)
                            .saturating_sub(view_size);

                        debug_parts.push(format!(
                            "Lines: {} | Top: {} | Bottom: {} | View: {}",
                            total_lines, from_top, from_bottom, view_size
                        ));
                    }
                }

                if show_fps {
                    let fps = ctx.input(|i| 1.0 / i.stable_dt);
                    debug_parts.push(format!("FPS: {:.1}", fps));
                }

                if !debug_parts.is_empty() {
                    ui.label(format!("📊 {}", debug_parts.join(" | ")));
                }
            });
        });
    });
}

use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::recent_projects::RecentProject;
use crate::theme::AppTheme;

/// Sublime-style fuzzy project finder opened with Ctrl+Shift+O.
/// Shows a big search field over the recent projects list (the same list the
/// Projects menu shows, minus already opened projects).
pub struct ProjectFinder {
    pub is_open: bool,
    query: String,
    selected: usize,
    /// Set on open so the search field grabs keyboard focus on the frame it
    /// appears (same flag trick as `tab.search_just_opened`). Without it the
    /// terminal view re-grabs focus every frame.
    just_opened: bool,
    /// Set when the selection moves via keyboard so the list scrolls to it.
    scroll_to_selected: bool,
}

/// Emitted when the user picks a project (Enter or click).
#[derive(Clone)]
pub struct ProjectFinderAction {
    pub name: String,
    pub path: PathBuf,
}

impl ProjectFinder {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected: 0,
            just_opened: false,
            scroll_to_selected: false,
        }
    }

    pub fn open(&mut self) {
        self.query.clear();
        self.selected = 0;
        self.scroll_to_selected = false;
        self.just_opened = true;
        self.is_open = true;
    }

    fn close(&mut self) {
        self.is_open = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        projects: &[RecentProject],
        open_paths: &HashSet<PathBuf>,
        theme: &AppTheme,
    ) -> Option<ProjectFinderAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        // Same list as the Projects menu: recent projects not yet opened.
        let candidates: Vec<&RecentProject> = projects
            .iter()
            .filter(|p| !open_paths.contains(&p.path))
            .collect();

        let mut matched: Vec<(i32, Vec<usize>, &RecentProject)> = Vec::new();
        for project in &candidates {
            let path_str = project.path.to_string_lossy();
            if let Some((score, offsets)) =
                fuzzy_match(&self.query, &project.name, &path_str)
            {
                matched.push((score, offsets, project));
            }
        }
        // Empty query keeps the recency order of the projects list.
        if !self.query.is_empty() {
            matched.sort_by_key(|(score, _, _)| std::cmp::Reverse(*score));
        }
        if self.selected >= matched.len() {
            self.selected = matched.len().saturating_sub(1);
        }

        let input_id = egui::Id::new("project_finder_input");

        // Consume the navigation keys before the text edit sees them.
        let mut move_delta: i32 = 0;
        let mut open_selected = false;
        let escape_pressed = ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                move_delta += 1;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                move_delta -= 1;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                open_selected = true;
            }
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
        });
        if escape_pressed {
            self.close();
            return None;
        }

        if move_delta != 0 && !matched.is_empty() {
            let len = matched.len() as i32;
            self.selected = (self.selected as i32 + move_delta).rem_euclid(len) as usize;
            self.scroll_to_selected = true;
        }
        if open_selected && !matched.is_empty() {
            let (_, _, project) = &matched[self.selected];
            action = Some(ProjectFinderAction {
                name: project.name.clone(),
                path: project.path.clone(),
            });
        }

        let font_size = theme.fonts.ui_font_size;
        let query_font_size = (font_size * 1.5).max(20.0);

        egui::Window::new("Project Finder")
            .id(egui::Id::new("project_finder_window"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 70.0])
            .order(egui::Order::Foreground)
            .min_width(620.0)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(14.0).show(ui, |ui| {
                    // Big search field.
                    egui::Frame::default()
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .corner_radius(6.0)
                        .fill(egui::Color32::from_white_alpha(8))
                        .show(ui, |ui| {
                            let edit_response = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .id(input_id)
                                    .desired_width(f32::INFINITY)
                                    .frame(egui::Frame::NONE)
                                    // The hint must match the field's font size:
                                    // while the field is empty egui clips the
                                    // caret to the hint text rect, so a small
                                    // hint makes the caret look small too.
                                    .hint_text(
                                        egui::RichText::new("Search project…")
                                            .size(query_font_size),
                                    )
                                    .font(egui::FontId::proportional(query_font_size)),
                            );
                            // Request focus on the widget itself (not via
                            // memory before creation — that request is lost
                            // on the frame the window first appears).
                            if self.just_opened {
                                edit_response.request_focus();
                                self.just_opened = false;
                            }
                        });

                    ui.add_space(10.0);

                    if matched.is_empty() {
                        let message = if self.query.is_empty() {
                            "No recent projects"
                        } else {
                            "No matching projects"
                        };
                        ui.set_min_width(592.0);
                        ui.label(egui::RichText::new(message).size(font_size).weak());
                    } else {
                        let mut clicked_action = None;
                        egui::ScrollArea::vertical()
                            .id_salt("project_finder_list")
                            .max_height(340.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (index, (_, offsets, project)) in
                                    matched.iter().enumerate()
                                {
                                    let selected = index == self.selected;
                                    let job = item_layout(
                                        &project.name,
                                        offsets,
                                        &project.path.to_string_lossy(),
                                        theme,
                                        font_size,
                                        selected,
                                    );

                                    let (bg, hover_bg) = if selected {
                                        (theme.tab_active_bg, theme.tab_active_bg)
                                    } else {
                                        (
                                            egui::Color32::TRANSPARENT,
                                            egui::Color32::from_white_alpha(15),
                                        )
                                    };
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = bg;
                                    ui.style_mut().visuals.widgets.hovered.bg_fill = hover_bg;
                                    ui.style_mut().visuals.widgets.active.bg_fill =
                                        theme.tab_active_bg;

                                    let response = ui.add(
                                        egui::Button::new(job)
                                            .min_size(egui::vec2(ui.available_width(), 52.0)),
                                    );
                                    if selected && self.scroll_to_selected {
                                        response.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    if response.clicked() {
                                        clicked_action = Some(ProjectFinderAction {
                                            name: project.name.clone(),
                                            path: project.path.clone(),
                                        });
                                    }
                                }
                            });
                        if self.scroll_to_selected {
                            self.scroll_to_selected = false;
                        }
                        if let Some(clicked) = clicked_action {
                            action = Some(clicked);
                        }
                    }
                });
            });

        if action.is_some() {
            self.close();
        }
        action
    }
}

/// Build the two-line item text (project name + path) with the fuzzy-matched
/// characters of the name highlighted.
fn item_layout(
    name: &str,
    matched_offsets: &[usize],
    path: &str,
    theme: &AppTheme,
    font_size: f32,
    selected: bool,
) -> egui::text::LayoutJob {
    let base = if selected {
        theme.tab_text
    } else {
        theme.panel_text
    };
    let highlight = theme.panel_text_selected;
    let dim = base.gamma_multiply(0.6);
    let name_size = (font_size * 1.25).max(16.0);
    let path_size = (font_size * 0.9).max(11.0);

    let mut job = egui::text::LayoutJob::default();

    // Group the name into highlighted/plain runs of whole characters.
    let mut run_start = 0;
    let mut run_is_match = false;
    for (offset, c) in name.char_indices() {
        let is_match = matched_offsets.contains(&offset);
        if is_match != run_is_match && offset > run_start {
            let text = &name[run_start..offset];
            let color = if run_is_match { highlight } else { base };
            job.append(
                text,
                0.0,
                egui::TextFormat::simple(
                    egui::FontId::proportional(name_size),
                    color,
                ),
            );
            run_start = offset;
        }
        run_is_match = is_match;
        let _ = c;
    }
    let tail_color = if run_is_match { highlight } else { base };
    job.append(
        &name[run_start..],
        0.0,
        egui::TextFormat::simple(egui::FontId::proportional(name_size), tail_color),
    );

    if !path.is_empty() {
        job.append(
            "\n",
            0.0,
            egui::TextFormat::simple(egui::FontId::proportional(name_size), base),
        );
        job.append(
            path,
            0.0,
            egui::TextFormat::simple(egui::FontId::proportional(path_size), dim),
        );
    }

    job
}

/// Case-insensitive fuzzy match. Tries the project name first (a name match
/// also returns the byte offsets used for highlighting), then falls back to
/// the full path. Returns `None` when the query matches neither.
fn fuzzy_match(query: &str, name: &str, path: &str) -> Option<(i32, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    let query_chars: Vec<char> =
        query.chars().map(|c| c.to_ascii_lowercase()).collect();

    match_subsequence(&query_chars, name)
        .map(|(score, offsets)| (score + 5, offsets))
        .or_else(|| {
            match_subsequence(&query_chars, path).map(|(score, _)| (score, Vec::new()))
        })
}

/// Subsequence matcher with a simple relevance score: word-boundary and
/// consecutive matches count more, longer texts count slightly less.
fn match_subsequence(query: &[char], text: &str) -> Option<(i32, Vec<usize>)> {
    let mut score = 0;
    let mut offsets = Vec::new();
    let mut qi = 0;
    let mut prev_match_end: Option<usize> = None;

    for (offset, c) in text.char_indices() {
        if qi < query.len() && c.to_ascii_lowercase() == query[qi] {
            score += 10;
            if prev_match_end == Some(offset) {
                score += 15; // consecutive match
            }
            if is_word_boundary(text, offset) {
                score += 20; // start of a word
            }
            offsets.push(offset);
            prev_match_end = Some(offset + c.len_utf8());
            qi += 1;
        }
    }

    if qi == query.len() {
        score -= text.chars().count() as i32 / 10;
        Some((score, offsets))
    } else {
        None
    }
}

/// Whether the character at `byte_offset` starts a "word" (start of text,
/// after a separator, or an uppercase letter after a lowercase one).
fn is_word_boundary(text: &str, byte_offset: usize) -> bool {
    let Some(prev) = text[..byte_offset].chars().next_back() else {
        return true;
    };
    let Some(current) = text[byte_offset..].chars().next() else {
        return false;
    };
    matches!(prev, '/' | '_' | '-' | ' ' | '.' | '(')
        || (prev.is_lowercase() && current.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        let (score, offsets) = fuzzy_match("", "h2term", "/home/oleg/h2term").unwrap();
        assert_eq!(score, 0);
        assert!(offsets.is_empty());
    }

    #[test]
    fn name_match_returns_highlight_offsets() {
        let (_, offsets) = fuzzy_match("h2t", "h2term", "/home/oleg/h2term").unwrap();
        assert_eq!(offsets, vec![0, 1, 2]);
    }

    #[test]
    fn path_fallback_matches_without_highlights() {
        let (_, offsets) = fuzzy_match("oleg", "h2term", "/home/oleg/h2term").unwrap();
        assert!(offsets.is_empty());
    }

    #[test]
    fn no_match_returns_none() {
        assert!(fuzzy_match("zzz", "h2term", "/home/oleg/h2term").is_none());
    }

    #[test]
    fn boundary_match_scores_higher_than_scattered() {
        let (boundary, _) = match_subsequence(&['h', '2'], "h2term").unwrap();
        // Simulate scattered letters via a different text where h and 2 are
        // not adjacent: they must exist in order.
        let (scattered, _) = match_subsequence(&['h', '2'], "ha12").unwrap();
        assert!(boundary > scattered);
    }
}

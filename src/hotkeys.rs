use egui::Context;
use std::collections::BTreeMap;

pub fn get_hotkeys() -> BTreeMap<&'static str, &'static str> {
    let mut hotkeys = BTreeMap::new();
    hotkeys.insert("Ctrl + Tab", "Switch to next tab");
    hotkeys.insert("Ctrl + Shift + Tab", "Switch to previous tab");
    hotkeys.insert("Ctrl + Shift + N", "Add new terminal tab");
    hotkeys.insert("Ctrl + Shift + A", "Add new agent tab (first agent only)");
    hotkeys.insert("Ctrl + Shift + O", "Open project finder (fuzzy search)");
    hotkeys.insert("Ctrl + Shift + Q", "Close current tab");
    hotkeys.insert("Ctrl + Shift + Page Up", "Scroll terminal one page up");
    hotkeys.insert("Ctrl + Shift + Page Down", "Scroll terminal one page down");
    hotkeys.insert("Ctrl + Shift + Home", "Scroll terminal to top");
    hotkeys.insert("Ctrl + Shift + End", "Scroll terminal to bottom");
    hotkeys.insert("Ctrl + F", "Toggle search in terminal");
    hotkeys
}

pub struct KeyboardEvents {
    pub switch_to_next_tab: bool,
    pub switch_to_prev_tab: bool,
    pub add_terminal_tab: bool,
    pub add_agent_tab: Option<usize>,
    pub close_tab: bool,
    pub scroll_to_top: bool,
    pub scroll_to_bottom: bool,
    pub scroll_page_up: bool,
    pub scroll_page_down: bool,
    pub toggle_search: bool,
    pub open_project_finder: bool,
}

pub fn handle_keyboard_events(ctx: &Context, active_group_exists: bool) -> KeyboardEvents {
    let input = ctx.input(|i| i.clone());

    let mut events = KeyboardEvents {
        switch_to_next_tab: false,
        switch_to_prev_tab: false,
        add_terminal_tab: false,
        add_agent_tab: None,
        close_tab: false,
        scroll_to_top: false,
        scroll_to_bottom: false,
        scroll_page_up: false,
        scroll_page_down: false,
        toggle_search: false,
        open_project_finder: false,
    };

    if input.key_pressed(egui::Key::Tab) && input.modifiers.ctrl {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::Tab));
        if input.modifiers.shift {
            events.switch_to_prev_tab = true;
        } else {
            events.switch_to_next_tab = true;
        }
    }

    if active_group_exists
        && input.key_pressed(egui::Key::N)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::N));
        events.add_terminal_tab = true;
    }

    if active_group_exists
        && input.key_pressed(egui::Key::A)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::A));
        events.add_agent_tab = Some(0);
    }

    if input.key_pressed(egui::Key::Q) && input.modifiers.ctrl && input.modifiers.shift {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::Q));
        events.close_tab = true;
    }

    if active_group_exists
        && input.key_pressed(egui::Key::Home)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::Home));
        events.scroll_to_top = true;
    }

    if active_group_exists
        && input.key_pressed(egui::Key::End)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::End));
        events.scroll_to_bottom = true;
    }

    if active_group_exists
        && input.key_pressed(egui::Key::PageUp)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::PageUp));
        events.scroll_page_up = true;
    }

    if active_group_exists
        && input.key_pressed(egui::Key::PageDown)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::PageDown));
        events.scroll_page_down = true;
    }

    if input.key_pressed(egui::Key::O) && input.modifiers.ctrl && input.modifiers.shift {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::O));
        events.open_project_finder = true;
    }

    if active_group_exists && input.key_pressed(egui::Key::F) && input.modifiers.ctrl {
        ctx.input_mut(|i| i.consume_key(i.modifiers, egui::Key::F));
        events.toggle_search = true;
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inject a key press with modifiers into the context without running a
    /// full egui pass; `key_pressed` scans the raw event list.
    fn press(ctx: &Context, key: egui::Key, modifiers: egui::Modifiers) {
        ctx.input_mut(|i| {
            i.modifiers = modifiers;
            i.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
        });
    }

    #[test]
    fn no_input_produces_no_events() {
        let ctx = Context::default();
        let events = handle_keyboard_events(&ctx, true);
        assert!(!events.switch_to_next_tab);
        assert!(!events.add_terminal_tab);
        assert!(events.add_agent_tab.is_none());
        assert!(!events.close_tab);
    }

    #[test]
    fn ctrl_tab_cycles_tabs() {
        let ctx = Context::default();
        press(&ctx, egui::Key::Tab, egui::Modifiers::CTRL);
        let events = handle_keyboard_events(&ctx, true);
        assert!(events.switch_to_next_tab);
        assert!(!events.switch_to_prev_tab);
    }

    #[test]
    fn ctrl_shift_tab_cycles_back() {
        let ctx = Context::default();
        press(&ctx, egui::Key::Tab, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
        let events = handle_keyboard_events(&ctx, true);
        assert!(events.switch_to_prev_tab);
        assert!(!events.switch_to_next_tab);
    }

    #[test]
    fn new_tab_requires_active_group() {
        let ctx = Context::default();
        press(&ctx, egui::Key::N, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
        assert!(handle_keyboard_events(&ctx, true).add_terminal_tab);
        assert!(!handle_keyboard_events(&ctx, false).add_terminal_tab);
    }

    #[test]
    fn ctrl_shift_a_opens_first_agent_tab() {
        let ctx = Context::default();
        press(&ctx, egui::Key::A, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
        let events = handle_keyboard_events(&ctx, true);
        assert_eq!(events.add_agent_tab, Some(0));
    }

    #[test]
    fn ctrl_f_toggles_search() {
        let ctx = Context::default();
        press(&ctx, egui::Key::F, egui::Modifiers::CTRL);
        assert!(handle_keyboard_events(&ctx, true).toggle_search);
    }

    #[test]
    fn ctrl_shift_q_closes_tab() {
        let ctx = Context::default();
        press(&ctx, egui::Key::Q, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
        assert!(handle_keyboard_events(&ctx, true).close_tab);
    }
}

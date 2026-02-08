use egui::Context;
use std::collections::BTreeMap;

pub fn get_hotkeys() -> BTreeMap<&'static str, &'static str> {
    let mut hotkeys = BTreeMap::new();
    hotkeys.insert("Ctrl + Tab", "Switch to next tab");
    hotkeys.insert("Ctrl + Shift + Tab", "Switch to previous tab");
    hotkeys.insert("Ctrl + Shift + N", "Add new terminal tab");
    hotkeys.insert("Ctrl + Shift + A", "Add new agent tab");
    hotkeys.insert("Ctrl + Shift + Q", "Close current tab");
    hotkeys.insert("Ctrl + Shift + Page Up", "Scroll terminal one page up");
    hotkeys.insert("Ctrl + Shift + Page Down", "Scroll terminal one page down");
    hotkeys.insert("Ctrl + Shift + Home", "Scroll terminal to top");
    hotkeys.insert("Ctrl + Shift + End", "Scroll terminal to bottom");
    hotkeys
}

pub struct KeyboardEvents {
    pub switch_to_next_tab: bool,
    pub switch_to_prev_tab: bool,
    pub add_terminal_tab: bool,
    pub add_agent_tab: bool,
    pub close_tab: bool,
    pub scroll_to_top: bool,
    pub scroll_to_bottom: bool,
    pub scroll_page_up: bool,
    pub scroll_page_down: bool,
}

pub fn handle_keyboard_events(ctx: &Context, active_group_exists: bool) -> KeyboardEvents {
    let input = ctx.input(|i| i.clone());

    let mut events = KeyboardEvents {
        switch_to_next_tab: false,
        switch_to_prev_tab: false,
        add_terminal_tab: false,
        add_agent_tab: false,
        close_tab: false,
        scroll_to_top: false,
        scroll_to_bottom: false,
        scroll_page_up: false,
        scroll_page_down: false,
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
        events.add_agent_tab = true;
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

    events
}

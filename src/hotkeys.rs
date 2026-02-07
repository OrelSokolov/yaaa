use egui::Context;
use std::collections::BTreeMap;

pub fn get_hotkeys() -> BTreeMap<&'static str, &'static str> {
    let mut hotkeys = BTreeMap::new();
    hotkeys.insert("Ctrl + Tab", "Switch to next tab");
    hotkeys.insert("Ctrl + Shift + Tab", "Switch to previous tab");
    hotkeys.insert("Ctrl + Shift + N", "Add new terminal tab");
    hotkeys.insert("Ctrl + Shift + A", "Add new agent tab");
    hotkeys.insert("Ctrl + Shift + Q", "Close current tab");
    hotkeys
}

pub struct KeyboardEvents {
    pub switch_to_next_tab: bool,
    pub switch_to_prev_tab: bool,
    pub add_terminal_tab: bool,
    pub add_agent_tab: bool,
    pub close_tab: bool,
}

pub fn handle_keyboard_events(ctx: &Context, active_group_exists: bool) -> KeyboardEvents {
    let input = ctx.input(|i| i.clone());

    let mut events = KeyboardEvents {
        switch_to_next_tab: false,
        switch_to_prev_tab: false,
        add_terminal_tab: false,
        add_agent_tab: false,
        close_tab: false,
    };

    if input.key_pressed(egui::Key::Tab) && input.modifiers.ctrl {
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
        events.add_terminal_tab = true;
    }

    if active_group_exists
        && input.key_pressed(egui::Key::A)
        && input.modifiers.ctrl
        && input.modifiers.shift
    {
        events.add_agent_tab = true;
    }

    if input.key_pressed(egui::Key::Q) && input.modifiers.ctrl && input.modifiers.shift {
        events.close_tab = true;
    }

    events
}

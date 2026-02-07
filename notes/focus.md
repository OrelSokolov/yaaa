# Focus Management

## Overview
This document describes how focus management works in the Yet Another AI Agent application, particularly for the settings window.

## Settings Window Focus

When the settings window is opened, the application uses a focus management system to prevent keyboard input from being sent to the terminal while the settings are being edited.

### Implementation Details

The focus control is implemented in `src/app.rs` at line 670-674:

```rust
let terminal = TerminalView::new(ui, &mut tab.backend)
    .set_focus(
        !self.show_rename_group
            && !self.show_settings
            && !should_block_input,
    )
    .set_size(ui.available_size());
```

### Focus Conditions

The terminal receives focus (`true`) only when **all** of the following conditions are met:

1. **`!self.show_rename_group`** - The "Rename Group" window is NOT open
2. **`!self.show_settings`** - The "Settings" window is NOT open
3. **`!should_block_input`** - The tab was just created and shouldn't receive input yet

When the settings window is opened (`self.show_settings = true`), the terminal receives `false` from `set_focus()`, which blocks all keyboard input from being sent to the terminal.

### Settings Window UI

The settings window is created at lines 358-388 in `src/app.rs`:

```rust
egui::Window::new("Settings")
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .open(&mut self.show_settings)
    .show(ctx, |ui| {
        ui.heading("General Settings");
        ui.add_space(10.0);

        ui.label("Default shell cmd:");
        ui.text_edit_singleline(&mut self.editing_default_shell_cmd);

        ui.add_space(5.0);

        ui.label("Default agent cmd:");
        ui.text_edit_singleline(&mut self.editing_default_agent_cmd);

        ui.add_space(15.0);

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            settings_cancel = true;
        }

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                settings_save = true;
            }
            if ui.button("Cancel").clicked() {
                settings_cancel = true;
            }
        });
    });
```

### Opening Settings Window

The settings window is opened from the menu bar (line 202-207):

```rust
ui.menu_button("🔧 General", |ui| {
    apply_menu_style(ui);
    if ui.button("💻 Terminal").clicked() {
        self.show_settings = true;
        ui.close();
    }
});
```

### Keyboard Shortcuts

When the settings window is open:
- **Enter** - Save settings and close the window
- **Escape** - Cancel changes and close the window

## Related Windows

The same focus mechanism is also used for the "Rename Group" window (`self.show_rename_group`), ensuring consistent behavior across modal windows.

## State Variables

- `show_settings: bool` - Controls visibility of the settings window
- `editing_default_shell_cmd: String` - Currently edited shell command
- `editing_default_agent_cmd: String` - Currently edited agent command
- `saved_default_shell_cmd: String` - Saved shell command (for cancel)
- `saved_default_agent_cmd: String` - Saved agent command (for cancel)

## Save/Cancel Behavior

### Save (lines 390-398):
```rust
if settings_save {
    self.saved_default_shell_cmd = self.editing_default_shell_cmd.clone();
    self.saved_default_agent_cmd = self.editing_default_agent_cmd.clone();
    self.tab_manager
        .set_default_shell_cmd(self.editing_default_shell_cmd.clone());
    self.tab_manager
        .set_default_agent_cmd(self.editing_default_agent_cmd.clone());
    self.save_settings();
    self.show_settings = false;
}
```

### Cancel (lines 400-404):
```rust
if settings_cancel {
    self.editing_default_shell_cmd = self.saved_default_shell_cmd.clone();
    self.editing_default_agent_cmd = self.saved_default_agent_cmd.clone();
    self.show_settings = false;
}
```

When either save or cancel is executed, `self.show_settings` is set to `false`, which allows the terminal to regain focus in the next frame.

# Changelog

## [0.3.2] - 2026-02-07

### Added
- My projects panel with improved project management
- Recent projects to menu for quick access to recently used projects

### Changed
- Large refactoring of code structure
- Update debian package building to version 3.1
- Fix GitHub Actions for cargo deb

### Fixed
- Namespace fixes

## [0.3.0] - 2026-02-06

### Changed
- Update UI with improved styling

### Fixed
- Fix bug with wrong path to agent
- Fix launcher
- Fixes for desktop run

## [0.2.0] - 2025-02-06

### Added
- Custom shell and agent commands settings
- Settings window for configuring default shell and agent commands
- Agent tabs support with Ctrl+Shift+A hotkey
- Centralized hotkey definitions system
- Enhanced tab naming to distinguish between Terminal and Agent tabs

### Changed
- Update egui_term dependency from local path to GitHub repository (OrelSokolov/egui_term)
- Improved Settings menu organization with submenus (General, Debug)
- Updated cursor icons on hover for buttons
- Updated close button icon (× → ✖)
- Updated About button icon (❓ → ℹ)
- Refactored tab management to support different tab types

## [0.1.4] - 2025-02-06

### Added
- Scroll area to sidebar for handling overflow tabs
- Blue highlight on group name hover
- Focus to last tab

### Changed
- Improve menu layout: add background color and bottom padding, move debug info to separate panel
- Split debug option into separate terminal lines and FPS controls, increase menu size
- Create reusable menu style helper to unify Settings and Help menus
- Set minimum width for dropdown menus to prevent text wrapping

## [0.1.3] - 2025-02-06

### Added
- Hotkeys dialog for viewing keyboard shortcuts
- FPS counter in debug mode

### Changed
- New menu item for About
- Update about dialog

### Fixed
- Debug for terminal lines
- Cleanup warnings
- Fix scrollback history clearing on terminal clear
- Fix tab numbering to be project-specific instead of global

## [0.1.2] - 2025-02-05

### Added
- Auto-group creation for current directory: when launching yaaa from a directory not in saved groups, automatically creates a new group with an active terminal tab focused

### Changed
- Updated "Add group" button to "Add project" for clarity

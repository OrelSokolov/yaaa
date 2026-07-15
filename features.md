# YAAA — Features

## Brief

**YAAA — a fast, lightweight terminal that lets you run multiple AI coding agents side by side across all your projects.**

---

## For Landing

### Run multiple agents in parallel
Open several AI coding agents (Claude Code, OpenCode, etc.) in separate tabs within the same project — no more window juggling.

### All your projects in one place
Group terminals and agent sessions by project folder. Switch between projects instantly, with everything exactly where you left it.

### Built for speed
Native binary, no Electron, no bloat. Launches instantly and uses minimal memory.

### Your terminal, your style
Full theme customization — colors, fonts, opacity, and transparency. Make it yours.

### Everything saved automatically
Your tabs, projects, and agent configurations are restored on every launch. Pick up right where you left off.

### Native cross-platform
Works on macOS, Linux, and Windows with one-click install.

---

## Full Feature List

### Terminal & Tabs
- Multi-tab terminal with unlimited sessions
- Project-based tab grouping (one folder = one group)
- Tab switching via Ctrl+Tab / Ctrl+Shift+Tab
- Quick-add terminal (Ctrl+Shift+N) and agent (Ctrl+Shift+A) tabs
- Close tabs with Ctrl+Shift+Q
- Auto-group creation for current working directory on launch

### AI Agent Integration
- Up to 4 configurable AI agents (name + command)
- Default agent: OpenCode (`opencode`)
- Per-project agent tabs with custom working directories
- Agent command with arguments support
- Login shell mode toggle

### Project Management
- "My Projects" sidebar with folder-based groups
- Recent projects menu for quick re-opening
- Group renaming
- Automatic session persistence (groups.json)
- Native folder picker for adding projects

### Search
- In-terminal search (Ctrl+F)
- Navigate matches with prev/next buttons
- Real-time match highlighting

### Scrolling & Navigation
- Page Up / Page Down scrolling (Ctrl+Shift+PageUp/Down)
- Scroll to top / bottom (Ctrl+Shift+Home/End)
- Smart scroll: auto-follows output, detects user scroll-up
- Alternate screen mode support (e.g. vim, less)
- Scrollback history clearing on terminal clear

### Clipboard
- Right-click context menu with Copy / Paste
- Selection-aware copy (only shows Copy when text is selected)
- Clipboard paste directly into terminal

### Theming & Appearance
- Custom theme system with live preview
- Adjustable background color and opacity (0–100% transparency)
- Customizable UI colors (sidebar text, selected, hover, tabs)
- Per-element button styling (tab, close, agent, terminal buttons)
- Customizable font sizes (UI, group names, tabs, terminal)
- System font fallback support
- Forced dark mode (consistent across platforms)

### Settings & Persistence
- Settings window for shell configuration
- Agents settings window (up to 4 agents with enable/disable)
- Theme settings window with collapsible sections
- Font settings window with live preview
- All settings persisted to JSON
- Legacy single-agent settings migration

### Shell Support
- Auto-detected shell chain ($SHELL → zsh → bash)
- Configurable default shell command
- Login shell mode (`--login` flag)
- Shell fallback mechanism (tries multiple shells if one fails)
- Cross-platform (zsh/bash on Unix, cmd/powershell on Windows)

### Debug Tools
- Toggleable terminal line count display
- FPS counter
- Debug panel in sidebar

### UI / UX
- Collapsible sidebar (show/hide)
- Exit confirmation dialog
- Hotkeys reference window
- About window with version info
- Hover cursors (pointing hand, not-allowed)
- Responsive layout with scroll areas
- Menu bar with Projects, Settings, and Help menus

### Installation
- macOS: Ruby installer with `.app` bundle and ad-hoc signing
- Linux: `.deb` package with desktop integration and icons
- Windows: MSI installer via WiX
- Specific version installation support
- Custom install directory support

### Performance
- Low redraw rate when idle (2 FPS active, 1 FPS inactive)
- Minimal CPU usage between input events
- Scheduled repaints for live terminal output

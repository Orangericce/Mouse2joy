# Development Tasks

## Overview
This file tracks the work items needed to deliver the full mouse2joy application. It serves as a rough roadmap and can be turned into GitHub issues.

## Phase 1 – Core MVP (Completed)

| Task | Status | Description |
|------|--------|-------------|
| `Cargo.toml` & workspace layout | ✅ | Created basic Rust project with all required crates. |
| `src/main.rs` | ✅ | Entry point that loads config, spawns device watcher and runs the UI. |
| `src/config.rs` | ✅ | TOML config management (load/save with debounce). |
| `src/state.rs` | ✅ | Shared `AppState` holding settings and runtime data. |
| `src/device_manager.rs` | ✅ | Stub for mouse device enumeration & hot‑plug (to be fleshed out). |
| `src/joystick_emulator.rs` | ✅ | Stub using `uinput` (to be fleshed out). |
| `src/audio.rs` | ✅ | Audio cue player for grab/release tones. |
| `src/ui.rs` | ✅ | High‑level Rune UI layout with sliders, checkboxes, import/export buttons and joystick indicator. |
| `README.md` | ✅ | Comprehensive documentation with install scripts, usage, config format and a roadmap. |
| `install.sh` / `uninstall.sh` | ✅ | One‑liner install/uninstall scripts (desktop entry + optional systemd service). |

## Phase 2 – Device & Input Layer (Completed)

| Task | Status | Description |
|------|--------|-------------|
| `udev` event listener | ✅ | Watch for mouse plug‑unplug events and refresh the device list. |
| `evdev` device opening | ✅ | Open selected mouse device and read motion events in a dedicated async task. |
| Axis scaling & dead‑zone | ✅ | Apply configured sensitivities and dead‑zone to raw mouse deltas. |
| Virtual joystick creation | ✅ | Use `uinput` to create a Linux joystick device and emit axis events. |
| Trigger‑button mapping | ✅ | Listen globally for the user‑selected key/mouse button (via `evdev`/`input` events). |

## Phase 3 – UI Interaction & Settings

| Task | Status | Description |
|------|--------|-------------|
| Slider ↔ text‑box sync | ✅ | Keep sensitivity fields in sync when user edits either control. |
| Debounced config saving | ✅ |  Sawyer‑second timer that writes changes to TOML. |
| Import / Export dialogs | ✅ | Native file chooser for reading/writing `config.toml`. |
| System‑tray integration | ✅ | Hide to tray on close, restore from tray icon. |
| Audio toggle | ✅ | Respect `audio_enabled` flag when playing grab/release tones. |

## Phase 4 – Polish & Deployment

| Task | Status | Description |
|------|--------|-------------|
| AppImage/Flatpak packaging | ⏳ | Create an AppImage and/or Flatpak for easier distribution. |
| Wayland support | ⏳ | Implement Wayland-specific input handling (for non-X11 systems). |
| Multi‑device support | ⏳ | Add support for multiple concurrent mice. |
| Visual polish | ⏳ | Refine the joystick indicator, use a modern font, ensure contrast on black background. |
| Documentation | ⏳ | Expand the README with screenshots, FAQs, and a "troubleshooting" section. |
| CI/CD pipeline | ⏳ | Add GitHub Actions to run tests, lint, and publish releases. |

## Notation

- ✅ = Implemented (MVP or stub present)
- ⏳ = Work in progress / to be completed
- ❌ = Not started

Feel free to promote tasks to issues or pull requests as you work on them.

---

### Notes

- This task list is intentionally high‑level; individual implementation details (e.g., exact event handling loops) will be broken out into separate issues as needed.
- When a task is completed, update the status column to ✅ and optionally add a short note.
- The roadmap in `README.md` mirrors the high‑level phases above – they can be kept in sync as development progresses.

## Notation

- ✅ = Implemented (MVP or stub present)
- ⏳ = Work in progress / to be completed
- ❌ = Not started

Feel free to promote tasks to issues or pull requests as you work on them.

---

### Notes

- This task list is intentionally high‑level; individual implementation details (e.g., exact event handling loops) will be broken out into separate issues as needed.
- When a task is completed, update the status column to ✅ and optionally add a short note.
- The roadmap in `README.md` mirrors the high‑level phases above – they can be kept in sync as development progresses.
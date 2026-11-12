# mouse2joy

A lightweight, UI‑driven Rust application that turns mouse movement into a virtual joystick, built with the **Rune** UI framework. It is designed for Linux (including Arch Linux) and aims to use minimal system resources.

## Overview

- **Mouse → Joystick**: Any mouse motion is translated into joystick axis events.
- **Modern & Minimal UI**: A black‑themed interface with a centered joystick indicator, settings panel, and configurable trigger button.
- **System‑Tray Support**: Optionally close to tray while the background joystick emulation continues.
- **Audio cues**: Optional high‑pitch (grab) and low‑pitch (release) tones.
- **Persistent config**: Settings are saved after a 5‑second debounce.
- **Import / Export**: Save / load config files through the native file chooser.

## Features

- Choose a mouse device from a dropdown of detected mice.
- Adjust sensitivity, dead‑zone and other settings with sliders and editable text boxes.
- Assign a keyboard or mouse button as the trigger – clicking the picker puts Rune into "listening" mode and the next key press becomes the new trigger.
- Real‑time joystick indicator on the top‑left, with a grab/release LED above it.
- System‑tray toggle and audio toggle.
- Config is saved on any change (5‑second debounce), including the last selected device.
- Import and export config files using the system file manager.
- Black background for OLED screens.

## Installation & Build

### Build from source

```bash
cargo build --release
```

The binary will be created at `target/release/mouse2joy`.

### One‑liner Install Script

```bash
curl -sSf https://raw.githubusercontent.com/Orangericce/mouse2joy/main/install.sh | bash -s -- install
```

This script will:

1. Compile the project.
2. Copy the binary to `/usr/local/bin/mouse2joy`.
3. Install a desktop entry at `~/.local/share/applications/mouse2joy.desktop`.
4. Optionally enable a user‑systemd service for auto‑start.

### Uninstall Script

```bash
curl -sSf https://raw.githubusercontent.com/Orangericce/mouse2joy/main/uninstall.sh | bash -s -- uninstall
```

Removes the binary, desktop entry, and service.

### Manual install (no service)

```bash
cp target/release/mouse2joy ~/.local/bin/
chmod +x ~/.local/bin/mouse2joy
```

Create a desktop entry file at `~/.local/share/applications/mouse2joy.desktop`:

```ini
[Desktop Entry]
Name=mouse2joy
Comment=Turn mouse input into a joystick for games
Exec=mouse2joy
Icon=gamecontroller
Terminal=false
Type=Application
Categories=Game;Utility;
```


## Usage

1. **Run the app** (`mouse2joy`).
2. **Select a mouse** from the dropdown – the app will attempt to open the device.
3. **Adjust sensitivities** using the sliders on the right. Each slider has an editable text box that can be edited directly.
4. **Set a trigger button** – click the box below the joystick indicator, then press the desired keyboard/mouse button. The box shows the human‑readable name (e.g., "Left Mouse Button", "A").
5. **Grab / Release** – press the trigger button to start tracking mouse motion, release to stop.
6. **Audio & Tray** – enable/disable audio cues and choose to close to system tray.
7. **Import / Export** – export your current configuration or load a previously saved one.

### Configuration File Format

The configuration is stored at `~/.config/mouse2joy/config.toml`. Example:

```toml
device_path = "/dev/input/event5"
sensitivity_x = 1.2
sensitivity_y = 1.2
deadzone = 0.05
trigger_button = "BTN_LEFT"
minimize_to_tray = true
audio_enabled = true
```

All fields are optional – missing values fall back to defaults.

## Roadmap

- **Phase 1 (done)**: Core mouse‑to‑joystick translation and basic Rune UI layout.
- **Phase 2**: Implement device detection with `udev`, hot‑plug support, and robust input reading.
- **Phase 3**: Add trigger‑button “listening” mode with global key/mouse hook.
- **Phase 4**: Integrate audio‑cue controls and system‑tray functionality.
- **Phase 5**: Export to more formats (JSON, YAML), add support for multiple concurrent devices.
- **Phase 6**: Wayland support, packaging for distro (AppImage, Flatpak), UI theming.

## Troubleshooting

### Permissions

The app needs read/write access to `/dev/input/event*` devices. The easiest way is to add a `udev` rule:

```bash
sudo tee /etc/udev/rules.d/99-mouse2joy.rules << EOF
KERNEL=="event*", GROUP="input", MODE="0660"
EOF
sudo udevadm control --reload-rules
sudo usermod -aG input $USER
```

After adding the rule, log out and back in (or restart) so the group change takes effect.

### Missing Audio

If audio cues don’t play, verify that the sound card is available (`lsmod | grep snd`). On some minimal containers `rodio` may fallback to silence – you can disable audio cues in the UI.

### UI Not Responding

Rune UI runs on a dedicated thread; if you encounter freezes, try running with `RUST_LOG=debug` and file an issue.

## License

This project is licensed under the MIT License – see the `LICENSE` file for details.

---

*Generated with ❤️ by the mouse2joy team.*

## Quick Links

- Repository: https://github.com/Orangericce/mouse2joy
- Rune UI framework: https://github.com/anomalyco/rune

---

### Install Script (`install.sh`)

```bash
#!/usr/bin/env bash
set -e

cargo build --release
cp target/release/mouse2joy /usr/local/bin/

# Install desktop entry
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/mouse2joy.desktop << EOF
[Desktop Entry]
Name=mouse2joy
Comment=Turn mouse input into a joystick for games
Exec=mouse2joy
Icon=gamecontroller
Terminal=false
Type=Application
Categories=Game;Utility;
EOF

# Optional: install a user‑systemd service (for auto‑start)
SERVICE_DIR=~/.config/systemd/user
sudo mkdir -p $SERVICE_DIR
sudo cp mouse2joy.service $SERVICE_DIR/
sudo systemctl --user enable mouse2joy
sudo systemctl --user start mouse2joy

echo "Installation complete! Run 'mouse2joy' to start."
```

### Uninstall Script (`uninstall.sh`)

```bash
#!/usr/bin/env bash
set -e

# Remove binary
sudo rm -f /usr/local/bin/mouse2joy

# Remove desktop entry
sudo rm -f ~/.local/share/applications/mouse2joy.desktop

# Stop & disable user systemd service
sudo systemctl --user stop mouse2joy
sudo systemctl --user disable mouse2joy
sudo rm -f ~/.config/systemd/user/mouse2joy.service

sudo systemctl --user daemon-reload

echo "Uninstallation complete."
```
#!/usr/bin/env bash
set -e

cargo build --release

# Install binary
sudo cp target/release/mouse2joy /usr/local/bin/

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
sudo cp mouse2joy.service $SERVICE_DIR/ 2>/dev/null || echo "mouse2joy.service not found – skipping systemd integration."
if [ -f "$SERVICE_DIR/mouse2joy.service" ]; then
    sudo systemctl --user enable mouse2joy
    sudo systemctl --user start mouse2joy
fi

echo "Installation complete! Run 'mouse2joy' to start."

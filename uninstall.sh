#!/usr/bin/env bash
set -e

# Remove binary
sudo rm -f /usr/local/bin/mouse2joy

# Remove desktop entry
sudo rm -f ~/.local/share/applications/mouse2joy.desktop

# Stop & disable user systemd service
sudo systemctl --user stop mouse2joy 2>/dev/null || true
sudo systemctl --user disable mouse2joy 2>/dev/null || true
sudo rm -f ~/.config/systemd/user/mouse2joy.service

sudo systemctl --user daemon-reload

echo "Uninstallation complete."

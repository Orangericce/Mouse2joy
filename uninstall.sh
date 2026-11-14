#!/usr/bin/env bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Remove binary
log_info "Removing binary from /usr/local/bin/mouse2joy"
sudo rm -f /usr/local/bin/mouse2joy

# Remove desktop entry
log_info "Removing desktop entry..."
sudo rm -f ~/.local/share/applications/mouse2joy.desktop

# Stop & disable user systemd service
log_info "Stopping and disabling systemd user service..."
systemctl --user stop mouse2joy 2>/dev/null || true
systemctl --user disable mouse2joy 2>/dev/null || true
rm -f ~/.config/systemd/user/mouse2joy.service
systemctl --user daemon-reload

# Remove config directory
log_info "Removing config directory..."
rm -rf ~/.config/mouse2joy

log_info "Uninstallation complete."
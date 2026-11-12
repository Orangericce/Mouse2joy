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

# Function to find the project root (where Cargo.toml lives)
# Returns empty string if not found
find_project_root() {
    local current_dir
    current_dir="$(pwd)"
    
    # If we're already in a git repository, use git root
    if command -v git >/dev/null 2>&1 && git rev-parse --git-dir >/dev/null 2>&1; then
        git rev-parse --show-toplevel
        return
    fi
    
    # Otherwise, search up parent directories for Cargo.toml
    local parent_dir
    parent_dir="$(dirname "$current_dir")"
    while [[ "$current_dir" != "/" ]]; do
        if [[ -f "$current_dir/Cargo.toml" ]]; then
            echo "$current_dir"
            return
        fi
        current_dir="$parent_dir"
        parent_dir="$(dirname "$current_dir")"
    done
    
    # Not found - return nothing
    return 1
}

# Find project root
PROJECT_ROOT="$(find_project_root || true)"

# If not in project, clone it
if [[ -z "$PROJECT_ROOT" ]] || [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
    log_info "Project not found locally. Cloning from GitHub..."
    TEMP_DIR="$(mktemp -d)"
    cd "$TEMP_DIR"
    git clone --depth 1 https://github.com/Orangericce/mouse2joy.git
    PROJECT_ROOT="$TEMP_DIR/mouse2joy"
    cd "$PROJECT_ROOT"
    log_info "Cloned to $PROJECT_ROOT"
else
    cd "$PROJECT_ROOT"
    log_info "Found project at $PROJECT_ROOT"
fi

# Build
log_info "Building mouse2joy (release)..."
cargo build --release || {
    log_error "cargo build failed. Please check your Rust installation."
    exit 1
}

# Install binary
log_info "Installing binary to /usr/local/bin/mouse2joy"
sudo cp target/release/mouse2joy /usr/local/bin/

# Install desktop entry
log_info "Installing desktop entry..."
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/mouse2joy.desktop << 'EOF'
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
if [[ -f "$PROJECT_ROOT/mouse2joy.service" ]]; then
    log_info "Installing systemd user service..."
    sudo mkdir -p "$SERVICE_DIR"
    sudo cp "$PROJECT_ROOT/mouse2joy.service" "$SERVICE_DIR/"
    if systemctl --user daemon-reload 2>/dev/null; then
        systemctl --user enable mouse2joy 2>/dev/null || true
        systemctl --user start mouse2joy 2>/dev/null || true
        log_info "Systemd service installed and started."
    else
        log_warn "Could not connect to user D-Bus. Service file installed but not enabled."
        log_warn "Run 'systemctl --user enable --now mouse2joy' manually after login."
    fi
else
    log_warn "mouse2joy.service not found in project - skipping systemd integration."
fi

log_info "Installation complete! Run 'mouse2joy' to start."
echo ""
echo "The binary is installed to /usr/local/bin/mouse2joy"
echo "The desktop entry is available in your application menu."
echo ""
echo "To uninstall: curl -sSf https://raw.githubusercontent.com/Orangericce/mouse2joy/main/uninstall.sh | bash -s -- uninstall"
#!/usr/bin/env bash
set -e

# Function to find the project root (where Cargo.toml lives)
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
    
    # Fallback to current directory
    echo "$(pwd)"
}

# Find and cd to the project root
PROJECT_ROOT="$(find_project_root)"
cd "$PROJECT_ROOT"

# Verify we're in the right place
cargo build --release || {
    echo "Error: cargo build failed. Are you in the mouse2joy project directory?"
    echo "Run: cd /home/orangericce/Documents/mouse2joy"
    exit 1
}

# Install binary
sudo cp target/release/mouse2joy /usr/local/bin/

# Install desktop entry
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
sudo mkdir -p $SERVICE_DIR
sudo cp mouse2joy.service $SERVICE_DIR/ 2>/dev/null || echo "mouse2joy.service not found – skipping systemd integration."
if [ -f "$SERVICE_DIR/mouse2joy.service" ]; then
    sudo systemctl --user enable mouse2joy
    sudo systemctl --user start mouse2joy
fi

echo "Installation complete! Run 'mouse2joy' to start."
echo "Note: The binary will be installed to /usr/local/bin/mouse2joy"
echo "The desktop entry will be available in your application menu."
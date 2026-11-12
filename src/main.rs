use std::sync::Arc;
use tokio::sync::Mutex;

mod config;
mod state;
mod device_manager;
mod joystick_emulator;
mod audio;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise logger
    env_logger::init();

    // Load configuration (creates default if missing)
    let cfg = config::load_config()?;

    // Shared application state
    let app_state = Arc::new(Mutex::new(state::AppState::new(cfg)));

    // Start device monitoring (runs in background)
    let dm_state = app_state.clone();
    tokio::spawn(async move {
        device_manager::run_device_watcher(dm_state).await;
    });

    // Extract initial state for UI (while we have a runtime)
    let initial_state = {
        let state = app_state.lock().await;
        state.clone()
    };

    // Start the UI (Rune) - runs synchronously, blocks this thread
    ui::run_ui(initial_state)?;

    Ok(())
}

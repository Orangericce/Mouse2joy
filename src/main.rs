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

    // Start joystick emulator (runs in background)
    let js_state = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = joystick_emulator::run_joystick_emulator(js_state).await {
            log::error!("Joystick emulator error: {}", e);
        }
    });

    // Start trigger listener (runs in background)
    let trigger_state = app_state.clone();
    tokio::spawn(async move {
        device_manager::run_trigger_listener(trigger_state).await;
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

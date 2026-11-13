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

    // Start debounced config saver
    let config_saver_tx = config::create_debounced_config_saver();
    let saver_state = app_state.clone();
    tokio::spawn(async move {
        // Periodically send current config to saver
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let state = saver_state.lock().await;
            let _ = config_saver_tx.send(state.to_config());
        }
    });

    // Start device monitoring (runs in background)
    let dm_state = app_state.clone();
    tokio::spawn(async move {
        device_manager::run_device_watcher(dm_state).await;
    });

    // Start joystick emulator (runs in background)
    let js_state = app_state.clone();
    let visual = ui::shared_visual();
    tokio::spawn(async move {
        if let Err(e) = joystick_emulator::run_joystick_emulator(js_state, &visual).await {
            log::error!("Joystick emulator error: {}", e);
        }
    });

    // Extract initial state for UI (while we have a runtime)
    let initial_state = {
        let state = app_state.lock().await;
        state.clone()
    };

    // Run UI with a channel to sync state back
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<state::AppState>();
    
    // Spawn a task to receive UI state updates and apply to shared state
    let sync_state = app_state.clone();
    tokio::spawn(async move {
        while let Some(new_state) = ui_rx.recv().await {
            let mut state = sync_state.lock().await;
            // Preserve runtime state
            let grabbed = state.grabbed;
            let mouse_dx = state.mouse_dx;
            let mouse_dy = state.mouse_dy;
            *state = new_state;
            state.grabbed = grabbed;
            state.mouse_dx = mouse_dx;
            state.mouse_dy = mouse_dy;
            // Save config immediately whenever UI settings change
            let cfg = state.to_config();
            if let Err(e) = config::save_config(&cfg) {
                log::warn!("Failed to save config on change: {}", e);
            }
        }
    });

    // Start the UI (Rune) - runs synchronously, blocks this thread
    ui::run_ui(initial_state, ui_tx)?;

    // UI has exited - save config immediately (debounced saver may not have fired)
    let final_state = {
        let state = app_state.lock().await;
        state.clone()
    };
    if let Err(e) = config::save_config(&final_state.to_config()) {
        log::warn!("Failed to save config on exit: {}", e);
    } else {
        log::info!("Config saved on exit");
    }

    Ok(())
}

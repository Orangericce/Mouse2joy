use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run_ui(state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
    // Minimal placeholder UI that just prints a log entry
    log::info!("UI placeholder – Rune integration will be fleshed out later");

    // In a real implementation you would run `rune::App::new(...).run()` here.
    // For now, just wait for the state to be mutated.
    let _ = state;

    Ok(())
}

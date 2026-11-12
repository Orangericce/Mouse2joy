use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

/// Runs a background task that watches for mouse devices being added or removed.
/// For now this is a stub – the real implementation will use the `udev` crate
/// to listen for `add`/`remove` events and open the selected device with `evdev`.
pub async fn run_device_watcher(_state: Arc<Mutex<AppState>>) {
    loop {
        // Placeholder: just sleep and log.
        log::info!("Device watcher tick – future implementation will monitor udev events");
        sleep(Duration::from_secs(5)).await;
    }
}

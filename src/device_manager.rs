use crate::state::AppState;
use evdev::{Device, EventType, RelativeAxisCode};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use udev::{Enumerator, MonitorBuilder};

/// Runs a background task that watches for mouse devices being added or removed.
/// Uses `udev` to listen for `add`/`remove` events and `evdev` to read mouse motion.
pub async fn run_device_watcher(state: Arc<Mutex<AppState>>) {
    // Channel for device list updates to UI
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<String>>();

    // Initial device enumeration
    let devices = enumerate_mice();
    log::info!("Initial mice found: {:?}", devices);
    let _ = tx.send(devices.clone());

    // Update UI state with initial device list
    {
        let mut s = state.lock().await;
        // Device list is managed by UI state, but we track current selection
        if s.device_path.is_none() && !devices.is_empty() {
            s.device_path = Some(devices[0].clone());
            log::info!("Auto-selected first device: {}", devices[0]);
        }
    }

    // Spawn udev monitor task (runs in blocking thread due to udev types not being Send)
    let udev_tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = run_udev_monitor_blocking(udev_tx) {
            log::error!("udev monitor error: {}", e);
        }
    });

    // Spawn mouse reader task (reads from selected device)
    let reader_state = state.clone();
    tokio::spawn(async move {
        run_mouse_reader(reader_state).await;
    });

    // Handle device list updates from udev monitor
    while let Some(devices) = rx.recv().await {
        log::info!("Device list updated: {:?}", devices);
        // In a full implementation, we'd notify the UI here
        // For now, the UI polls enumerate_mice() on its own
    }
}

/// Enumerate available mice and trackpads from /dev/input/by-id
pub fn enumerate_mice() -> Vec<String> {
    let mut devices = Vec::new();
    let by_id = Path::new("/dev/input/by-id");
    if let Ok(entries) = fs::read_dir(by_id) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            // Filter for mice and trackpads (exclude keyboards)
            if (lower.contains("mouse") || lower.contains("trackpad") || lower.contains("touchpad"))
                && !lower.contains("kbd") {
                devices.push(format!("/dev/input/by-id/{}", name));
            }
        }
    }
    // Also check /dev/input for event* devices as fallback
    let by_path = Path::new("/dev/input");
    if let Ok(entries) = fs::read_dir(by_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("event") {
                let path = format!("/dev/input/{}", name);
                // Check if it's a mouse/trackpad by reading capabilities
                if is_mouse_device(&path) && !devices.iter().any(|d| same_device(d, &path)) {
                    devices.push(path);
                }
            }
        }
    }
    devices.sort();
    devices.dedup();
    devices
}

/// Check if a device path corresponds to a mouse or trackpad
fn is_mouse_device(path: &str) -> bool {
    if let Ok(device) = Device::open(path) {
        let caps = device.supported_events();
        // Check for relative axes (mouse movement)
        let has_rel = caps.contains(EventType::RELATIVE);
        // Check for absolute axes (touchpad/touchscreen)
        let has_abs = caps.contains(EventType::ABSOLUTE);
        // Check for keyboard keys (exclude keyboards)
        let has_keys = caps.contains(EventType::KEY);
        
        // Mouse: has relative axes, no keyboard keys
        let is_mouse = has_rel && !has_keys;
        
        // Touchpad: has absolute axes and has some keys
        // (exclude full keyboards by checking for limited keys)
        let is_touchpad = has_abs && has_keys;
        
        // For touchpad, check if it has standard touchpad axes (ABS_X, ABS_Y)
        // These are absolute axis codes 0 and 1 respectively
        if is_touchpad {
            // Try to check supported_absolute_axes if available
            if let Some(axes) = device.supported_absolute_axes() {
                use evdev::AbsoluteAxisCode;
                let has_x = axes.contains(AbsoluteAxisCode::ABS_X);
                let has_y = axes.contains(AbsoluteAxisCode::ABS_Y);
                if has_x && has_y {
                    return true;
                }
            }
        }
        
        is_mouse || is_touchpad
    } else {
        false
    }
}

/// Check if two device paths refer to the same physical device
fn same_device(a: &str, b: &str) -> bool {
    fs::read_link(a).ok() == fs::read_link(b).ok()
}

/// Run udev monitor for hot-plug events (blocking version for spawn_blocking)
fn run_udev_monitor_blocking(tx: mpsc::UnboundedSender<Vec<String>>) -> anyhow::Result<()> {
    let mut enumerator = Enumerator::new()?;
    enumerator.match_subsystem("input")?;
    enumerator.match_property("ID_INPUT_MOUSE", "1")?;

    let mut monitor = MonitorBuilder::new()?
        .match_subsystem("input")?
        .listen()?;

    let mut iter = monitor.iter();

    loop {
        // Poll for udev events (non-blocking)
        // SocketIter implements Iterator, so we can use next() directly
        if let Some(event) = iter.next() {
            log::debug!("udev event: {:?}", event);
            // Device changed - re-enumerate
            let devices = enumerate_mice();
            let _ = tx.send(devices);
        }
        
        // Small sleep to prevent busy loop
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Read mouse motion events from the selected device
async fn run_mouse_reader(state: Arc<Mutex<AppState>>) {
    let mut current_device: Option<String> = None;
    let mut device: Option<Device> = None;

    loop {
        // Check if device path changed
        let (new_path, grabbed, sensitivity_x, sensitivity_y, deadzone) = {
            let s = state.lock().await;
            (
                s.device_path.clone(),
                s.grabbed,
                s.sensitivity_x,
                s.sensitivity_y,
                s.deadzone,
            )
        };

        // Reopen device if path changed
        if new_path != current_device {
            current_device = new_path.clone();
            device = current_device.as_ref().and_then(|p| {
                match Device::open(p) {
                    Ok(d) => {
                        log::info!("Opened mouse device: {}", p);
                        Some(d)
                    }
                    Err(e) => {
                        log::warn!("Failed to open mouse device {}: {}", p, e);
                        None
                    }
                }
            });
        }

        // Read events if device is open and grabbed
        if let Some(ref mut dev) = device {
            if grabbed {
                // Try to grab the device exclusively
                let _ = dev.grab();

                // Read events (blocking - will wait for events)
                if let Ok(events) = dev.fetch_events() {
                    for event in events {
                        if event.event_type() == EventType::RELATIVE {
                            let code = RelativeAxisCode(event.code());
                            match code {
                                RelativeAxisCode::REL_X => {
                                    let mut s = state.lock().await;
                                    let scaled = apply_sensitivity_and_deadzone(event.value() as f32, sensitivity_x, deadzone);
                                    s.mouse_dx += scaled as i32;
                                }
                                RelativeAxisCode::REL_Y => {
                                    let mut s = state.lock().await;
                                    let scaled = apply_sensitivity_and_deadzone(event.value() as f32, sensitivity_y, deadzone);
                                    s.mouse_dy += scaled as i32;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } else {
                // Release grab if we had it
                let _ = dev.ungrab();
            }
        }

        sleep(Duration::from_millis(1)).await;
    }
}

/// Apply sensitivity and deadzone to a raw axis value
fn apply_sensitivity_and_deadzone(value: f32, sensitivity: f32, deadzone: f32) -> f32 {
    let abs_val = value.abs();
    if abs_val < deadzone {
        return 0.0;
    }
    // Scale the value beyond the deadzone
    let sign = if value > 0.0 { 1.0 } else { -1.0 };
    (abs_val - deadzone) * sensitivity * sign
}

/// Watch for trigger button press/release globally
pub async fn run_trigger_listener(_state: Arc<Mutex<AppState>>) {
    // This would require listening to all input devices for the trigger key
    // For now, we'll implement a simpler approach: the UI sets `grabbed` directly
    // when the trigger button is pressed in the UI.
    // A full implementation would use uinput or a global key hook.

    // We can use the trigger_button from config to know what to listen for
    loop {
        sleep(Duration::from_millis(10)).await;

        // Check if we need to update grabbed state from UI
        // The UI will set `grabbed = true` when trigger is pressed
        // and `grabbed = false` when released
    }
}
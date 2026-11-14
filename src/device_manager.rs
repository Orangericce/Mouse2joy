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
        let has_rel = caps.contains(EventType::RELATIVE);
        let has_abs = caps.contains(EventType::ABSOLUTE);
        let has_keys = caps.contains(EventType::KEY);
        
        // Mouse: has relative axes (specifically REL_X or REL_Y)
        // Mice have buttons (keys), so we can't use !has_keys
        let is_mouse = if has_rel {
            if let Some(axes) = device.supported_relative_axes() {
                use evdev::RelativeAxisCode;
                axes.contains(RelativeAxisCode::REL_X) || axes.contains(RelativeAxisCode::REL_Y)
            } else {
                false
            }
        } else {
            false
        };
        
        // Touchpad: has absolute axes (ABS_X, ABS_Y) and touch-related keys
        let is_touchpad = if has_abs && has_keys {
            if let Some(axes) = device.supported_absolute_axes() {
                use evdev::AbsoluteAxisCode;
                let has_x = axes.contains(AbsoluteAxisCode::ABS_X);
                let has_y = axes.contains(AbsoluteAxisCode::ABS_Y);
                // Also check for touch-specific keys to distinguish from tablets
                if let Some(keys) = device.supported_keys() {
                    use evdev::KeyCode;
                    let has_touch_keys = keys.contains(KeyCode::BTN_TOUCH) 
                        || keys.contains(KeyCode::BTN_TOOL_FINGER)
                        || keys.contains(KeyCode::BTN_TOOL_DOUBLETAP);
                    has_x && has_y && has_touch_keys
                } else {
                    has_x && has_y
                }
            } else {
                false
            }
        } else {
            false
        };
        
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
    // Match both mice and touchpads
    enumerator.match_property("ID_INPUT_MOUSE", "1")?;
    
    let monitor = MonitorBuilder::new()?
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

/// Read mouse motion events from the selected device.
/// Mirrors the Python reference: uses `mouse.grab()`/`mouse.ungrab()` to
/// exclusively capture the device when grabbed, and reads raw REL_X/REL_Y
/// events to drive the virtual joystick axes.
/// Always accumulates REL events for visual display; only emits to uinput when grabbed.
async fn run_mouse_reader(state: Arc<Mutex<AppState>>) {
    let mut current_device: Option<String> = None;
    let mut device: Option<Device> = None;

    loop {
        // Check if device path changed
        let (new_path, trigger_code, sensitivity_x, sensitivity_y, deadzone) = {
            let s = state.lock().await;
            (
                s.device_path.clone(),
                s.trigger_code,
                s.sensitivity_x,
                s.sensitivity_y,
                s.deadzone,
            )
        };

        // Reopen device if path changed
        if new_path != current_device {
            // Ungrab old device before dropping it
            if let Some(ref mut d) = device {
                let _ = d.ungrab();
            }
            // Reset grab state when device changes (can't grab new device yet)
            {
                let mut s = state.lock().await;
                if s.grabbed {
                    log::info!("Device changed while grabbed — resetting grab state");
                    s.grabbed = false;
                }
            }
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

        // Read events if device is open
        if let Some(ref mut dev) = device {
            // Drain all pending events into a Vec first, so we can call
            // dev.grab()/dev.ungrab() while iterating without overlapping
            // mutable borrows.
            let events: Vec<_> = dev
                .fetch_events()
                .map(|f| f.collect())
                .unwrap_or_default();
            for event in events {
                match event.event_type() {
                    EventType::RELATIVE => {
                        // Always accumulate REL events for visual display (so knob moves even when not grabbed).
                        // The joystick emulator will only apply them to the virtual device when grabbed.
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
                    EventType::KEY => {
                        let event_code = event.code();
                        
                        // Debug: log all key events
                        log::debug!("KEY event: code={}, value={}", event_code, event.value());
                        
                        // Check if UI is listening for a trigger button binding
                        let listening = {
                            let s = state.lock().await;
                            s.trigger_listening
                        };
                        
                        if listening && event.value() == 1 {
                            // Capture this key/button as the trigger binding - store raw code
                            // Reject left/right click
                            if event_code != 0x110 && event_code != 0x111 { // BTN_LEFT=0x110, BTN_RIGHT=0x111
                                let mut s = state.lock().await;
                                // Store raw code as source of truth
                                s.trigger_code = Some(event_code);
                                // Also store display name (via Debug format of KeyCode)
                                use evdev::KeyCode;
                                let key_code = KeyCode(event_code);
                                s.trigger_button = format!("{:?}", key_code);
                                s.trigger_listening = false;
                                log::info!("Trigger binding captured: {} (raw code {})", s.trigger_button, event_code);
                            } else {
                                log::info!("Rejected left/right click as trigger (code {})", event_code);
                            }
                            continue;
                        }
                        
                        // Normal trigger handling (toggle grab) - only on button PRESS (value == 1)
                        // Use stored trigger_code as source of truth
                        if let Some(target_code) = trigger_code {
                            log::debug!("Trigger check: target_code={}, event_code={}, value={}, match={}", 
                                target_code, event_code, event.value(), event_code == target_code && event.value() == 1);
                            if event_code == target_code && event.value() == 1 {
                                // Key down - toggle grabbed state
                                let mut s = state.lock().await;
                                let new_grabbed = !s.grabbed;
                                s.grabbed = new_grabbed;

                                if new_grabbed {
                                    if let Err(e) = dev.grab() {
                                        log::warn!("Failed to grab device: {}", e);
                                        // Revert grab state on failure
                                        s.grabbed = false;
                                    } else {
                                        log::info!("Grabbed device: {:?}", current_device);
                                    }
                                } else {
                                    let _ = dev.ungrab();
                                    log::info!("Ungrabbed device: {:?}", current_device);
                                }
                                log::info!("Trigger (code {}) toggled grabbed = {}", target_code, s.grabbed);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        sleep(Duration::from_millis(10)).await;
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
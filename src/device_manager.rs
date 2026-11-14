use crate::state::AppState;
use evdev::{Device, EventType, RelativeAxisCode, KeyCode};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use udev::{Enumerator, MonitorBuilder};

/// All bindable trigger names with their evdev key codes and display labels.
/// Order here determines display order in the trigger menu.
const TRIGGER_TABLE: &[(&str, u16, &str)] = &[
    ("BTN_SIDE", KeyCode::BTN_SIDE.code(), "Mouse Side (4)"),
    ("BTN_FORWARD", KeyCode::BTN_FORWARD.code(), "Mouse Forward (5)"),
    ("BTN_MIDDLE", KeyCode::BTN_MIDDLE.code(), "Middle Click"),
    ("BTN_EXTRA", KeyCode::BTN_EXTRA.code(), "Mouse Extra (6)"),
    ("BTN_TASK", KeyCode::BTN_TASK.code(), "Task Button"),
    ("SPACE", KeyCode::KEY_SPACE.code(), "Space"),
    ("ENTER", KeyCode::KEY_ENTER.code(), "Enter"),
    ("TAB", KeyCode::KEY_TAB.code(), "Tab"),
    ("SHIFT", KeyCode::KEY_LEFTSHIFT.code(), "Shift"),
    ("CTRL", KeyCode::KEY_LEFTCTRL.code(), "Ctrl"),
    ("ALT", KeyCode::KEY_LEFTALT.code(), "Alt"),
    ("CAPSLOCK", KeyCode::KEY_CAPSLOCK.code(), "CapsLock"),
    ("F1", KeyCode::KEY_F1.code(), "F1"),
    ("F2", KeyCode::KEY_F2.code(), "F2"),
    ("F3", KeyCode::KEY_F3.code(), "F3"),
    ("F4", KeyCode::KEY_F4.code(), "F4"),
    ("F5", KeyCode::KEY_F5.code(), "F5"),
    ("F6", KeyCode::KEY_F6.code(), "F6"),
    ("F7", KeyCode::KEY_F7.code(), "F7"),
    ("F8", KeyCode::KEY_F8.code(), "F8"),
    ("F9", KeyCode::KEY_F9.code(), "F9"),
    ("F10", KeyCode::KEY_F10.code(), "F10"),
    ("F11", KeyCode::KEY_F11.code(), "F11"),
    ("F12", KeyCode::KEY_F12.code(), "F12"),
];

/// Maps string representations of trigger buttons/keys to evdev key codes.
/// Rejects BTN_LEFT and BTN_RIGHT (returns None).
pub fn parse_trigger_key_code(name: &str) -> Option<u16> {
    let s = name.trim().to_uppercase();
    // Disallowed
    match s.as_str() {
        "NOT BOUND" | "" | "NONE" => return None,
        "BTN_LEFT" | "BTN_RIGHT" | "LEFT CLICK" | "RIGHT CLICK" | "LEFT" | "RIGHT" => return None,
        _ => {}
    }

    // Aliases -> canonical name
    let canonical = match s.as_str() {
        // Mouse button aliases
        "MOUSE 4" | "MOUSE4" | "SIDE" => "BTN_SIDE",
        "MOUSE 5" | "MOUSE5" | "FORWARD" => "BTN_FORWARD",
        "MIDDLE CLICK" | "WHEEL CLICK" | "MIDDLE" => "BTN_MIDDLE",
        "MOUSE 6" | "EXTRA" => "BTN_EXTRA",
        "TASK" => "BTN_TASK",
        // Keyboard aliases
        "SHIFT" | "LEFTSHIFT" => "KEY_LEFTSHIFT",
        "RIGHTSHIFT" => "KEY_RIGHTSHIFT",
        "CTRL" | "LEFTCTRL" => "KEY_LEFTCTRL",
        "RIGHTCTRL" => "KEY_RIGHTCTRL",
        "ALT" | "LEFTALT" => "KEY_LEFTALT",
        "RIGHTALT" => "KEY_RIGHTALT",
        "CAPSLOCK" => "KEY_CAPSLOCK",
        "SPACE" => "KEY_SPACE",
        "ENTER" | "RETURN" => "KEY_ENTER",
        "TAB" => "KEY_TAB",
        // Function keys
        "F1" => "KEY_F1", "F2" => "KEY_F2", "F3" => "KEY_F3", "F4" => "KEY_F4",
        "F5" => "KEY_F5", "F6" => "KEY_F6", "F7" => "KEY_F7", "F8" => "KEY_F8",
        "F9" => "KEY_F9", "F10" => "KEY_F10", "F11" => "KEY_F11", "F12" => "KEY_F12",
        // Letters A-Z
        "A" => "KEY_A", "B" => "KEY_B", "C" => "KEY_C", "D" => "KEY_D", "E" => "KEY_E",
        "F" => "KEY_F", "G" => "KEY_G", "H" => "KEY_H", "I" => "KEY_I", "J" => "KEY_J",
        "K" => "KEY_K", "L" => "KEY_L", "M" => "KEY_M", "N" => "KEY_N", "O" => "KEY_O",
        "P" => "KEY_P", "Q" => "KEY_Q", "R" => "KEY_R", "S" => "KEY_S", "T" => "KEY_T",
        "U" => "KEY_U", "V" => "KEY_V", "W" => "KEY_W", "X" => "KEY_X", "Y" => "KEY_Y",
        "Z" => "KEY_Z",
        // Numbers 0-9
        "0" => "KEY_0", "1" => "KEY_1", "2" => "KEY_2", "3" => "KEY_3", "4" => "KEY_4",
        "5" => "KEY_5", "6" => "KEY_6", "7" => "KEY_7", "8" => "KEY_8", "9" => "KEY_9",
        other => other,
    };

    // Look up canonical name in table
    TRIGGER_TABLE
        .iter()
        .find(|(n, _, _)| *n == canonical)
        .map(|(_, c, _)| *c)
}

/// Reverse lookup: evdev key code -> canonical trigger name (e.g., "BTN_SIDE").
/// Returns None if the code isn't in the bindable table.
pub fn key_code_to_name(code: u16) -> Option<&'static str> {
    TRIGGER_TABLE
        .iter()
        .find(|(_, c, _)| *c == code)
        .map(|(n, _, _)| *n)
}

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
/// Only processes REL events when grabbed (matching Python reference).
async fn run_mouse_reader(state: Arc<Mutex<AppState>>) {
    let mut current_device: Option<String> = None;
    let mut device: Option<Device> = None;

    loop {
        // Check if device path changed
        let (new_path, trigger_button, sensitivity_x, sensitivity_y, deadzone) = {
            let s = state.lock().await;
            (
                s.device_path.clone(),
                s.trigger_button.clone(),
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
                        // Only process REL events when grabbed (matching Python reference)
                        let grabbed = {
                            let s = state.lock().await;
                            s.grabbed
                        };
                        if !grabbed {
                            continue;
                        }
                        let code = RelativeAxisCode(event.code());
                        match code {
                            RelativeAxisCode::REL_X => {
                                let mut s = state.lock().await;
                                // Only accumulate the scaled value — do NOT also add raw event.value(),
                                // which would double-count movement.
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
                        
                        // Check if UI is listening for a trigger button binding
                        let listening = {
                            let s = state.lock().await;
                            s.trigger_listening
                        };
                        
                        if listening && event.value() == 1 {
                            // Capture this key/button as the trigger binding
                            if let Some(name) = key_code_to_name(event_code) {
                                // Reject left/right click
                                if name != "BTN_LEFT" && name != "BTN_RIGHT" {
                                    let mut s = state.lock().await;
                                    s.trigger_button = name.to_string();
                                    s.trigger_listening = false;
                                    log::info!("Trigger binding captured: {} (code {})", name, event_code);
                                } else {
                                    log::info!("Rejected left/right click as trigger (code {})", event_code);
                                }
                            } else {
                                log::debug!("Trigger listening: code {} not in bindable table", event_code);
                            }
                            continue;
                        }
                        
                        // Normal trigger handling (toggle grab) - only on button PRESS (value == 1)
                        if let Some(target_code) = parse_trigger_key_code(&trigger_button) {
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
                                log::info!("Trigger {} toggled grabbed = {}", trigger_button, s.grabbed);
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
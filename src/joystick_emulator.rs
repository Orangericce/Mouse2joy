use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Stub for joystick emulation using the `uinput` crate.
/// In the final version this will create a virtual joystick device and
/// write axis events based on the mouse deltas stored in `AppState`.
pub struct JoystickEmulator {
    // In a full implementation we would keep the uinput device handle here.
}

impl JoystickEmulator {
    pub fn new() -> Self {
        JoystickEmulator {}
    }

    /// Apply the current mouse deltas to the virtual joystick.
    pub async fn update(&self, _state: Arc<Mutex<AppState>>) {
        // Placeholder – actual implementation will translate deltas -> axis values.
    }
}

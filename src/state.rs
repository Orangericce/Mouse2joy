use crate::config::Config;

#[derive(Debug, Clone)]
pub struct AppState {
    // User‑adjustable settings
    pub device_path: Option<String>,
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
    pub deadzone: f32,
    pub trigger_button: String,
    pub minimize_to_tray: bool,
    pub grab_audio_enabled: bool,
    pub release_audio_enabled: bool,

    // Runtime state
    pub grabbed: bool,
    pub mouse_dx: i32,
    pub mouse_dy: i32,
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        AppState {
            device_path: cfg.device_path,
            sensitivity_x: cfg.sensitivity_x,
            sensitivity_y: cfg.sensitivity_y,
            deadzone: cfg.deadzone,
            trigger_button: cfg.trigger_button,
            minimize_to_tray: cfg.minimize_to_tray,
            grab_audio_enabled: cfg.grab_audio_enabled,
            release_audio_enabled: cfg.release_audio_enabled,
            grabbed: false,
            mouse_dx: 0,
            mouse_dy: 0,
        }
    }

    /// Convert current state to Config for saving
    pub fn to_config(&self) -> Config {
        Config {
            device_path: self.device_path.clone(),
            sensitivity_x: self.sensitivity_x,
            sensitivity_y: self.sensitivity_y,
            deadzone: self.deadzone,
            trigger_button: self.trigger_button.clone(),
            minimize_to_tray: self.minimize_to_tray,
            grab_audio_enabled: self.grab_audio_enabled,
            release_audio_enabled: self.release_audio_enabled,
        }
    }

    /// Reset motion deltas (called after they have been consumed)
    pub fn reset_mouse_deltas(&mut self) {
        self.mouse_dx = 0;
        self.mouse_dy = 0;
    }
}

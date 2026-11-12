use crate::config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct AppState {
    // User‑adjustable settings
    pub device_path: Option<String>,
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
    pub deadzone: f32,
    pub trigger_button: String,
    pub minimize_to_tray: bool,
    pub audio_enabled: bool,

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
            audio_enabled: cfg.audio_enabled,
            grabbed: false,
            mouse_dx: 0,
            mouse_dy: 0,
        }
    }
}

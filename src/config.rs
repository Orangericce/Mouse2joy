use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::sleep;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Config {
    pub device_path: Option<String>,
    #[serde(default = "default_sensitivity")]
    pub sensitivity_x: f32,
    #[serde(default = "default_sensitivity")]
    pub sensitivity_y: f32,
    #[serde(default = "default_deadzone")]
    pub deadzone: f32,
    #[serde(default = "default_trigger_button")]
    pub trigger_button: String,  // Display name (for backward compat)
    #[serde(default)]
    pub trigger_code: Option<u16>,  // Actual raw evdev code (source of truth)
    #[serde(default)]
    pub minimize_to_tray: bool,
    #[serde(default = "default_audio_enabled")]
    pub grab_audio_enabled: bool,
    #[serde(default = "default_audio_enabled")]
    pub release_audio_enabled: bool,
    // Spring-back centering (matches Python reference)
    #[serde(default = "default_center_strength_x")]
    pub center_strength_x: f32,
    #[serde(default = "default_center_strength_y")]
    pub center_strength_y: f32,
    #[serde(default = "default_center_while_grabbed")]
    pub center_while_grabbed: bool,
}

fn default_sensitivity() -> f32 { 1.0 }
fn default_deadzone() -> f32 { 0.05 }
fn default_trigger_button() -> String { "Not bound".to_string() }
fn default_audio_enabled() -> bool { true }
fn default_center_strength_x() -> f32 { 0.04 }  // Python: CENTER_STRENGTH_X = 0.04
fn default_center_strength_y() -> f32 { 0.02 }  // Python: CENTER_STRENGTH_Y = 0.02
fn default_center_while_grabbed() -> bool { true }  // Python: CENTER_WHILE_GRABBED = True

impl Default for Config {
    fn default() -> Self {
        Config {
            device_path: None,
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            deadzone: 0.05,
            trigger_button: "Not bound".to_string(),
            trigger_code: None,
            minimize_to_tray: false,
            grab_audio_enabled: true,
            release_audio_enabled: true,
            center_strength_x: 0.04,
            center_strength_y: 0.02,
            center_while_grabbed: true,
        }
    }
}

fn config_path() -> PathBuf {
    let mut dir = dirs::config_dir().expect("Unable to locate config directory");
    dir.push("mouse2joy");
    fs::create_dir_all(&dir).ok();
    dir.push("config.toml");
    dir
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = config_path();
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    } else {
        let cfg = Config::default();
        save_config(&cfg)?;
        Ok(cfg)
    }
}

pub fn save_config(cfg: &Config) -> anyhow::Result<()> {
    let toml_str = toml::to_string_pretty(cfg)?;
    let path = config_path();
    fs::write(path, toml_str)?;
    Ok(())
}

/// Returns a sender that can be used to request config saves with debouncing.
/// The returned receiver should be spawned as a background task.
pub fn create_debounced_config_saver() -> mpsc::UnboundedSender<Config> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Config>();
    
    tokio::spawn(async move {
        let mut last_saved: Option<Config> = None;
        let mut pending_config: Option<Config> = None;
        
        loop {
            // Wait for either a new config or timeout
            tokio::select! {
                Some(cfg) = rx.recv() => {
                    // New config received - update pending and reset timer
                    pending_config = Some(cfg);
                }
                _ = sleep(Duration::from_secs(5)), if pending_config.is_some() => {
                    // 5 seconds elapsed since last config change - save it
                    if let Some(cfg) = pending_config.take() {
                        if Some(&cfg) != last_saved.as_ref() {
                            if let Err(e) = save_config(&cfg) {
                                log::warn!("Failed to save config: {}", e);
                            } else {
                                log::debug!("Config saved (debounced)");
                                last_saved = Some(cfg);
                            }
                        }
                    }
                }
            }
        }
    });
    
    tx
}

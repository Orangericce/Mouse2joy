use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub trigger_button: String,
    #[serde(default)]
    pub minimize_to_tray: bool,
    #[serde(default = "default_audio_enabled")]
    pub audio_enabled: bool,
}

fn default_sensitivity() -> f32 { 1.0 }
fn default_deadzone() -> f32 { 0.05 }
fn default_trigger_button() -> String { "BTN_LEFT".to_string() }
fn default_audio_enabled() -> bool { true }

impl Default for Config {
    fn default() -> Self {
        Config {
            device_path: None,
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            deadzone: 0.05,
            trigger_button: "BTN_LEFT".to_string(),
            minimize_to_tray: false,
            audio_enabled: true,
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

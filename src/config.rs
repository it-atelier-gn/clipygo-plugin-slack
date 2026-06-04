use serde::{Deserialize, Serialize};

/// Stored at:
///   Windows : %APPDATA%\clipygo-plugin-slack\config.json
///   macOS   : ~/Library/Application Support/clipygo-plugin-slack/config.json
///   Linux   : ~/.config/clipygo-plugin-slack/config.json
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub bot_token: String,
}

pub fn config_path() -> std::path::PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clipygo-plugin-slack");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("config.json")
}

pub fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &Config) {
    if let Ok(data) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(config_path(), data);
    }
}

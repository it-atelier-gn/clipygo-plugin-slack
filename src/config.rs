use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub bot_token: String,
}

pub fn config_path() -> std::path::PathBuf {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join("config.json")
}

#[cfg(not(test))]
fn config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clipygo-plugin-slack")
}

#[cfg(test)]
fn config_dir() -> std::path::PathBuf {
    std::env::temp_dir()
        .join("clipygo-plugin-slack-tests")
        .join(format!(
            "{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_stays_out_of_user_config_dir() {
        if let Some(real) = dirs::config_dir() {
            assert!(!config_path().starts_with(real.join("clipygo-plugin-slack")));
        }
    }

    #[test]
    fn config_path_differs_per_thread() {
        let other = std::thread::spawn(config_path).join().unwrap();
        assert_ne!(config_path(), other);
    }
}

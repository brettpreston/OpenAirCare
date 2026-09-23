//! Persisted user settings: the last audiogram and adjustments, plus the
//! preferred device MAC. Stored as JSON under the XDG config directory.

use std::path::PathBuf;

use airpods_proto::hearing::{Adjustments, Audiogram};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub audiogram: Option<Audiogram>,
    pub adjustments: Option<Adjustments>,
    pub device_mac: Option<String>,
    pub auto_reconnect: Option<bool>,
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("openairaid");
    }
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("openairaid");
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("openairaid")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        match std::fs::read_to_string(settings_path()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                log::warn!("settings.json unreadable ({e}), starting fresh");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                log::warn!("cannot create {}: {e}", dir.display());
                return;
            }
        }
        match serde_json::to_string_pretty(self) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&path, s) {
                    log::warn!("cannot write {}: {e}", path.display());
                }
            }
            Err(e) => log::warn!("cannot serialize settings: {e}"),
        }
    }
}

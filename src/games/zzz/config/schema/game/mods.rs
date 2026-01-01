use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mods {
    /// Whether ZZMI modding is enabled
    pub enabled: bool,
    
    /// Path to user's mods folder (where character mods are placed)
    /// If empty, uses default: ~/.local/share/sleepy-launcher/zzmi/Mods
    pub mods_folder: PathBuf,
}

impl Default for Mods {
    fn default() -> Self {
        Self {
            enabled: false,
            mods_folder: PathBuf::new(), // Empty = use default
        }
    }
}

impl From<&JsonValue> for Mods {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            enabled: value.get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(default.enabled),

            mods_folder: value.get("mods_folder")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or(default.mods_folder),
        }
    }
}

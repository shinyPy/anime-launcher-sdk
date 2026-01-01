use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mods {
    /// Whether ZZMI modding is enabled
    pub enabled: bool,
    
    /// Path to ZZMI config folder (containing d3dx.ini, Core/, Mods/, etc.)
    pub zzmi_path: PathBuf,
}

impl Default for Mods {
    fn default() -> Self {
        Self {
            enabled: false,
            zzmi_path: PathBuf::new(),
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

            zzmi_path: value.get("zzmi_path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or(default.zzmi_path),
        }
    }
}

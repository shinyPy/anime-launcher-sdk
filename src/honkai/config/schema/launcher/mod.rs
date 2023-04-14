use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

#[cfg(feature = "discord-rpc")]
pub mod discord_rpc;

use crate::config::schema_blanks::prelude::*;
use crate::honkai::consts::launcher_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launcher {
    pub language: String,
    pub temp: Option<PathBuf>,
    pub repairer: Repairer,

    #[cfg(feature = "discord-rpc")]
    pub discord_rpc: discord_rpc::DiscordRpc
}

impl Default for Launcher {
    #[inline]
    fn default() -> Self {
        Self {
            language: String::from("en-us"),
            temp: launcher_dir().ok(),
            repairer: Repairer::default(),

            #[cfg(feature = "discord-rpc")]
            discord_rpc: discord_rpc::DiscordRpc::default()
        }
    }
}

impl From<&JsonValue> for Launcher {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            language: match value.get("language") {
                Some(value) => value.as_str().unwrap_or(&default.language).to_string(),
                None => default.language
            },

            temp: match value.get("temp") {
                Some(value) => {
                    if value.is_null() {
                        None
                    } else {
                        match value.as_str() {
                            Some(value) => Some(PathBuf::from(value)),
                            None => default.temp
                        }
                    }
                },
                None => default.temp
            },

            repairer: match value.get("repairer") {
                Some(value) => Repairer::from(value),
                None => default.repairer
            },

            #[cfg(feature = "discord-rpc")]
            discord_rpc: match value.get("discord_rpc") {
                Some(value) => discord_rpc::DiscordRpc::from(value),
                None => default.discord_rpc
            },
        }
    }
}

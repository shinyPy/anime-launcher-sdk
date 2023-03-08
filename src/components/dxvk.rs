use std::path::PathBuf;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;
use wincompatlib::prelude::*;

use super::loader::ComponentsLoader;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub title: String,
    pub features: Features,
    pub versions: Vec<Version>
}

impl Group {
    /// Find dxvk group with given name in components index
    /// 
    /// This method will also check all version names within this group, so both `vanilla` and `dxvk-1.10.3` will work
    pub fn find_in<T: Into<PathBuf>, F: AsRef<str>>(components: T, name: F) -> anyhow::Result<Option<Self>> {
        let name = name.as_ref();

        for group in get_groups(components)? {
            if group.name == name || group.versions.iter().any(move |version| version.name == name) {
                return Ok(Some(group));
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Features {
    /// Standard environment variables that are applied when you launch the game
    /// 
    /// Available keywords:
    /// - `%build%` - path to wine build
    /// - `%prefix%` - path to wine prefix
    /// - `%temp%` - path to temp folder specified in config file
    /// - `%launcher%` - path to launcher folder
    /// - `%game%` - path to the game
    pub env: HashMap<String, String>
}

impl Default for Features {
    fn default() -> Self {
        Self {
            env: HashMap::new()
        }
    }
}

impl From<&JsonValue> for Features {
    fn from(value: &JsonValue) -> Self {
        let mut default = Self::default();

        Self {
            env: match value.get("env") {
                Some(value) => {
                    if let Some(object) = value.as_object() {
                        for (key, value) in object {
                            if let Some(value) = value.as_str() {
                                default.env.insert(key.to_string(), value.to_string());
                            } else {
                                default.env.insert(key.to_string(), value.to_string());
                            }
                        }
                    }

                    default.env
                },
                None => default.env
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub version: String,
    pub uri: String,
    pub features: Option<Features>
}

impl Version {
    /// Get latest recommended dxvk version
    pub fn latest<T: Into<PathBuf>>(components: T) -> anyhow::Result<Self> {
        Ok(get_groups(components)?[0].versions[0].clone())
    }

    /// Find dxvk version with given name in components index
    pub fn find_in<T: Into<PathBuf>, F: AsRef<str>>(components: T, name: F) -> anyhow::Result<Option<Self>> {
        let name = name.as_ref();

        for group in get_groups(components)? {
            if let Some(version) = group.versions.into_iter().find(move |version| version.name == name || version.version == name) {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Find dxvk group current version belongs to
    pub fn find_group<T: Into<PathBuf>>(&self, components: T) -> anyhow::Result<Option<Group>> {
        let name = self.name.as_str();

        for group in get_groups(components)? {
            if group.versions.iter().any(move |version| version.name == name || version.version == name) {
                return Ok(Some(group));
            }
        }

        Ok(None)
    }

    /// Check is current dxvk downloaded in specified folder
    #[inline]
    pub fn is_downloaded_in<T: Into<PathBuf>>(&self, folder: T) -> bool {
        folder.into().join(&self.name).exists()
    }

    /// Install current dxvk
    #[tracing::instrument(level = "debug", ret)]
    #[inline]
    pub fn install<T: Into<PathBuf> + std::fmt::Debug>(&self, dxvks_folder: T, wine: &Wine, params: InstallParams) -> std::io::Result<()> {
        tracing::debug!("Installing DXVK");

        Dxvk::install(
            wine,
            dxvks_folder.into().join(&self.name),
            params
        )
    }

    /// Uninstall current dxvk
    #[tracing::instrument(level = "debug", ret)]
    #[inline]
    pub fn uninstall(&self, wine: &Wine, params: InstallParams) -> std::io::Result<()> {
        tracing::debug!("Uninstalling DXVK");

        Dxvk::uninstall(
            wine,
            params
        )
    }
}

pub fn get_groups<T: Into<PathBuf>>(components: T) -> anyhow::Result<Vec<Group>> {
    ComponentsLoader::new(components).get_dxvk_versions()
}

/// List downloaded dxvk versions in some specific folder
pub fn get_downloaded<T: Into<PathBuf>>(components: T, folder: T) -> anyhow::Result<Vec<Group>> {
    let mut downloaded = Vec::new();

    let folder: PathBuf = folder.into();

    for mut group in get_groups(components)? {
        group.versions = group.versions.into_iter()
            .filter(|version| folder.join(&version.name).exists())
            .collect();

        if !group.versions.is_empty() {
            downloaded.push(group);
        }
    }

    Ok(downloaded)
}

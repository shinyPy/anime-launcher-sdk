//! ZZMI (Zenless Zone Zero Model Importer) support module
//! 
//! Downloads and manages 3DMigoto DLLs from XXMI-Libs-Package for mod support.

use std::fs::{self, File};
use std::io::{self, Read, Write, Cursor};
use std::path::{Path, PathBuf};

use crate::zzz::consts;

const XXMI_LIBS_API: &str = "https://api.github.com/repos/SpectrumQT/XXMI-Libs-Package/releases/latest";
const USER_AGENT: &str = "sleepy-launcher";

/// Information about the XXMI libs installation
#[derive(Debug, Clone)]
pub struct XxmiLibsInfo {
    pub version: String,
    pub path: PathBuf,
}

/// Fetches the latest XXMI-Libs-Package release info from GitHub
#[cfg(feature = "zzmi")]
fn fetch_latest_release() -> anyhow::Result<(String, String)> {
    use reqwest::blocking::Client;

    let client = Client::new();

    let response: serde_json::Value = client
        .get(XXMI_LIBS_API)
        .header("User-Agent", USER_AGENT)
        .send()?
        .json()?;

    let tag_name = response["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tag_name in release"))?
        .to_string();

    // Find the XXMI-PACKAGE zip asset
    let assets = response["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No assets in release"))?;

    let download_url = assets
        .iter()
        .find(|asset| {
            asset["name"]
                .as_str()
                .map(|n| n.starts_with("XXMI-PACKAGE") && n.ends_with(".zip"))
                .unwrap_or(false)
        })
        .and_then(|asset| asset["browser_download_url"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No XXMI-PACKAGE zip found in release"))?
        .to_string();

    Ok((tag_name, download_url))
}

/// Downloads a file from URL to the specified path
#[cfg(feature = "zzmi")]
fn download_file(url: &str, dest: &Path) -> anyhow::Result<()> {
    use reqwest::blocking::Client;

    tracing::info!("Downloading XXMI libs from {}", url);

    let client = Client::new();
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()?;

    let bytes = response.bytes()?;
    
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let mut file = File::create(dest)?;
    file.write_all(&bytes)?;

    Ok(())
}

/// Extracts a zip file to the specified directory
#[cfg(feature = "zzmi")]
fn extract_zip(zip_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    use zip::ZipArchive;

    tracing::info!("Extracting XXMI libs to {:?}", dest_dir);

    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    fs::create_dir_all(dest_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

/// Gets the ZZMI libs directory path
pub fn get_libs_dir() -> anyhow::Result<PathBuf> {
    Ok(consts::launcher_dir()?.join("zzmi").join("xxmi-libs"))
}

/// Gets the installed version of XXMI libs, if any
pub fn get_installed_version() -> anyhow::Result<Option<String>> {
    let libs_dir = get_libs_dir()?;
    let manifest_path = libs_dir.join("Manifest.json");

    if !manifest_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&content)?;

    Ok(manifest["version"].as_str().map(String::from))
}

/// Ensures XXMI libs are downloaded and up to date
/// Returns the path to the libs directory
#[cfg(feature = "zzmi")]
pub fn ensure_xxmi_libs() -> anyhow::Result<XxmiLibsInfo> {
    let libs_dir = get_libs_dir()?;
    
    // Check if we need to download/update
    let installed_version = get_installed_version().ok().flatten();
    let (latest_version, download_url) = fetch_latest_release()?;

    let needs_download = match &installed_version {
        Some(v) => v != &latest_version,
        None => true,
    };

    if needs_download {
        tracing::info!("Downloading XXMI libs {} (current: {:?})", latest_version, installed_version);

        let cache_dir = consts::launcher_dir()?.join("zzmi").join("cache");
        fs::create_dir_all(&cache_dir)?;

        let zip_path = cache_dir.join(format!("xxmi-libs-{}.zip", latest_version));

        // Download the zip
        download_file(&download_url, &zip_path)?;

        // Clear old libs and extract new ones
        if libs_dir.exists() {
            fs::remove_dir_all(&libs_dir)?;
        }

        extract_zip(&zip_path, &libs_dir)?;

        // Clean up zip file
        fs::remove_file(&zip_path)?;

        tracing::info!("XXMI libs {} installed successfully", latest_version);
    }

    Ok(XxmiLibsInfo {
        version: latest_version,
        path: libs_dir,
    })
}

/// Prepares ZZMI mods for game launch
/// - Copies DLLs to game directory (d3d11.dll -> dxgi.dll for DXVK compatibility)
/// - Symlinks ZZMI config folders
#[cfg(feature = "zzmi")]
pub fn prepare_mods(
    game_dir: &Path,
    libs_path: &Path,
    zzmi_config_path: &Path,
) -> anyhow::Result<()> {
    tracing::info!("Preparing ZZMI mods for {:?}", game_dir);

    // Copy DLLs from XXMI libs
    // d3d11.dll is renamed to dxgi.dll to avoid conflicts with DXVK
    let d3d11_src = libs_path.join("d3d11.dll");
    let dxgi_dst = game_dir.join("dxgi.dll");
    if d3d11_src.exists() {
        fs::copy(&d3d11_src, &dxgi_dst)?;
        tracing::debug!("Copied d3d11.dll -> dxgi.dll");
    }

    let d3dcompiler_src = libs_path.join("d3dcompiler_47.dll");
    let d3dcompiler_dst = game_dir.join("d3dcompiler_47.dll");
    if d3dcompiler_src.exists() {
        fs::copy(&d3dcompiler_src, &d3dcompiler_dst)?;
        tracing::debug!("Copied d3dcompiler_47.dll");
    }

    // Copy d3dx.ini from ZZMI config
    let ini_src = zzmi_config_path.join("d3dx.ini");
    let ini_dst = game_dir.join("d3dx.ini");
    if ini_src.exists() {
        fs::copy(&ini_src, &ini_dst)?;
        tracing::debug!("Copied d3dx.ini");
    }

    // Symlink folders from ZZMI config
    for folder in &["Core", "ShaderFixes", "Mods"] {
        let src = zzmi_config_path.join(folder);
        let dst = game_dir.join(folder);

        if src.is_dir() && !dst.exists() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&src, &dst)?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&src, &dst)?;

            tracing::debug!("Symlinked {}", folder);
        }
    }

    tracing::info!("ZZMI mods prepared successfully");
    Ok(())
}

/// Cleans up ZZMI mod files from game directory
#[cfg(feature = "zzmi")]
pub fn cleanup_mods(game_dir: &Path) -> anyhow::Result<()> {
    tracing::info!("Cleaning up ZZMI mods from {:?}", game_dir);

    // Remove DLLs
    for file in &["dxgi.dll", "d3dcompiler_47.dll", "d3dx.ini"] {
        let path = game_dir.join(file);
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }

    // Remove symlinks
    for folder in &["Core", "ShaderFixes", "Mods"] {
        let path = game_dir.join(folder);
        if path.is_symlink() {
            fs::remove_file(&path)?;  // Removes symlink, not target
        }
    }

    Ok(())
}

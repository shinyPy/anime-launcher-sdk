//! ZZMI (Zenless Zone Zero Model Importer) support module
//! 
//! Downloads and manages 3DMigoto components for mod support:
//! - XXMI-Libs-Package: DLLs (d3d11.dll, d3dcompiler_47.dll)
//! - ZZMI-Package: Config and scripts (d3dx.ini, Core/, ShaderFixes/)

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::zzz::consts;

const XXMI_LIBS_API: &str = "https://api.github.com/repos/SpectrumQT/XXMI-Libs-Package/releases/latest";
const ZZMI_PACKAGE_API: &str = "https://api.github.com/repos/leotorrez/ZZMI-Package/releases/latest";
const USER_AGENT: &str = "sleepy-launcher";

/// Information about the ZZMI installation
#[derive(Debug, Clone)]
pub struct ZzmiInfo {
    pub libs_version: String,
    pub zzmi_version: String,
    pub libs_path: PathBuf,
    pub zzmi_path: PathBuf,
}

/// Gets the base ZZMI directory in launcher folder
pub fn get_zzmi_base_dir() -> anyhow::Result<PathBuf> {
    Ok(consts::launcher_dir()?.join("zzmi"))
}

/// Gets the XXMI libs directory path
pub fn get_libs_dir() -> anyhow::Result<PathBuf> {
    Ok(get_zzmi_base_dir()?.join("xxmi-libs"))
}

/// Gets the ZZMI package directory path (d3dx.ini, Core/, ShaderFixes/)
pub fn get_zzmi_dir() -> anyhow::Result<PathBuf> {
    Ok(get_zzmi_base_dir()?.join("zzmi-package"))
}

/// Gets the default mods folder path
pub fn get_default_mods_dir() -> anyhow::Result<PathBuf> {
    Ok(get_zzmi_base_dir()?.join("Mods"))
}

/// Recursively finds a file by name in a directory
fn find_file_recursive(dir: &Path, filename: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    
    // Check current directory
    let direct_path = dir.join(filename);
    if direct_path.exists() {
        return Some(direct_path);
    }
    
    // Search subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_file_recursive(&path, filename) {
                    return Some(found);
                }
            }
        }
    }
    
    None
}

/// Recursively finds a directory by name
fn find_dir_recursive(dir: &Path, dirname: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    
    // Check current directory for the dirname
    let direct_path = dir.join(dirname);
    if direct_path.is_dir() {
        return Some(direct_path);
    }
    
    // Search subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_dir_recursive(&path, dirname) {
                    return Some(found);
                }
            }
        }
    }
    
    None
}

/// Fetches the latest release info from a GitHub repo
#[cfg(feature = "zzmi")]
fn fetch_github_release(api_url: &str, asset_prefix: &str) -> anyhow::Result<(String, String)> {
    use reqwest::blocking::Client;

    let client = Client::new();

    let response: serde_json::Value = client
        .get(api_url)
        .header("User-Agent", USER_AGENT)
        .send()?
        .json()?;

    let tag_name = response["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tag_name in release"))?
        .to_string();

    let assets = response["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No assets in release"))?;

    let download_url = assets
        .iter()
        .find(|asset| {
            asset["name"]
                .as_str()
                .map(|n| n.starts_with(asset_prefix) && n.ends_with(".zip"))
                .unwrap_or(false)
        })
        .and_then(|asset| asset["browser_download_url"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No {} zip found in release", asset_prefix))?
        .to_string();

    Ok((tag_name, download_url))
}

/// Downloads a file from URL to the specified path
#[cfg(feature = "zzmi")]
fn download_file(url: &str, dest: &Path) -> anyhow::Result<()> {
    use reqwest::blocking::Client;

    tracing::info!("Downloading from {}", url);

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

    tracing::info!("Extracting to {:?}", dest_dir);

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

/// Saves version info to a JSON file
#[cfg(feature = "zzmi")]
fn save_version(dir: &Path, version: &str) -> anyhow::Result<()> {
    let version_file = dir.join("version.json");
    let content = serde_json::json!({ "version": version });
    fs::write(&version_file, serde_json::to_string_pretty(&content)?)?;
    Ok(())
}

/// Reads version info from a JSON file
fn read_version(dir: &Path) -> Option<String> {
    let version_file = dir.join("version.json");
    if !version_file.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&version_file).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json["version"].as_str().map(String::from)
}

/// Ensures XXMI libs are downloaded and up to date
#[cfg(feature = "zzmi")]
pub fn ensure_xxmi_libs() -> anyhow::Result<(String, PathBuf)> {
    let libs_dir = get_libs_dir()?;
    
    let installed_version = read_version(&libs_dir);
    let (latest_version, download_url) = fetch_github_release(XXMI_LIBS_API, "XXMI-PACKAGE")?;

    let needs_download = installed_version.as_ref() != Some(&latest_version);

    if needs_download {
        tracing::info!("Downloading XXMI libs {} (current: {:?})", latest_version, installed_version);

        let cache_dir = get_zzmi_base_dir()?.join("cache");
        fs::create_dir_all(&cache_dir)?;

        let zip_path = cache_dir.join(format!("xxmi-libs-{}.zip", latest_version));

        download_file(&download_url, &zip_path)?;

        if libs_dir.exists() {
            fs::remove_dir_all(&libs_dir)?;
        }

        extract_zip(&zip_path, &libs_dir)?;
        save_version(&libs_dir, &latest_version)?;
        fs::remove_file(&zip_path)?;

        tracing::info!("XXMI libs {} installed successfully", latest_version);
    }

    Ok((latest_version, libs_dir))
}

/// Ensures ZZMI package is downloaded and up to date
#[cfg(feature = "zzmi")]
pub fn ensure_zzmi_package() -> anyhow::Result<(String, PathBuf)> {
    let zzmi_dir = get_zzmi_dir()?;
    
    let installed_version = read_version(&zzmi_dir);
    let (latest_version, download_url) = fetch_github_release(ZZMI_PACKAGE_API, "ZZMI")?;

    let needs_download = installed_version.as_ref() != Some(&latest_version);

    if needs_download {
        tracing::info!("Downloading ZZMI package {} (current: {:?})", latest_version, installed_version);

        let cache_dir = get_zzmi_base_dir()?.join("cache");
        fs::create_dir_all(&cache_dir)?;

        let zip_path = cache_dir.join(format!("zzmi-package-{}.zip", latest_version));

        download_file(&download_url, &zip_path)?;

        if zzmi_dir.exists() {
            fs::remove_dir_all(&zzmi_dir)?;
        }

        extract_zip(&zip_path, &zzmi_dir)?;
        save_version(&zzmi_dir, &latest_version)?;
        fs::remove_file(&zip_path)?;

        tracing::info!("ZZMI package {} installed successfully", latest_version);
    }

    Ok((latest_version, zzmi_dir))
}

/// Ensures all ZZMI components are downloaded
#[cfg(feature = "zzmi")]
pub fn ensure_all() -> anyhow::Result<ZzmiInfo> {
    let (libs_version, libs_path) = ensure_xxmi_libs()?;
    let (zzmi_version, zzmi_path) = ensure_zzmi_package()?;
    
    // Create default mods folder if it doesn't exist
    let default_mods = get_default_mods_dir()?;
    if !default_mods.exists() {
        fs::create_dir_all(&default_mods)?;
    }

    Ok(ZzmiInfo {
        libs_version,
        zzmi_version,
        libs_path,
        zzmi_path,
    })
}

/// Prepares ZZMI mods for game launch
/// - Copies DLLs to game directory (d3d11.dll -> dxgi.dll for DXVK compatibility)
/// - Copies/symlinks ZZMI config files
/// - Symlinks user's mods folder
#[cfg(feature = "zzmi")]
pub fn prepare_mods(game_dir: &Path, mods_folder: &Path) -> anyhow::Result<()> {
    tracing::info!("Preparing ZZMI mods for {:?}", game_dir);

    // First ensure everything is downloaded
    let info = ensure_all()?;
    
    tracing::info!("XXMI libs at: {:?}", info.libs_path);
    tracing::info!("ZZMI package at: {:?}", info.zzmi_path);

    // Find d3d11.dll recursively in the libs directory
    if let Some(d3d11_src) = find_file_recursive(&info.libs_path, "d3d11.dll") {
        let dxgi_dst = game_dir.join("dxgi.dll");
        fs::copy(&d3d11_src, &dxgi_dst)?;
        tracing::warn!("ZZMI: Copied {:?} -> {:?}", d3d11_src, dxgi_dst);
    } else {
        tracing::error!("d3d11.dll not found anywhere in {:?}", info.libs_path);
        anyhow::bail!("d3d11.dll not found in XXMI libs package");
    }

    // Find and copy d3dcompiler_47.dll
    if let Some(d3dcompiler_src) = find_file_recursive(&info.libs_path, "d3dcompiler_47.dll") {
        let d3dcompiler_dst = game_dir.join("d3dcompiler_47.dll");
        fs::copy(&d3dcompiler_src, &d3dcompiler_dst)?;
        tracing::warn!("ZZMI: Copied {:?} -> {:?}", d3dcompiler_src, d3dcompiler_dst);
    }

    // Find and copy nvapi64.dll
    if let Some(nvapi_src) = find_file_recursive(&info.libs_path, "nvapi64.dll") {
        let nvapi_dst = game_dir.join("nvapi64.dll");
        fs::copy(&nvapi_src, &nvapi_dst)?;
        tracing::warn!("ZZMI: Copied {:?} -> {:?}", nvapi_src, nvapi_dst);
    }

    // Find d3dx.ini recursively 
    let d3dx_ini = find_file_recursive(&info.zzmi_path, "d3dx.ini")
        .ok_or_else(|| anyhow::anyhow!("d3dx.ini not found in ZZMI package"))?;
    
    let zzmi_config_dir = d3dx_ini.parent()
        .ok_or_else(|| anyhow::anyhow!("Could not get parent directory of d3dx.ini"))?;
    
    tracing::warn!("ZZMI: Found config at: {:?}", zzmi_config_dir);

    // Copy d3dx.ini
    let ini_dst = game_dir.join("d3dx.ini");
    fs::copy(&d3dx_ini, &ini_dst)?;
    tracing::warn!("ZZMI: Copied d3dx.ini");

    // Symlink Core and ShaderFixes from ZZMI package
    for folder in &["Core", "ShaderFixes"] {
        let src = zzmi_config_dir.join(folder);
        let dst = game_dir.join(folder);

        // Remove existing symlink/folder if present
        if dst.exists() || dst.is_symlink() {
            if dst.is_symlink() {
                fs::remove_file(&dst)?;
            } else if dst.is_dir() {
                fs::remove_dir_all(&dst)?;
            }
        }

        if src.is_dir() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&src, &dst)?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&src, &dst)?;

            tracing::debug!("Symlinked {}", folder);
        }
    }

    // Symlink user's Mods folder
    let mods_dst = game_dir.join("Mods");
    if mods_dst.exists() || mods_dst.is_symlink() {
        if mods_dst.is_symlink() {
            fs::remove_file(&mods_dst)?;
        } else if mods_dst.is_dir() {
            fs::remove_dir_all(&mods_dst)?;
        }
    }

    // Create mods folder if it doesn't exist
    if !mods_folder.exists() {
        fs::create_dir_all(mods_folder)?;
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(mods_folder, &mods_dst)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(mods_folder, &mods_dst)?;

    tracing::debug!("Symlinked Mods folder to {:?}", mods_folder);

    tracing::info!("ZZMI mods prepared successfully");
    Ok(())
}

/// Cleans up ZZMI mod files from game directory
#[cfg(feature = "zzmi")]
pub fn cleanup_mods(game_dir: &Path) -> anyhow::Result<()> {
    tracing::info!("Cleaning up ZZMI mods from {:?}", game_dir);

    // Remove DLLs
    for file in &["dxgi.dll", "d3dcompiler_47.dll", "nvapi64.dll", "d3dx.ini", "d3d11.dll"] {
        let path = game_dir.join(file);
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }

    // Remove symlinks
    for folder in &["Core", "ShaderFixes", "Mods"] {
        let path = game_dir.join(folder);
        if path.is_symlink() {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

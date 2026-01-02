//! Steam integration module for adding the game as a non-Steam shortcut
//!
//! This module manages Steam shortcuts.vdf to add/remove the game from Steam.
//! The shortcut launches the launcher with --run-game flag to maintain all features.

use std::fs;
use std::path::PathBuf;

/// Steam user info with userdata directory
#[derive(Debug, Clone)]
pub struct SteamUser {
    pub user_id: String,
    pub userdata_path: PathBuf,
}

/// App name used in Steam shortcuts
pub const STEAM_APP_NAME: &str = "Zenless Zone Zero";

/// Tag used to identify our shortcut
pub const SHORTCUT_TAG: &str = "sleepy-launcher";

/// Get common Steam installation directories
fn get_steam_root_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    if let Some(home) = dirs::home_dir() {
        // Common Linux Steam paths
        paths.push(home.join(".steam/root"));
        paths.push(home.join(".steam/steam"));
        paths.push(home.join(".local/share/Steam"));
        
        // Flatpak Steam
        paths.push(home.join(".var/app/com.valvesoftware.Steam/.steam/root"));
        paths.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    
    // Filter to only existing paths
    paths.into_iter().filter(|p| p.exists()).collect()
}

/// Find all Steam userdata directories
pub fn find_steam_users() -> Vec<SteamUser> {
    let mut users = Vec::new();
    
    for steam_root in get_steam_root_paths() {
        let userdata_dir = steam_root.join("userdata");
        if !userdata_dir.exists() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(&userdata_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(user_id) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip non-numeric directories
                        if user_id.parse::<u64>().is_ok() {
                            let config_dir = path.join("config");
                            if config_dir.exists() {
                                users.push(SteamUser {
                                    user_id: user_id.to_string(),
                                    userdata_path: path.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicate by user_id (same user might appear in multiple Steam roots)
    users.sort_by(|a, b| a.user_id.cmp(&b.user_id));
    users.dedup_by(|a, b| a.user_id == b.user_id);
    
    users
}

/// Get the shortcuts.vdf path for a Steam user
pub fn get_shortcuts_path(user: &SteamUser) -> PathBuf {
    user.userdata_path.join("config/shortcuts.vdf")
}

/// Get the launcher executable path
fn get_launcher_exe() -> anyhow::Result<String> {
    if let Ok(exe) = std::env::current_exe() {
        return Ok(format!("\"{}\"", exe.to_string_lossy()));
    }
    
    anyhow::bail!("Could not determine launcher executable path")
}

/// Get the launcher directory
fn get_launcher_dir() -> anyhow::Result<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return Ok(format!("\"{}\"", parent.to_string_lossy()));
        }
    }
    
    anyhow::bail!("Could not determine launcher directory")
}

/// Check if our shortcut already exists in the shortcuts file
#[cfg(feature = "steam")]
pub fn is_shortcut_added(user: &SteamUser) -> bool {
    use steam_shortcuts_util::parse_shortcuts;
    
    let shortcuts_path = get_shortcuts_path(user);
    if !shortcuts_path.exists() {
        return false;
    }
    
    let Ok(content) = fs::read(&shortcuts_path) else {
        return false;
    };
    
    let Ok(shortcuts) = parse_shortcuts(&content) else {
        return false;
    };
    
    // Check if any shortcut has our app name and tag
    shortcuts.iter().any(|s| {
        s.app_name == STEAM_APP_NAME && s.tags.contains(&SHORTCUT_TAG)
    })
}

/// Add the game as a non-Steam shortcut
#[cfg(feature = "steam")]
pub fn add_shortcut(user: &SteamUser) -> anyhow::Result<()> {
    use steam_shortcuts_util::{parse_shortcuts, shortcuts_to_bytes, Shortcut, shortcut::ShortcutOwned, calculate_app_id_for_shortcut};
    
    let shortcuts_path = get_shortcuts_path(user);
    
    // Read existing shortcuts or start with empty vec
    let mut owned_shortcuts: Vec<ShortcutOwned> = if shortcuts_path.exists() {
        let content = fs::read(&shortcuts_path)?;
        parse_shortcuts(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse shortcuts: {}", e))?
            .into_iter()
            .map(|s| s.to_owned())
            .collect()
    } else {
        Vec::new()
    };
    
    // Check if already added
    if owned_shortcuts.iter().any(|s| s.app_name == STEAM_APP_NAME && s.tags.contains(&SHORTCUT_TAG.to_string())) {
        tracing::info!("Steam shortcut already exists for user {}", user.user_id);
        return Ok(());
    }
    
    // Get launcher paths
    let exe = get_launcher_exe()?;
    let start_dir = get_launcher_dir()?;
    
    // Create new shortcut as owned
    let mut new_shortcut = ShortcutOwned {
        order: owned_shortcuts.len().to_string(),
        app_id: 0, // Will be calculated below
        app_name: STEAM_APP_NAME.to_string(),
        exe,
        start_dir,
        icon: String::new(),
        shortcut_path: String::new(),
        launch_options: "--run-game".to_string(),
        is_hidden: false,
        allow_desktop_config: true,
        allow_overlay: true,
        open_vr: 0,
        dev_kit: 0,
        dev_kit_game_id: String::new(),
        dev_kit_overrite_app_id: 0,
        last_play_time: 0,
        tags: vec![
            SHORTCUT_TAG.to_string(),
            "Installed".to_string(),
            "Ready To Play".to_string(),
        ],
    };
    
    // Calculate app_id using the borrowed form
    {
        let borrowed = new_shortcut.borrow();
        new_shortcut.app_id = calculate_app_id_for_shortcut(&borrowed);
    }
    
    owned_shortcuts.push(new_shortcut);
    
    // Convert to borrowed for writing
    let borrowed_shortcuts: Vec<Shortcut> = owned_shortcuts.iter().map(|s| s.borrow()).collect();
    
    // Write back to file
    let bytes = shortcuts_to_bytes(&borrowed_shortcuts);
    
    // Ensure config directory exists
    if let Some(parent) = shortcuts_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(&shortcuts_path, bytes)?;
    
    tracing::info!("Added Steam shortcut for user {}", user.user_id);
    Ok(())
}

/// Remove the game shortcut from Steam
#[cfg(feature = "steam")]
pub fn remove_shortcut(user: &SteamUser) -> anyhow::Result<()> {
    use steam_shortcuts_util::{parse_shortcuts, shortcuts_to_bytes, shortcut::ShortcutOwned};
    
    let shortcuts_path = get_shortcuts_path(user);
    if !shortcuts_path.exists() {
        return Ok(());
    }
    
    let content = fs::read(&shortcuts_path)?;
    let shortcuts = parse_shortcuts(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse shortcuts: {}", e))?;
    
    // Filter out our shortcut, keep as owned
    let filtered: Vec<ShortcutOwned> = shortcuts
        .into_iter()
        .filter(|s| !(s.app_name == STEAM_APP_NAME && s.tags.contains(&SHORTCUT_TAG)))
        .map(|s| s.to_owned())
        .collect();
    
    // Convert to borrowed for writing
    let borrowed: Vec<_> = filtered.iter().map(|s| s.borrow()).collect();
    
    // Write back
    let bytes = shortcuts_to_bytes(&borrowed);
    fs::write(&shortcuts_path, bytes)?;
    
    tracing::info!("Removed Steam shortcut for user {}", user.user_id);
    Ok(())
}

/// Get list of Steam users that have our shortcut added
#[cfg(feature = "steam")]
pub fn get_users_with_shortcut() -> Vec<SteamUser> {
    find_steam_users()
        .into_iter()
        .filter(|u| is_shortcut_added(u))
        .collect()
}

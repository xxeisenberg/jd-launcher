use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use uuid::Uuid;

// Models

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InstalledModpackInfo {
    pub project_id: String,
    pub version_id: String,
    pub version_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Minecraft manifest URL
    pub version_url: String,
    /// "none" | "fabric" | "forge" | "neoforge"
    pub modloader: String,
    pub modloader_version: Option<String>,
    pub game_dir: String,
    pub java_path: Option<String>,
    pub jvm_args: String,
    pub resolution: Resolution,
    #[serde(default)]
    pub modpack_info: Option<InstalledModpackInfo>,
}

impl Profile {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        version_url: impl Into<String>,
    ) -> Self {
        let config = load_profiles();
        let settings = config.settings;
        let id = Uuid::new_v4().to_string();
        let default_game_dir =
            std::path::PathBuf::from(crate::helper::expand_path(&settings.game_root_directory))
                .join(format!("instances/{}", id))
                .to_string_lossy()
                .to_string();
        Profile {
            id,
            name: name.into(),
            version: version.into(),
            version_url: version_url.into(),
            modloader: "none".to_string(),
            modloader_version: None,
            game_dir: default_game_dir,
            java_path: settings.custom_java_path.clone(),
            jvm_args: settings.default_jvm_args.clone(),
            resolution: Resolution {
                width: settings.default_resolution_width,
                height: settings.default_resolution_height,
            },
            modpack_info: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LauncherSettings {
    pub theme: String,
    pub language: String,
    #[serde(default = "default_accent")]
    pub accent_color: String,
    #[serde(default = "default_font")]
    pub font_family: String,
    #[serde(default = "default_style")]
    pub ui_style: String,
    #[serde(default = "default_scale")]
    pub ui_scale: u32,
    pub close_on_launch: bool,
    pub default_resolution_width: u32,
    pub default_resolution_height: u32,
    pub fullscreen: bool,
    pub default_jvm_args: String,
    pub custom_java_path: Option<String>,
    pub game_root_directory: String,
    pub http_proxy: Option<String>,
    pub verbose_logging: bool,
    pub show_snapshots: bool,
    pub show_old_beta: bool,
    pub show_old_alpha: bool,
}

fn default_accent() -> String {
    "Blue".to_string()
}
fn default_font() -> String {
    "Geist".to_string()
}
fn default_style() -> String {
    "Vega".to_string()
}
fn default_scale() -> u32 {
    100
}

impl Default for LauncherSettings {
    fn default() -> Self {
        let base_dir = crate::helper::get_app_dir();
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            accent_color: "Blue".to_string(),
            font_family: "Geist".to_string(),
            ui_style: "Vega".to_string(),
            ui_scale: 100,
            close_on_launch: false,
            default_resolution_width: 854,
            default_resolution_height: 480,
            fullscreen: false,
            default_jvm_args: "-Xmx2G -Xms512M".to_string(),
            custom_java_path: None,
            game_root_directory: base_dir.to_string_lossy().to_string(),
            http_proxy: None,
            verbose_logging: false,
            show_snapshots: false,
            show_old_beta: false,
            show_old_alpha: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesConfig {
    pub profiles: Vec<Profile>,
    pub last_profile_id: Option<String>,
    #[serde(default)]
    pub settings: LauncherSettings,
}

// Storage

fn profiles_path() -> PathBuf {
    let base_dir = crate::helper::get_app_dir();

    base_dir.join("profiles.json")
}

pub fn load_profiles() -> ProfilesConfig {
    let path = profiles_path();
    if !path.exists() {
        return ProfilesConfig::default();
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return ProfilesConfig::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_profiles(config: &ProfilesConfig) -> Result<(), String> {
    let path = profiles_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

// Commands

#[tauri::command]
#[specta::specta]
pub fn list_profiles() -> Vec<Profile> {
    load_profiles().profiles
}

#[tauri::command]
#[specta::specta]
pub fn save_profile(profile: Profile) -> Result<(), String> {
    let mut config = load_profiles();
    if let Some(pos) = config.profiles.iter().position(|p| p.id == profile.id) {
        config.profiles[pos] = profile;
    } else {
        config.profiles.push(profile);
    }
    save_profiles(&config)
}

#[tauri::command]
#[specta::specta]
pub fn delete_profile(id: String, delete_folder: bool) -> Result<(), String> {
    let mut config = load_profiles();
    let profile = config.profiles.iter().find(|p| p.id == id).cloned();

    config.profiles.retain(|p| p.id != id);
    if config.last_profile_id.as_deref() == Some(&id) {
        config.last_profile_id = None;
    }

    if delete_folder {
        if let Some(p) = profile {
            let path = std::path::PathBuf::from(p.game_dir);
            if path.exists() {
                let _ = std::fs::remove_dir_all(path);
            }
        }
    }

    save_profiles(&config)
}

#[tauri::command]
#[specta::specta]
pub fn duplicate_profile(id: String) -> Result<Profile, String> {
    let mut config = load_profiles();
    let original = config
        .profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found", id))?;

    let new_id = Uuid::new_v4().to_string();
    let base_dir = crate::helper::get_app_dir();
    let new_game_dir = base_dir
        .join(format!("instances/{}", new_id))
        .to_string_lossy()
        .to_string();
    let dup = Profile {
        id: new_id,
        name: format!("{} (copy)", original.name),
        game_dir: new_game_dir,
        ..original
    };

    config.profiles.push(dup.clone());
    save_profiles(&config)?;
    Ok(dup)
}

#[tauri::command]
#[specta::specta]
pub fn get_last_profile_id() -> Option<String> {
    load_profiles().last_profile_id
}

#[tauri::command]
#[specta::specta]
pub fn set_last_profile_id(id: String) -> Result<(), String> {
    let mut config = load_profiles();
    config.last_profile_id = Some(id);
    save_profiles(&config)
}

#[tauri::command]
#[specta::specta]
pub fn get_settings() -> LauncherSettings {
    load_profiles().settings
}

#[tauri::command]
#[specta::specta]
pub fn update_settings(settings: LauncherSettings) -> Result<(), String> {
    let mut config = load_profiles();
    config.settings = settings;
    save_profiles(&config)
}

#[tauri::command]
#[specta::specta]
pub fn reset_settings() -> Result<(), String> {
    let mut config = load_profiles();
    config.settings = LauncherSettings::default();
    save_profiles(&config)
}

#[tauri::command]
#[specta::specta]
pub fn list_mods(profile_id: String) -> Result<Vec<String>, String> {
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    let mods_dir = PathBuf::from(&profile.game_dir).join("mods");
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let mut mods = Vec::new();
    for entry in std::fs::read_dir(mods_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "jar" {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        mods.push(name.to_string());
                    }
                }
            }
        }
    }

    // Sort mods alphabetically
    mods.sort();
    Ok(mods)
}

#[tauri::command]
#[specta::specta]
pub fn list_shaders(profile_id: String) -> Result<Vec<String>, String> {
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    let shaders_dir = PathBuf::from(&profile.game_dir).join("shaderpacks");
    if !shaders_dir.exists() {
        return Ok(Vec::new());
    }

    let mut shaders = Vec::new();
    for entry in std::fs::read_dir(shaders_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            // Shaders can be both zip files or directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') && !name.ends_with(".txt") {
                    shaders.push(name.to_string());
                }
            }
        }
    }

    // Sort shaders alphabetically
    shaders.sort();
    Ok(shaders)
}

#[tauri::command]
#[specta::specta]
pub fn list_resource_packs(profile_id: String) -> Result<Vec<String>, String> {
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    let rp_dir = PathBuf::from(&profile.game_dir).join("resourcepacks");
    if !rp_dir.exists() {
        return Ok(Vec::new());
    }

    let mut packs = Vec::new();
    for entry in std::fs::read_dir(rp_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') && !name.ends_with(".txt") {
                    packs.push(name.to_string());
                }
            }
        }
    }

    packs.sort();
    Ok(packs)
}

// Import Export

#[tauri::command]
#[specta::specta]
pub fn export_profile(id: String, dest_path: String) -> Result<(), String> {
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found", id))?;

    let file = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write profile metadata
    let profile_json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    zip.start_file("profile.json", options)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    zip.write_all(profile_json.as_bytes())
        .map_err(|e| e.to_string())?;

    // Write game dir
    let game_dir = PathBuf::from(&profile.game_dir);
    if game_dir.exists() {
        add_dir_to_zip(&mut zip, &game_dir, &game_dir, options)?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &PathBuf,
    dir: &PathBuf,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    use std::io::Write;
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
        let rel_str = format!("gameDir/{}", rel.to_string_lossy().replace('\\', "/"));

        if path.is_dir() {
            zip.add_directory(&rel_str, options)
                .map_err(|e| e.to_string())?;
            add_dir_to_zip(zip, base, &path, options)?;
        } else {
            zip.start_file(&rel_str, options)
                .map_err(|e| e.to_string())?;
            let data = std::fs::read(&path).map_err(|e| e.to_string())?;
            zip.write_all(&data).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn import_profile(zip_path: String) -> Result<Profile, String> {
    let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    // Read profile.json
    let profile_str = {
        let mut pf = archive
            .by_name("profile.json")
            .map_err(|_| "No profile.json found in ZIP".to_string())?;
        use std::io::Read;
        let mut s = String::new();
        pf.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };

    let mut profile: Profile = serde_json::from_str(&profile_str).map_err(|e| e.to_string())?;

    // Assign new id to avoid collision
    let new_id = Uuid::new_v4().to_string();
    let base_dir = crate::helper::get_app_dir();
    let new_game_dir = base_dir
        .join(format!("instances/{}", new_id))
        .to_string_lossy()
        .to_string();
    profile.id = new_id;
    profile.game_dir = new_game_dir.clone();

    // Extract gameDir contents
    let game_dir_path = PathBuf::from(&new_game_dir);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        if let Some(rel) = name.strip_prefix("gameDir/") {
            if rel.is_empty() {
                continue;
            }
            let target = game_dir_path.join(rel);
            if name.ends_with('/') {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            } else {
                if let Some(p) = target.parent() {
                    std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                let mut f = std::fs::File::create(&target).map_err(|e| e.to_string())?;
                use std::io::Read;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                use std::io::Write;
                f.write_all(&buf).map_err(|e| e.to_string())?;
            }
        }
    }

    // Persist
    let mut config = load_profiles();
    config.profiles.push(profile.clone());
    save_profiles(&config)?;
    Ok(profile)
}

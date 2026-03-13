use base64::Engine as _;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tauri_plugin_opener::{open_path, reveal_item_in_dir};
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
    pub group: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub modpack_info: Option<InstalledModpackInfo>,
}

impl Profile {
    #[allow(dead_code)]
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
            group: None,
            favorite: false,
            modpack_info: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InstalledMod {
    pub file_name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub icon_data_url: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InstalledContentPack {
    pub file_name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub icon_data_url: Option<String>,
    pub icon_url: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LocalWorld {
    pub folder_name: String,
    pub path: String,
    pub modified_ms: f64,
    pub size_bytes: f64,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LocalScreenshot {
    pub file_name: String,
    pub path: String,
    pub modified_ms: f64,
    pub size_bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LauncherSettings {
    pub theme: String,
    pub language: String,
    #[serde(default = "default_online")]
    pub online_mode: bool,
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
    #[serde(default)]
    pub groups: Vec<Group>,
}

fn default_online() -> bool {
    true
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
            online_mode: true,
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
            groups: Vec::new(),
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

#[derive(Default)]
struct ParsedModMetadata {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
    icon_path: Option<String>,
}

fn get_profile(profile_id: &str) -> Result<Profile, String> {
    load_profiles()
        .profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))
}

fn profile_subfolder(profile_id: &str, subfolder: &str) -> Result<PathBuf, String> {
    let profile = get_profile(profile_id)?;
    Ok(PathBuf::from(crate::helper::expand_path(&profile.game_dir)).join(subfolder))
}

fn file_modified_ms(metadata: &std::fs::Metadata) -> f64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as f64)
        .unwrap_or(0.0)
}

fn directory_size(path: &Path) -> f64 {
    let mut total = 0.0;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0.0;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                total += directory_size(&path);
            } else {
                total += metadata.len() as f64;
            }
        }
    }

    total
}

fn read_zip_entry_string<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    entry_name: &str,
) -> Option<String> {
    let mut file = archive.by_name(entry_name).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn manifest_version<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Option<String> {
    let manifest = read_zip_entry_string(archive, "META-INF/MANIFEST.MF")?;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Implementation-Version:") {
            let version = value.trim();
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
        if let Some(value) = trimmed.strip_prefix("Specification-Version:") {
            let version = value.trim();
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}

fn first_non_empty(values: &[Option<String>]) -> Option<String> {
    values
        .iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
        .cloned()
}

fn json_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        serde_json::Value::Number(n) => Some(n.to_string()),
        other => Some(other.to_string().trim_matches('"').to_string()),
    }
}

fn json_authors(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        serde_json::Value::Array(values) => {
            let authors = values
                .iter()
                .filter_map(|entry| match entry {
                    serde_json::Value::String(name) => Some(name.trim().to_string()),
                    serde_json::Value::Object(map) => map
                        .get("name")
                        .and_then(|name| name.as_str())
                        .map(|name| name.trim().to_string()),
                    _ => None,
                })
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>();
            (!authors.is_empty()).then(|| authors.join(", "))
        }
        _ => None,
    }
}

fn fabric_icon(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(path) => Some(path.clone()),
        serde_json::Value::Object(map) => map
            .iter()
            .filter_map(|(size, path)| Some((size.parse::<u32>().ok()?, path.as_str()?)))
            .max_by_key(|(size, _)| *size)
            .map(|(_, path)| path.to_string()),
        _ => None,
    }
}

fn toml_string(value: Option<&toml::Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn parse_fabric_metadata<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<ParsedModMetadata> {
    let raw = read_zip_entry_string(archive, "fabric.mod.json")?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    Some(ParsedModMetadata {
        name: first_non_empty(&[
            json_string(parsed.get("name")),
            json_string(parsed.get("id")),
        ]),
        version: json_string(parsed.get("version")),
        author: json_authors(parsed.get("authors")).or_else(|| json_authors(parsed.get("author"))),
        icon_path: fabric_icon(parsed.get("icon")),
    })
}

fn parse_mcmod_info_metadata<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<ParsedModMetadata> {
    let raw = read_zip_entry_string(archive, "mcmod.info")?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let entry = match parsed {
        serde_json::Value::Array(values) => values.into_iter().next()?,
        serde_json::Value::Object(map) => map.get("modList")?.as_array()?.first()?.clone(),
        _ => return None,
    };
    Some(ParsedModMetadata {
        name: first_non_empty(&[
            json_string(entry.get("name")),
            json_string(entry.get("modid")),
        ]),
        version: json_string(entry.get("version")),
        author: json_authors(entry.get("authorList")).or_else(|| json_string(entry.get("author"))),
        icon_path: json_string(entry.get("logoFile")),
    })
}

fn parse_mods_toml_metadata<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    jar_version: Option<&str>,
) -> Option<ParsedModMetadata> {
    let raw = read_zip_entry_string(archive, "META-INF/mods.toml")?;
    let parsed: toml::Value = toml::from_str(&raw).ok()?;
    let mods = parsed.get("mods")?.as_array()?;
    let entry = mods.first()?;
    let raw_version = toml_string(entry.get("version"));
    let version = match raw_version.as_deref() {
        Some("${file.jarVersion}") => jar_version.map(|value| value.to_string()),
        _ => raw_version,
    };

    Some(ParsedModMetadata {
        name: first_non_empty(&[
            toml_string(entry.get("displayName")),
            toml_string(entry.get("modId")),
        ]),
        version,
        author: toml_string(entry.get("authors")).or_else(|| toml_string(parsed.get("authors"))),
        icon_path: toml_string(entry.get("logoFile"))
            .or_else(|| toml_string(parsed.get("logoFile"))),
    })
}

fn icon_data_url<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    icon_path: &str,
) -> Option<String> {
    let normalized = icon_path.trim_start_matches('/');
    let extension = Path::new(normalized)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())?;

    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    };

    let mut file = archive.by_name(normalized).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

fn installed_mod_from_path(path: &Path) -> InstalledMod {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let enabled = !file_name.ends_with(".disabled");
    let fallback_name = file_name.trim_end_matches(".disabled").to_string();

    let mut display_name = fallback_name.clone();
    let mut version = None;
    let mut author = None;
    let mut icon_path = None;

    if let Ok(file) = File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            let jar_version = manifest_version(&mut archive);
            let metadata = parse_fabric_metadata(&mut archive)
                .or_else(|| parse_mcmod_info_metadata(&mut archive))
                .or_else(|| parse_mods_toml_metadata(&mut archive, jar_version.as_deref()));

            if let Some(metadata) = metadata {
                if let Some(name) = metadata.name.filter(|name| !name.trim().is_empty()) {
                    display_name = name;
                }
                version = metadata.version;
                author = metadata.author;
                icon_path = metadata.icon_path;
            }

            let icon_data_url = icon_path
                .as_deref()
                .and_then(|path| icon_data_url(&mut archive, path));

            return InstalledMod {
                file_name,
                display_name,
                version: version.or(jar_version),
                author,
                icon_data_url,
                enabled,
            };
        }
    }

    InstalledMod {
        file_name,
        display_name,
        version,
        author,
        icon_data_url: None,
        enabled,
    }
}

fn display_name_from_filename(name: &str) -> String {
    let stem = name.trim_end_matches(".disabled").trim_end_matches(".zip");
    stem.replace(['-', '_'], " ")
}

fn parse_pack_mcmeta(raw: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(raw).ok()?;
    let desc = parsed.get("pack")?.get("description")?;
    match desc {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            // JSON text component
            Some(desc.to_string())
        }
        _ => None,
    }
}

fn read_pack_png_data_url_from_zip<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<String> {
    let mut file = archive.by_name("pack.png").ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

fn read_pack_png_data_url_from_dir(dir: &Path) -> Option<String> {
    let icon_path = dir.join("pack.png");
    let bytes = std::fs::read(&icon_path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

struct ShadersProperties {
    name: Option<String>,
    description: Option<String>,
    author: Option<String>,
    version: Option<String>,
}

fn parse_shaders_properties(raw: &str) -> Option<ShadersProperties> {
    let mut props = ShadersProperties {
        name: None,
        description: None,
        author: None,
        version: None,
    };
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let value = v.trim().replace("\\n", "\n");

            if key == "pack.name" && !value.is_empty() {
                props.name = Some(value);
            } else if (key == "pack.description" || key == "description") && !value.is_empty() {
                props.description = Some(value);
            } else if key == "pack.author" && !value.is_empty() {
                props.author = Some(value);
            } else if key == "pack.version" && !value.is_empty() {
                props.version = Some(value);
            }
        }
    }

    if props.name.is_none()
        && props.description.is_none()
        && props.author.is_none()
        && props.version.is_none()
    {
        None
    } else {
        Some(props)
    }
}

fn read_modrinth_sidecar(path: &Path) -> Option<serde_json::Value> {
    let sidecar_name = format!("{}.modrinth.json", path.file_name()?.to_str()?);
    let sidecar_path = path.parent()?.join(sidecar_name);
    let raw = std::fs::read_to_string(sidecar_path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn content_pack_from_path(path: &Path) -> InstalledContentPack {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let enabled = !file_name.ends_with(".disabled");
    let mut display_name = display_name_from_filename(&file_name);

    let mut description = None;
    let mut author = None;
    let mut version = None;
    let mut icon_data_url = None;
    let mut icon_url = None;

    if path.is_dir() {
        // Read pack.mcmeta from directory
        let mcmeta_path = path.join("pack.mcmeta");
        if let Ok(raw) = std::fs::read_to_string(&mcmeta_path) {
            description = parse_pack_mcmeta(&raw);
        }

        let properties_path = path.join("shaders/shaders.properties");
        if let Ok(raw) = std::fs::read_to_string(&properties_path)
            .or_else(|_| std::fs::read_to_string(path.join("shaders.properties")))
        {
            if let Some(meta) = parse_shaders_properties(&raw) {
                if let Some(name) = meta.name {
                    display_name = name;
                }
                if let Some(desc) = meta.description {
                    description = Some(desc);
                }
                author = meta.author;
                version = meta.version;
            }
        }

        icon_data_url = read_pack_png_data_url_from_dir(path);
    } else if let Ok(file) = File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            // Read pack.mcmeta from zip
            if let Some(raw) = read_zip_entry_string(&mut archive, "pack.mcmeta") {
                if let Some(desc) = parse_pack_mcmeta(&raw) {
                    description = Some(desc);
                }
            }
            if let Some(raw) = read_zip_entry_string(&mut archive, "shaders/shaders.properties")
                .or_else(|| read_zip_entry_string(&mut archive, "shaders.properties"))
            {
                if let Some(meta) = parse_shaders_properties(&raw) {
                    if let Some(name) = meta.name {
                        display_name = name;
                    }
                    if let Some(desc) = meta.description {
                        description = Some(desc);
                    }
                    author = meta.author;
                    version = meta.version;
                }
            }
            icon_data_url = read_pack_png_data_url_from_zip(&mut archive);
        }
    }

    // Merge Modrinth sidecar as fallback
    if let Some(sidecar) = read_modrinth_sidecar(path) {
        if display_name == display_name_from_filename(&file_name) {
            if let Some(title) = sidecar.get("title").and_then(|v| v.as_str()) {
                if !title.is_empty() {
                    display_name = title.to_string();
                }
            }
        }
        if description.is_none() {
            description = sidecar
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
        }
        if author.is_none() {
            author = sidecar
                .get("author")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
        }
        if version.is_none() {
            version = sidecar
                .get("version_number")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
        }
        if icon_data_url.is_none() {
            icon_url = sidecar
                .get("icon_url")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
        }
    }

    InstalledContentPack {
        file_name,
        display_name,
        description,
        author,
        version,
        icon_data_url,
        icon_url,
        enabled,
    }
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
    let mut profile = profile;
    profile.group = profile.group.and_then(|group| {
        let trimmed = group.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });

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
            let expanded_path = crate::helper::expand_path(&p.game_dir);
            let path = std::path::PathBuf::from(expanded_path);
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
pub fn list_mods(profile_id: String) -> Result<Vec<InstalledMod>, String> {
    let mods_dir = profile_subfolder(&profile_id, "mods")?;
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let mut mods = Vec::new();
    for entry in std::fs::read_dir(mods_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            {
                if name.ends_with(".jar") || name.ends_with(".disabled") {
                    mods.push(installed_mod_from_path(&path));
                }
            }
        }
    }

    mods.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()))
    });
    Ok(mods)
}

#[tauri::command]
#[specta::specta]
pub fn list_shaders(profile_id: String) -> Result<Vec<InstalledContentPack>, String> {
    let shaders_dir = profile_subfolder(&profile_id, "shaderpacks")?;
    if !shaders_dir.exists() {
        return Ok(Vec::new());
    }

    let mut shaders = Vec::new();
    for entry in std::fs::read_dir(shaders_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.')
                    && !name.ends_with(".txt")
                    && !name.ends_with(".modrinth.json")
                {
                    shaders.push(content_pack_from_path(&path));
                }
            }
        }
    }

    shaders.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()))
    });
    Ok(shaders)
}

#[tauri::command]
#[specta::specta]
pub fn list_resource_packs(profile_id: String) -> Result<Vec<InstalledContentPack>, String> {
    let rp_dir = profile_subfolder(&profile_id, "resourcepacks")?;
    if !rp_dir.exists() {
        return Ok(Vec::new());
    }

    let mut packs = Vec::new();
    for entry in std::fs::read_dir(rp_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.')
                    && !name.ends_with(".txt")
                    && !name.ends_with(".modrinth.json")
                {
                    packs.push(content_pack_from_path(&path));
                }
            }
        }
    }

    packs.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()))
    });
    Ok(packs)
}

#[tauri::command]
#[specta::specta]
pub fn list_worlds(profile_id: String) -> Result<Vec<LocalWorld>, String> {
    let saves_dir = profile_subfolder(&profile_id, "saves")?;
    if !saves_dir.exists() {
        return Ok(Vec::new());
    }

    let mut worlds = Vec::new();
    for entry in std::fs::read_dir(saves_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        if metadata.is_dir() {
            let folder_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            if folder_name.starts_with('.') {
                continue;
            }

            worlds.push(LocalWorld {
                folder_name,
                path: path.to_string_lossy().to_string(),
                modified_ms: file_modified_ms(&metadata),
                size_bytes: directory_size(&path),
                icon_path: {
                    let icon_file = path.join("icon.png");
                    if icon_file.exists() {
                        Some(icon_file.to_string_lossy().to_string())
                    } else {
                        None
                    }
                },
            });
        }
    }

    worlds.sort_by(|a, b| {
        b.modified_ms
            .partial_cmp(&a.modified_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.folder_name
                    .to_lowercase()
                    .cmp(&b.folder_name.to_lowercase())
            })
    });
    Ok(worlds)
}

#[tauri::command]
#[specta::specta]
pub fn list_screenshots(profile_id: String) -> Result<Vec<LocalScreenshot>, String> {
    let screenshots_dir = profile_subfolder(&profile_id, "screenshots")?;
    if !screenshots_dir.exists() {
        return Ok(Vec::new());
    }

    let mut screenshots = Vec::new();
    for entry in std::fs::read_dir(screenshots_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        let is_image = matches!(
            extension.as_deref(),
            Some("png") | Some("jpg") | Some("jpeg") | Some("webp")
        );

        if is_image {
            screenshots.push(LocalScreenshot {
                file_name,
                path: path.to_string_lossy().to_string(),
                modified_ms: file_modified_ms(&metadata),
                size_bytes: metadata.len() as f64,
            });
        }
    }

    screenshots.sort_by(|a, b| {
        b.modified_ms
            .partial_cmp(&a.modified_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()))
    });
    Ok(screenshots)
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
    let expanded_path = crate::helper::expand_path(&profile.game_dir);
    let game_dir = PathBuf::from(expanded_path);
    if game_dir.exists() {
        add_dir_to_zip(&mut zip, &game_dir, &game_dir, options)?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn export_group(group_name: String, dest_dir: String) -> Result<(), String> {
    let config = load_profiles();
    let matching: Vec<&Profile> = config
        .profiles
        .iter()
        .filter(|p| p.group.as_deref() == Some(&group_name))
        .collect();

    if matching.is_empty() {
        return Err(format!("No profiles found in group '{}'", group_name));
    }

    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    for profile in matching {
        let safe_name = profile
            .name
            .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let dest_path = PathBuf::from(&dest_dir)
            .join(format!("{}.zip", safe_name))
            .to_string_lossy()
            .to_string();
        export_profile(profile.id.clone(), dest_path)?;
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_group(group_name: String, delete_folders: bool) -> Result<(), String> {
    let mut config = load_profiles();

    let ids_to_delete: Vec<String> = config
        .profiles
        .iter()
        .filter(|p| p.group.as_deref() == Some(&group_name))
        .map(|p| p.id.clone())
        .collect();

    for id in &ids_to_delete {
        if delete_folders {
            if let Some(p) = config.profiles.iter().find(|p| &p.id == id) {
                let expanded = crate::helper::expand_path(&p.game_dir);
                let path = std::path::PathBuf::from(expanded);
                if path.exists() {
                    let _ = std::fs::remove_dir_all(path);
                }
            }
        }
    }

    config.profiles.retain(|p| !ids_to_delete.contains(&p.id));
    if let Some(ref last) = config.last_profile_id {
        if ids_to_delete.contains(last) {
            config.last_profile_id = None;
        }
    }

    // Remove from settings groups
    config.settings.groups.retain(|g| g.name != group_name);

    save_profiles(&config)
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

#[tauri::command]
#[specta::specta]
pub fn toggle_content(
    profile_id: String,
    subfolder: String,
    current_name: String,
) -> Result<String, String> {
    let base_dir = profile_subfolder(&profile_id, &subfolder)?;
    if !base_dir.exists() {
        return Err(format!("Folder {} does not exist", subfolder));
    }

    let source = base_dir.join(&current_name);
    if !source.exists() {
        return Err(format!("File {} does not exist", current_name));
    }

    let is_disabled = current_name.ends_with(".disabled");
    let target_name = if is_disabled {
        current_name.strip_suffix(".disabled").unwrap().to_string()
    } else {
        format!("{}.disabled", current_name)
    };

    let target = base_dir.join(&target_name);
    std::fs::rename(&source, &target).map_err(|e| format!("Failed to toggle file: {}", e))?;

    Ok(target_name)
}

#[tauri::command]
#[specta::specta]
pub fn delete_content(
    profile_id: String,
    subfolder: String,
    file_name: String,
) -> Result<(), String> {
    let target = profile_subfolder(&profile_id, &subfolder)?.join(&file_name);

    if !target.exists() {
        return Ok(());
    }

    if target.is_dir() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("Failed to delete directory: {}", e))?;
    } else {
        std::fs::remove_file(&target).map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn open_content_folder(profile_id: String, subfolder: String) -> Result<(), String> {
    let target = profile_subfolder(&profile_id, &subfolder)?;
    if !target.exists() {
        return Err(format!("Folder {} does not exist", subfolder));
    }

    open_path(target, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn open_content_entry(
    profile_id: String,
    subfolder: String,
    file_name: String,
) -> Result<(), String> {
    let target = profile_subfolder(&profile_id, &subfolder)?.join(&file_name);
    if !target.exists() {
        return Err(format!("Entry {} does not exist", file_name));
    }

    open_path(target, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn reveal_content_entry(
    profile_id: String,
    subfolder: String,
    file_name: String,
) -> Result<(), String> {
    let target = profile_subfolder(&profile_id, &subfolder)?.join(&file_name);
    if !target.exists() {
        return Err(format!("Entry {} does not exist", file_name));
    }

    reveal_item_in_dir(target).map_err(|e| e.to_string())
}

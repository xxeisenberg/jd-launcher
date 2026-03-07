use crate::profiles::{load_profiles, save_profiles, InstalledModpackInfo, Profile, Resolution};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use uuid::Uuid;

// Modrinth API types

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModpackSearchResult {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
    pub downloads: u32,
    pub author: String,
    pub categories: Vec<String>,
    pub latest_mc_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModpackVersion {
    pub version_id: String,
    pub name: String,
    pub version_number: String,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub download_url: String,
    pub file_size: u32,
    pub date_published: String,
}

// Modrinth search response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthHit>,
    total_hits: u32,
}

#[derive(Debug, Deserialize)]
struct ModrinthHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    downloads: u64,
    author: String,
    categories: Vec<String>,
    versions: Vec<String>,
}

// Modrinth version response
#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    id: String,
    name: String,
    version_number: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    files: Vec<ModrinthFile>,
    date_published: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModrinthFile {
    url: String,
    size: u64,
    primary: bool,
    filename: String,
}

// mrpack index types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MrpackIndex {
    name: String,
    dependencies: Option<HashMap<String, String>>,
    files: Vec<MrpackFile>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MrpackFile {
    path: String,
    hashes: MrpackHashes,
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MrpackHashes {
    sha1: Option<String>,
    sha512: Option<String>,
}

// Progress event
#[derive(Debug, Clone, Serialize)]
struct DownloadProgress {
    completed: usize,
    total: usize,
    phase: String,
}

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "JDLauncher/0.1.0 (github.com/jd-launcher)";

fn modrinth_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .unwrap_or_default()
}

fn urlencod(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

// Search modpacks
#[tauri::command]
#[specta::specta]
pub async fn search_modpacks(
    query: String,
    offset: u32,
) -> Result<Vec<ModpackSearchResult>, String> {
    let client = modrinth_client();
    let encoded_query = urlencod(&query);
    let encoded_facets = urlencod(r#"[["project_type:modpack"]]"#);
    let index = if query.is_empty() {
        "downloads"
    } else {
        "relevance"
    };
    let url = format!(
        "{}/search?query={}&facets={}&limit=20&offset={}&index={}",
        MODRINTH_API, encoded_query, encoded_facets, offset, index
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Modrinth API error: {}", resp.status()));
    }

    let data: ModrinthSearchResponse = resp
        .json::<ModrinthSearchResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let results = data
        .hits
        .into_iter()
        .map(|hit| {
            let latest_mc = hit.versions.last().cloned().unwrap_or_default();
            ModpackSearchResult {
                project_id: hit.project_id,
                slug: hit.slug,
                title: hit.title,
                description: hit.description,
                icon_url: hit.icon_url.unwrap_or_default(),
                downloads: hit.downloads as u32,
                author: hit.author,
                categories: hit.categories,
                latest_mc_version: latest_mc,
            }
        })
        .collect();

    Ok(results)
}

// Get versions for a modpack
#[tauri::command]
#[specta::specta]
pub async fn get_modpack_versions(project_id: String) -> Result<Vec<ModpackVersion>, String> {
    let client = modrinth_client();

    let url = format!("{}/project/{}/version", MODRINTH_API, project_id);

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Modrinth API error: {}", resp.status()));
    }

    let versions: Vec<ModrinthVersion> = resp
        .json::<Vec<ModrinthVersion>>()
        .await
        .map_err(|e| e.to_string())?;

    let results = versions
        .into_iter()
        .filter_map(|v| {
            let primary = v.files.iter().find(|f| f.primary).or(v.files.first())?;
            Some(ModpackVersion {
                version_id: v.id,
                name: v.name,
                version_number: v.version_number,
                mc_versions: v.game_versions,
                loaders: v.loaders,
                download_url: primary.url.clone(),
                file_size: primary.size as u32,
                date_published: v.date_published,
            })
        })
        .collect();

    Ok(results)
}

// Install a modpack
#[tauri::command]
#[specta::specta]
pub async fn install_modpack(
    app: AppHandle,
    project_id: String,
    version_id: String,
    modpack_name: String,
) -> Result<Profile, String> {
    let client = modrinth_client();

    // Fetch version info
    let ver_resp = client
        .get(format!("{}/version/{}", MODRINTH_API, version_id))
        .send()
        .await
        .map_err(|e| format!("Failed to get version: {}", e))?;

    let version: ModrinthVersion = ver_resp
        .json::<ModrinthVersion>()
        .await
        .map_err(|e| e.to_string())?;
    let primary_file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or(version.files.first())
        .ok_or("No files in version")?;

    // Emit progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed: 0,
            total: 1,
            phase: "modpack".into(),
        },
    );

    // Download mrpack
    let mrpack_bytes = client
        .get(&primary_file.url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read: {}", e))?;

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed: 1,
            total: 1,
            phase: "modpack".into(),
        },
    );

    // Parse mrpack ZIP
    let cursor = std::io::Cursor::new(&mrpack_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid mrpack: {}", e))?;

    // Read modrinth.index.json
    let index: MrpackIndex = {
        let mut index_file = archive
            .by_name("modrinth.index.json")
            .map_err(|_| "Missing modrinth.index.json")?;
        let mut buf = String::new();
        index_file
            .read_to_string(&mut buf)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&buf).map_err(|e| format!("Invalid index: {}", e))?
    };

    // Determine MC version and modloader
    let deps = index.dependencies.unwrap_or_default();
    let mc_version = deps.get("minecraft").cloned().unwrap_or_else(|| {
        version
            .game_versions
            .first()
            .cloned()
            .unwrap_or("1.20.1".into())
    });

    let (modloader, modloader_version) = if let Some(v) = deps.get("fabric-loader") {
        ("fabric".to_string(), Some(v.clone()))
    } else if let Some(v) = deps.get("forge") {
        ("forge".to_string(), Some(v.clone()))
    } else if let Some(v) = deps.get("neoforge") {
        ("neoforge".to_string(), Some(v.clone()))
    } else if let Some(v) = deps.get("quilt-loader") {
        ("quilt".to_string(), Some(v.clone()))
    } else {
        ("none".to_string(), None)
    };

    // Get version URL from manifest
    let version_url = get_mc_version_url(&mc_version).await?;

    // Create profile
    let config = load_profiles();
    let settings = config.settings;
    let profile_id = Uuid::new_v4().to_string();
    let game_dir =
        std::path::PathBuf::from(crate::helper::expand_path(&settings.game_root_directory))
            .join(format!("instances/{}", profile_id))
            .to_string_lossy()
            .to_string();

    std::fs::create_dir_all(&game_dir).map_err(|e| format!("Failed to create game dir: {}", e))?;

    // Download all files from index
    let file_count = index.files.len();
    if file_count > 0 {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                completed: 0,
                total: file_count,
                phase: "modpack_files".into(),
            },
        );
    }

    let sem = Arc::new(Semaphore::new(20));
    let completed = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for file in &index.files {
        if file.downloads.is_empty() {
            continue;
        }
        let url = file.downloads[0].clone();
        let target = std::path::PathBuf::from(&game_dir)
            .join(&file.path)
            .to_string_lossy()
            .to_string();
        let sha1 = file.hashes.sha1.clone().unwrap_or_default();
        let sem_c = Arc::clone(&sem);
        let counter = Arc::clone(&completed);
        let app_c = app.clone();

        let task = tokio::spawn(async move {
            let _permit = sem_c.acquire().await.unwrap();
            if let Err(e) = crate::helper::download_file_with_retry(&url, &target, 3, &sha1).await {
                eprintln!("Failed to download {}: {}", url, e);
            }
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_c.emit(
                "download-progress",
                DownloadProgress {
                    completed: done,
                    total: file_count,
                    phase: "modpack_files".into(),
                },
            );
        });
        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }

    // Extract overrides
    extract_overrides(&mrpack_bytes, &game_dir, "overrides")?;
    extract_overrides(&mrpack_bytes, &game_dir, "client-overrides")?;

    // Build profile
    let profile = Profile {
        id: profile_id,
        name: modpack_name,
        version: mc_version,
        version_url,
        modloader,
        modloader_version,
        game_dir,
        java_path: settings.custom_java_path.clone(),
        jvm_args: settings.default_jvm_args.clone(),
        resolution: Resolution {
            width: settings.default_resolution_width,
            height: settings.default_resolution_height,
        },
        modpack_info: Some(InstalledModpackInfo {
            project_id,
            version_id: version.id,
            version_name: version.version_number,
        }),
    };

    // Save
    let mut config = load_profiles();
    config.profiles.push(profile.clone());
    save_profiles(&config)?;

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed: 0,
            total: 0,
            phase: "done".into(),
        },
    );

    Ok(profile)
}

// Check for update
#[tauri::command]
#[specta::specta]
pub async fn check_modpack_update(profile_id: String) -> Result<Option<ModpackVersion>, String> {
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;

    let info = profile
        .modpack_info
        .as_ref()
        .ok_or("Not a modpack profile")?;

    let versions = get_modpack_versions(info.project_id.clone()).await?;
    if let Some(latest) = versions.first() {
        if latest.version_id != info.version_id {
            return Ok(Some(latest.clone()));
        }
    }
    Ok(None)
}

// Update modpack
#[tauri::command]
#[specta::specta]
pub async fn update_modpack(app: AppHandle, profile_id: String) -> Result<Profile, String> {
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .cloned()
        .ok_or("Profile not found")?;

    let info = profile
        .modpack_info
        .as_ref()
        .ok_or("Not a modpack profile")?;

    let versions = get_modpack_versions(info.project_id.clone()).await?;
    let latest = versions.first().ok_or("No versions found")?;

    if latest.version_id == info.version_id {
        return Err("Already up to date".into());
    }

    // Delete old profile, install new
    let mut config = load_profiles();
    config.profiles.retain(|p| p.id != profile_id);
    save_profiles(&config)?;

    // Clean old game dir
    let game_path = std::path::PathBuf::from(&profile.game_dir);
    if game_path.exists() {
        let _ = std::fs::remove_dir_all(&game_path);
    }

    install_modpack(
        app,
        info.project_id.clone(),
        latest.version_id.clone(),
        profile.name.clone(),
    )
    .await
}

// Search mods/shaders/resource packs
#[tauri::command]
#[specta::specta]
pub async fn search_modrinth(
    query: String,
    project_type: String,
    game_version: String,
    modloader: String,
    offset: u32,
) -> Result<Vec<ModpackSearchResult>, String> {
    let client = modrinth_client();
    let encoded_query = urlencod(&query);

    // Build facets array
    let mut facets = vec![format!("[\"project_type:{}\"]", project_type)];
    if !game_version.is_empty() {
        facets.push(format!("[\"versions:{}\"]", game_version));
    }
    if modloader != "none" && !modloader.is_empty() && project_type == "mod" {
        facets.push(format!("[\"categories:{}\"]", modloader));
    }
    let facets_str = format!("[{}]", facets.join(","));
    let encoded_facets = urlencod(&facets_str);

    let index = if query.is_empty() {
        "downloads"
    } else {
        "relevance"
    };
    let url = format!(
        "{}/search?query={}&facets={}&limit=20&offset={}&index={}",
        MODRINTH_API, encoded_query, encoded_facets, offset, index
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Modrinth API error: {}", resp.status()));
    }

    let data: ModrinthSearchResponse = resp
        .json::<ModrinthSearchResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let results = data
        .hits
        .into_iter()
        .map(|hit| {
            let latest_mc = hit.versions.last().cloned().unwrap_or_default();
            ModpackSearchResult {
                project_id: hit.project_id,
                slug: hit.slug,
                title: hit.title,
                description: hit.description,
                icon_url: hit.icon_url.unwrap_or_default(),
                downloads: hit.downloads as u32,
                author: hit.author,
                categories: hit.categories,
                latest_mc_version: latest_mc,
            }
        })
        .collect();

    Ok(results)
}

// Install a mod/shader/resource pack
#[tauri::command]
#[specta::specta]
pub async fn install_modrinth_content(
    project_id: String,
    version_id: String,
    game_dir: String,
    subfolder: String,
) -> Result<(), String> {
    let client = modrinth_client();

    let ver_resp = client
        .get(format!("{}/version/{}", MODRINTH_API, version_id))
        .send()
        .await
        .map_err(|e| format!("Failed to get version: {}", e))?;

    let version: ModrinthVersion = ver_resp
        .json::<ModrinthVersion>()
        .await
        .map_err(|e| e.to_string())?;

    let primary_file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or(version.files.first())
        .ok_or("No files in version")?;

    let expanded_game_dir = crate::helper::expand_path(&game_dir);
    let target_dir = std::path::PathBuf::from(expanded_game_dir).join(&subfolder);
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create dir: {}", e))?;

    let target_path = target_dir.join(&primary_file.filename);

    let bytes = client
        .get(&primary_file.url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Read failed: {}", e))?;

    std::fs::write(&target_path, &bytes).map_err(|e| format!("Write failed: {}", e))?;

    // suppress unused var warning
    let _ = project_id;

    Ok(())
}

// Helper: extract overrides from mrpack
fn extract_overrides(mrpack_bytes: &[u8], game_dir: &str, prefix: &str) -> Result<(), String> {
    let cursor = std::io::Cursor::new(mrpack_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    let prefix_slash = format!("{}/", prefix);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();

        if let Some(rel) = name.strip_prefix(&prefix_slash) {
            if rel.is_empty() {
                continue;
            }
            let target = std::path::PathBuf::from(game_dir).join(rel);

            if name.ends_with('/') {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            } else {
                if let Some(p) = target.parent() {
                    std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                std::fs::write(&target, &buf).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

// Helper: resolve MC version URL
async fn get_mc_version_url(mc_version: &str) -> Result<String, String> {
    let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let resp: serde_json::Value = reqwest::get(manifest_url)
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let versions = resp["versions"]
        .as_array()
        .ok_or("Invalid manifest format")?;

    for v in versions {
        if v["id"].as_str() == Some(mc_version) {
            return v["url"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or("Missing url field".into());
        }
    }

    Err(format!("MC version '{}' not found in manifest", mc_version))
}

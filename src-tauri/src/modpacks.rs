use crate::profiles::{load_profiles, modify_profiles, InstalledModpackInfo, Profile, Resolution};
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
    /// e.g. {"client": "required", "server": "unsupported"}
    env: Option<HashMap<String, String>>,
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
const USER_AGENT: &str = "xxeisenberg/jd-launcher (jaideep@null.net)";

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

/// Name of the file (inside an instance folder) that records which files the
/// installed modpack version put there, so an update can remove files the
/// new version dropped without touching anything the user added.
const MODPACK_MANIFEST: &str = ".jd-modpack-files.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct ModpackManifest {
    files: Vec<String>,
}

struct MrpackInstallResult {
    mc_version: String,
    version_url: String,
    modloader: String,
    modloader_version: Option<String>,
}

fn emit_progress(app: &AppHandle, completed: usize, total: usize, phase: &str) {
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed,
            total,
            phase: phase.into(),
        },
    );
}

async fn fetch_modrinth_version(
    client: &reqwest::Client,
    version_id: &str,
) -> Result<ModrinthVersion, String> {
    let resp = client
        .get(format!("{}/version/{}", MODRINTH_API, version_id))
        .send()
        .await
        .map_err(|e| format!("Failed to get version: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Modrinth API error: {}", resp.status()));
    }
    resp.json::<ModrinthVersion>()
        .await
        .map_err(|e| e.to_string())
}

async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} downloading {}", resp.status(), url));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read download: {}", e))
}

fn read_manifest(game_dir: &std::path::Path) -> Option<ModpackManifest> {
    let raw = std::fs::read_to_string(game_dir.join(MODPACK_MANIFEST)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Download a Modrinth modpack version and install it into `game_dir`.
/// Works both for a fresh folder and for updating an existing instance:
/// pack files are (re)written, overrides are applied, and files that the
/// previous pack version installed but this one no longer lists are removed.
/// User files (saves, screenshots, options, extra mods) are left alone.
async fn install_mrpack_into(
    app: &AppHandle,
    client: &reqwest::Client,
    version: &ModrinthVersion,
    game_dir: &std::path::Path,
) -> Result<MrpackInstallResult, String> {
    let primary_file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or(version.files.first())
        .ok_or("No files in version")?;

    emit_progress(app, 0, 1, "modpack");
    let mrpack_bytes = download_bytes(client, &primary_file.url).await?;
    emit_progress(app, 1, 1, "modpack");

    // Parse mrpack ZIP
    let index: MrpackIndex = {
        let cursor = std::io::Cursor::new(&mrpack_bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid mrpack: {}", e))?;
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
    let deps = index.dependencies.clone().unwrap_or_default();
    let mc_version = deps
        .get("minecraft")
        .cloned()
        .or_else(|| version.game_versions.first().cloned())
        .ok_or("Modpack does not specify a Minecraft version")?;

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

    let version_url = get_mc_version_url(&mc_version).await?;

    std::fs::create_dir_all(game_dir).map_err(|e| format!("Failed to create game dir: {}", e))?;

    // Validate every path up front, so a malicious pack writes nothing at all.
    let mut planned: Vec<(String, std::path::PathBuf, String, String)> = Vec::new();
    for file in &index.files {
        let client_unsupported = file
            .env
            .as_ref()
            .and_then(|env| env.get("client"))
            .is_some_and(|v| v == "unsupported");
        if client_unsupported {
            continue;
        }
        let url = file
            .downloads
            .first()
            .ok_or_else(|| format!("No download URL for {}", file.path))?
            .clone();
        let target = crate::helper::safe_join(game_dir, &file.path)
            .ok_or_else(|| format!("Modpack contains an unsafe file path: {}", file.path))?;
        let sha1 = file.hashes.sha1.clone().unwrap_or_default();
        planned.push((file.path.clone(), target, url, sha1));
    }

    let file_count = planned.len();
    if file_count > 0 {
        emit_progress(app, 0, file_count, "modpack_files");
    }

    let sem = Arc::new(Semaphore::new(20));
    let completed = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for (_, target, url, sha1) in &planned {
        let url = url.clone();
        let target = target.to_string_lossy().to_string();
        let sha1 = sha1.clone();
        let sem_c = Arc::clone(&sem);
        let counter = Arc::clone(&completed);
        let app_c = app.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem_c.acquire().await.map_err(|e| e.to_string())?;
            let result = crate::helper::download_file_with_retry(&url, &target, 3, &sha1).await;
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            emit_progress(&app_c, done, file_count, "modpack_files");
            result
        }));
    }

    let mut failures = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => failures.push(e),
            Err(e) => failures.push(format!("Download task crashed: {}", e)),
        }
    }
    if !failures.is_empty() {
        return Err(summarize_failures("modpack file", &failures));
    }

    // Overrides
    extract_overrides(&mrpack_bytes, game_dir, "overrides")?;
    extract_overrides(&mrpack_bytes, game_dir, "client-overrides")?;

    // Remove files the previous pack version installed that this one dropped.
    let new_files: Vec<String> = planned.into_iter().map(|(path, ..)| path).collect();
    if let Some(old) = read_manifest(game_dir) {
        for old_path in old.files.iter().filter(|p| !new_files.contains(p)) {
            if let Some(path) = crate::helper::safe_join(game_dir, old_path) {
                if path.is_file() {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
    let manifest = serde_json::to_string_pretty(&ModpackManifest { files: new_files })
        .map_err(|e| e.to_string())?;
    crate::helper::atomic_write(&game_dir.join(MODPACK_MANIFEST), manifest.as_bytes())?;

    Ok(MrpackInstallResult {
        mc_version,
        version_url,
        modloader,
        modloader_version,
    })
}

pub fn summarize_failures(what: &str, failures: &[String]) -> String {
    let shown: Vec<&str> = failures.iter().take(3).map(String::as_str).collect();
    let more = failures.len().saturating_sub(shown.len());
    let mut msg = format!(
        "{} {} download(s) failed. Check your connection and try again.\n{}",
        failures.len(),
        what,
        shown.join("\n")
    );
    if more > 0 {
        msg.push_str(&format!("\n…and {} more", more));
    }
    msg
}

#[tauri::command]
#[specta::specta]
pub async fn install_modpack(
    app: AppHandle,
    project_id: String,
    version_id: String,
    modpack_name: String,
) -> Result<Profile, String> {
    let result = install_modpack_inner(&app, project_id, version_id, modpack_name).await;
    // Always clear the progress bar, including on failure.
    emit_progress(&app, 0, 0, "done");
    result
}

async fn install_modpack_inner(
    app: &AppHandle,
    project_id: String,
    version_id: String,
    modpack_name: String,
) -> Result<Profile, String> {
    let client = modrinth_client();
    let version = fetch_modrinth_version(&client, &version_id).await?;

    let settings = load_profiles().settings;
    let profile_id = Uuid::new_v4().to_string();
    let game_dir =
        std::path::PathBuf::from(crate::helper::expand_path(&settings.game_root_directory))
            .join("instances")
            .join(&profile_id);

    let installed = match install_mrpack_into(app, &client, &version, &game_dir).await {
        Ok(installed) => installed,
        Err(e) => {
            // Brand-new folder that we created: safe to remove the partial install.
            let _ = std::fs::remove_dir_all(&game_dir);
            return Err(e);
        }
    };

    let profile = Profile {
        id: profile_id,
        name: modpack_name,
        version: installed.mc_version,
        version_url: installed.version_url,
        modloader: installed.modloader,
        modloader_version: installed.modloader_version,
        game_dir: game_dir.to_string_lossy().to_string(),
        java_path: settings.custom_java_path.clone(),
        jvm_args: settings.default_jvm_args.clone(),
        resolution: Resolution {
            width: settings.default_resolution_width,
            height: settings.default_resolution_height,
        },
        group: None,
        favorite: false,
        modpack_info: Some(InstalledModpackInfo {
            project_id,
            version_id: version.id,
            version_name: version.version_number,
        }),
    };

    modify_profiles(|config| {
        config.profiles.push(profile.clone());
        Ok(())
    })?;

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

// Update modpack in place. The instance keeps its id, folder, worlds,
// screenshots, options, group, favorite, Java and memory settings.
#[tauri::command]
#[specta::specta]
pub async fn update_modpack(app: AppHandle, profile_id: String) -> Result<Profile, String> {
    let result = update_modpack_inner(&app, profile_id).await;
    emit_progress(&app, 0, 0, "done");
    result
}

async fn update_modpack_inner(app: &AppHandle, profile_id: String) -> Result<Profile, String> {
    let profile = load_profiles()
        .profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;

    let info = profile
        .modpack_info
        .clone()
        .ok_or("Not a modpack profile")?;

    let versions = get_modpack_versions(info.project_id.clone()).await?;
    let latest = versions.first().ok_or("No versions found")?;

    if latest.version_id == info.version_id {
        return Err("Already up to date".into());
    }

    let client = modrinth_client();
    let version = fetch_modrinth_version(&client, &latest.version_id).await?;
    let game_dir = std::path::PathBuf::from(crate::helper::expand_path(&profile.game_dir));

    // Instances installed before the file manifest existed: we can't tell
    // pack mods from user mods, so move the old mods folder aside (never
    // delete it) to avoid duplicate/conflicting jars after the update.
    if read_manifest(&game_dir).is_none() {
        let mods = game_dir.join("mods");
        if mods.is_dir() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let backup_root = game_dir.join(".jd-backup");
            std::fs::create_dir_all(&backup_root).map_err(|e| e.to_string())?;
            std::fs::rename(&mods, backup_root.join(format!("mods-{}", stamp)))
                .map_err(|e| format!("Failed to back up old mods folder: {}", e))?;
        }
    }

    let installed = install_mrpack_into(app, &client, &version, &game_dir)
        .await
        .map_err(|e| {
            format!(
                "Update failed; the instance still points at the previous version. {}",
                e
            )
        })?;

    modify_profiles(|config| {
        let p = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or("Profile was removed during the update")?;
        p.version = installed.mc_version;
        p.version_url = installed.version_url;
        p.modloader = installed.modloader;
        p.modloader_version = installed.modloader_version;
        p.modpack_info = Some(InstalledModpackInfo {
            project_id: info.project_id.clone(),
            version_id: version.id.clone(),
            version_name: version.version_number.clone(),
        });
        Ok(p.clone())
    })
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

// Sidecar metadata saved alongside downloaded content
#[derive(Debug, Serialize, Deserialize)]
struct ContentSidecar {
    project_id: String,
    title: String,
    author: String,
    icon_url: String,
    version_name: String,
    version_number: String,
    description: String,
}

// Install a mod/shader/resource pack
#[tauri::command]
#[specta::specta]
pub async fn install_modrinth_content(
    project_id: String,
    version_id: String,
    game_dir: String,
    subfolder: String,
    // Metadata from search result
    project_title: String,
    project_author: String,
    project_icon_url: String,
    project_description: String,
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
    let target_dir = crate::helper::safe_join(std::path::Path::new(&expanded_game_dir), &subfolder)
        .ok_or_else(|| format!("Invalid content folder: {}", subfolder))?;
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create dir: {}", e))?;

    let target_path = crate::helper::safe_join(&target_dir, &primary_file.filename)
        .filter(|p| p.parent() == Some(target_dir.as_path()))
        .ok_or_else(|| format!("Invalid file name: {}", primary_file.filename))?;

    let bytes = client
        .get(&primary_file.url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Read failed: {}", e))?;

    std::fs::write(&target_path, &bytes).map_err(|e| format!("Write failed: {}", e))?;

    // Save sidecar metadata
    let sidecar = ContentSidecar {
        project_id,
        title: project_title,
        author: project_author,
        icon_url: project_icon_url,
        version_name: version.name,
        version_number: version.version_number,
        description: project_description,
    };
    let sidecar_path = target_dir.join(format!("{}.modrinth.json", primary_file.filename));
    if let Ok(json) = serde_json::to_string_pretty(&sidecar) {
        let _ = std::fs::write(&sidecar_path, json);
    }

    Ok(())
}

// Helper: extract overrides from mrpack
fn extract_overrides(
    mrpack_bytes: &[u8],
    game_dir: &std::path::Path,
    prefix: &str,
) -> Result<(), String> {
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
            // Reject zip-slip entries such as "overrides/../../evil".
            if file.enclosed_name().is_none() {
                return Err(format!("Modpack contains an unsafe path: {}", name));
            }
            let target = crate::helper::safe_join(game_dir, rel)
                .ok_or_else(|| format!("Modpack contains an unsafe path: {}", name))?;

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

#[cfg(test)]
mod tests {
    use super::extract_overrides;
    use std::io::Write;

    fn zip_with(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, body) in entries {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn overrides_extract_normally() {
        let dir = std::env::temp_dir().join(format!("jd-ovr-{}", uuid::Uuid::new_v4()));
        let bytes = zip_with(&[("overrides/config/a.txt", "hi")]);
        extract_overrides(&bytes, &dir, "overrides").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("config/a.txt")).unwrap(),
            "hi"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn overrides_reject_zip_slip() {
        let parent = std::env::temp_dir().join(format!("jd-slip-{}", uuid::Uuid::new_v4()));
        let dir = parent.join("instance");
        std::fs::create_dir_all(&dir).unwrap();
        let bytes = zip_with(&[("overrides/../../evil.txt", "pwned")]);
        assert!(extract_overrides(&bytes, &dir, "overrides").is_err());
        assert!(!parent.join("evil.txt").exists());
        std::fs::remove_dir_all(&parent).unwrap();
    }
}

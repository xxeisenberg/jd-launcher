use crate::helper::{download_file_with_retry, verify_file};
use crate::microsoft_auth;
use crate::modloaders;
use crate::profiles::{load_profiles, save_profiles};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[allow(non_snake_case)]
pub struct Version {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub releaseTime: String,
    pub sha1: String,
    pub complianceLevel: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionBlueprint {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub downloads: VersionDownloads,
    pub libraries: Vec<Library>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadableFile,
    pub server: Option<DownloadableFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadableFile>,
    pub classifiers: Option<HashMap<String, DownloadableFile>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsRule {
    pub name: String,
    pub arch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadableFile {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Assets {
    pub objects: HashMap<String, AssetObj>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetObj {
    pub hash: String,
    pub size: u64,
}

#[tauri::command]
#[specta::specta]
fn mc_cache_path() -> String {
    let dir = crate::helper::get_app_dir().join("cache");
    std::fs::create_dir_all(&dir).ok();
    dir.join("vanilla_versions.json")
        .to_string_lossy()
        .to_string()
}

fn read_mc_cache() -> Option<Vec<Version>> {
    let data = std::fs::read_to_string(mc_cache_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_mc_cache(versions: &[Version]) {
    if let Ok(json) = serde_json::to_string(versions) {
        let _ = std::fs::write(mc_cache_path(), json);
    }
}

async fn fetch_vanilla_versions() -> Result<Vec<Version>, String> {
    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let response: Manifest = reqwest::get(url)
        .await
        .map_err(|x| x.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.versions)
}

#[derive(Debug, Clone, serde::Serialize)]
struct VanillaVersionsUpdated {
    versions: Vec<Version>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_available_versions(app: tauri::AppHandle) -> Result<Vec<Version>, String> {
    use tauri::Emitter;

    if let Some(cached) = read_mc_cache() {
        let cached_clone = cached.clone();
        tokio::spawn(async move {
            if let Ok(fresh) = fetch_vanilla_versions().await {
                let cached_ids: Vec<&str> = cached_clone.iter().map(|v| v.id.as_str()).collect();
                let fresh_ids: Vec<&str> = fresh.iter().map(|v| v.id.as_str()).collect();
                if cached_ids != fresh_ids {
                    write_mc_cache(&fresh);
                    let _ = app.emit(
                        "minecraft-versions-updated",
                        VanillaVersionsUpdated { versions: fresh },
                    );
                }
            }
        });
        return Ok(cached);
    }

    let versions = fetch_vanilla_versions().await?;
    write_mc_cache(&versions);
    Ok(versions)
}

#[derive(Debug, Clone, serde::Serialize)]
struct DownloadProgress {
    completed: usize,
    total: usize,
    phase: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameLogEvent {
    src: String,
    line: String,
}

#[tauri::command]
#[specta::specta]
pub fn save_log_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Launch Minecraft using a saved profile.
#[tauri::command]
#[specta::specta]
pub async fn download_version_and_run(
    app: AppHandle,
    profile_id: String,
    username: Option<String>,
) -> Result<(), String> {
    // Load profile
    let config = load_profiles();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    // Persist as last used
    {
        let mut cfg = load_profiles();
        cfg.last_profile_id = Some(profile_id.clone());
        let _ = save_profiles(&cfg);
    }

    let response = reqwest::get(&profile.version_url)
        .await
        .map_err(|e| e.to_string())?;
    let blueprint: VersionBlueprint = response.json().await.map_err(|e| e.to_string())?;

    let os = match std::env::consts::OS {
        "linux" => "linux",
        "windows" => "windows",
        "macos" => "osx",
        other => return Err(format!("OS {} not supported.", other)),
    };

    let arch = std::env::consts::ARCH;

    // Shared assets
    let shared_dir = crate::helper::get_app_dir().to_string_lossy().to_string();
    // Game directory
    let game_dir = crate::helper::expand_path(&profile.game_dir);
    std::fs::create_dir_all(&game_dir).map_err(|e| format!("Failed to create game dir: {}", e))?;

    // Client jar
    let client_path = std::path::PathBuf::from(&shared_dir)
        .join("versions")
        .join(&blueprint.id)
        .join(format!("{}.jar", blueprint.id))
        .to_string_lossy()
        .to_string();
    let client_needs_download = !std::path::Path::new(&client_path).exists()
        || !verify_file(&client_path, &blueprint.downloads.client.sha1).unwrap_or(false);
    if client_needs_download {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                completed: 0,
                total: 1,
                phase: "client".into(),
            },
        );
    }
    if let Err(e) = download_file_with_retry(
        &blueprint.downloads.client.url,
        &client_path,
        3,
        &blueprint.downloads.client.sha1,
    )
    .await
    {
        eprintln!("{:?}", e);
    };
    if client_needs_download {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                completed: 1,
                total: 1,
                phase: "client".into(),
            },
        );
    }

    // Libraries
    let lib_semaphore = Arc::new(Semaphore::new(30));
    let mut lib_download_tasks = Vec::new();

    let mut to_download_libs: Vec<Library> = Vec::new();

    for lib in blueprint.libraries {
        if let Some(rules) = &lib.rules {
            let mut allowed = false;
            for rule in rules {
                let os_matches = match &rule.os {
                    Some(os_rule) => {
                        let mut name_matches = os_rule.name == os;
                        // Special case: Minecraft sometimes uses "osx" and treats it separately
                        if os == "osx" && os_rule.name != "osx" && os_rule.name != "macos" {
                            name_matches = false;
                        }

                        let arch_matches = match &os_rule.arch {
                            Some(arch_rule) => {
                                // For Apple Silicon, Minecraft's JSON often specifies rule arch as "arm64"
                                if arch == "aarch64" && arch_rule == "arm64" {
                                    true
                                } else {
                                    arch == arch_rule
                                }
                            }
                            None => true,
                        };

                        name_matches && arch_matches
                    }
                    None => true,
                };
                if os_matches {
                    allowed = rule.action == "allow";
                }
            }
            if allowed {
                to_download_libs.push(lib);
            }
        } else {
            to_download_libs.push(lib);
        }
    }

    struct LibTask {
        url: String,
        path: String,
        sha1: String,
    }
    let mut lib_tasks_needed: Vec<LibTask> = Vec::new();

    for lib in &to_download_libs {
        if let Some(artifact) = &lib.downloads.artifact {
            if let Some(rel_path) = &artifact.path {
                let lib_path = std::path::PathBuf::from(&shared_dir)
                    .join("libraries")
                    .join(rel_path)
                    .to_string_lossy()
                    .to_string();
                let needs = !std::path::Path::new(&lib_path).exists()
                    || !verify_file(&lib_path, &artifact.sha1).unwrap_or(false);
                if needs {
                    lib_tasks_needed.push(LibTask {
                        url: artifact.url.clone(),
                        path: lib_path,
                        sha1: artifact.sha1.clone(),
                    });
                }
            }
        }
        if let Some(natives_map) = &lib.natives {
            if let Some(classifier_key) = natives_map.get(os) {
                if let Some(classifiers) = &lib.downloads.classifiers {
                    if let Some(native_file) = classifiers.get(classifier_key) {
                        if let Some(rel_path) = &native_file.path {
                            let lib_path = std::path::PathBuf::from(&shared_dir)
                                .join("libraries")
                                .join(rel_path)
                                .to_string_lossy()
                                .to_string();
                            let needs = !std::path::Path::new(&lib_path).exists()
                                || !verify_file(&lib_path, &native_file.sha1).unwrap_or(false);
                            if needs {
                                lib_tasks_needed.push(LibTask {
                                    url: native_file.url.clone(),
                                    path: lib_path,
                                    sha1: native_file.sha1.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let lib_download_count = lib_tasks_needed.len();
    let lib_completed = Arc::new(AtomicUsize::new(0));

    if lib_download_count > 0 {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                completed: 0,
                total: lib_download_count,
                phase: "libraries".into(),
            },
        );
    }

    for task_info in lib_tasks_needed {
        let sem_clone = Arc::clone(&lib_semaphore);
        let counter = Arc::clone(&lib_completed);
        let app_clone = app.clone();

        let task = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();
            if let Err(e) =
                download_file_with_retry(&task_info.url, &task_info.path, 3, &task_info.sha1).await
            {
                eprintln!("Failed to download library: {:?}", e);
            }
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_clone.emit(
                "download-progress",
                DownloadProgress {
                    completed: done,
                    total: lib_download_count,
                    phase: "libraries".into(),
                },
            );
        });
        lib_download_tasks.push(task);
    }

    for task in lib_download_tasks {
        if let Err(e) = task.await {
            eprintln!("A library download task panicked: {:?}", e);
        }
    }

    // Assets (shared)
    let asset_index_path = std::path::PathBuf::from(&shared_dir)
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", blueprint.asset_index.id))
        .to_string_lossy()
        .to_string();
    if let Err(e) = download_file_with_retry(
        &blueprint.asset_index.url,
        &asset_index_path,
        3,
        &blueprint.asset_index.sha1,
    )
    .await
    {
        eprintln!("Failed to download asset index: {:?}", e);
    }

    let semaphore = Arc::new(Semaphore::new(50));
    let mut download_tasks = Vec::new();

    let raw_json = std::fs::read_to_string(&asset_index_path)
        .map_err(|e| format!("Failed to read asset index file: {}", e))?;
    let asset_json: Assets = serde_json::from_str(&raw_json)
        .map_err(|e| format!("Failed to parse asset index JSON: {}", e))?;

    let asset_entries: Vec<(String, String, String)> = asset_json
        .objects
        .values()
        .map(|asset| {
            let full_hash = asset.hash.clone();
            let folder = &full_hash[0..2];
            let asset_path = std::path::PathBuf::from(&shared_dir)
                .join("assets")
                .join("objects")
                .join(folder)
                .join(&full_hash)
                .to_string_lossy()
                .to_string();
            let asset_url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                folder, full_hash
            );
            (full_hash, asset_path, asset_url)
        })
        .collect();

    let needs_download: Vec<(String, String, String)> = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        asset_entries
            .into_par_iter()
            .filter(|(hash, path, _url)| {
                let already_ok =
                    std::path::Path::new(path).exists() && verify_file(path, hash).unwrap_or(false);
                !already_ok
            })
            .collect()
    })
    .await
    .map_err(|e| format!("Hash check failed: {:?}", e))?;

    let download_count = needs_download.len();
    let asset_completed = Arc::new(AtomicUsize::new(0));

    if download_count > 0 {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                completed: 0,
                total: download_count,
                phase: "assets".into(),
            },
        );
    }

    for (full_hash, asset_path, asset_url) in needs_download {
        let sem_clone = Arc::clone(&semaphore);
        let counter = Arc::clone(&asset_completed);
        let app_clone = app.clone();

        let task = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();
            if let Err(e) = download_file_with_retry(&asset_url, &asset_path, 3, &full_hash).await {
                eprintln!("Failed to download {}: {:?}", asset_url, e);
            }
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_clone.emit(
                "download-progress",
                DownloadProgress {
                    completed: done,
                    total: download_count,
                    phase: "assets".into(),
                },
            );
        });

        download_tasks.push(task);
    }

    for task in download_tasks {
        let _ = task.await;
    }

    // Extracting Natives
    let natives_path = std::path::PathBuf::from(&shared_dir)
        .join("versions")
        .join(&blueprint.id)
        .join("natives")
        .to_string_lossy()
        .to_string();
    if std::fs::create_dir_all(&natives_path).is_err() {
        return Err("Failed to create natives folder.".to_string());
    }

    for lib in &to_download_libs {
        if let Some(natives_map) = &lib.natives {
            if let Some(classifier_key) = natives_map.get(os) {
                if let Some(classifiers) = &lib.downloads.classifiers {
                    if let Some(native_file) = classifiers.get(classifier_key) {
                        if let Some(path) = &native_file.path {
                            let full_path = std::path::PathBuf::from(&shared_dir)
                                .join("libraries")
                                .join(path)
                                .to_string_lossy()
                                .to_string();
                            let _ = unzip_jar(&full_path, &natives_path);
                        }
                    }
                }
            }
        }
    }

    // Setup Java cmd
    let required_java_version = crate::java_manager::get_required_java_version(&blueprint.id);
    let mut java_cmd = profile.java_path.clone().filter(|s| !s.is_empty());

    if java_cmd.is_none() {
        let sys_javas = crate::java_manager::detect_system_javas();
        if let Some(exact) = sys_javas
            .iter()
            .find(|j| j.version == required_java_version)
        {
            java_cmd = Some(exact.path.clone());
        } else if let Some(valid) = sys_javas
            .into_iter()
            .find(|j| j.version >= required_java_version)
        {
            java_cmd = Some(valid.path);
        } else {
            match crate::java_manager::download_java(required_java_version, app.clone()).await {
                Ok(path) => java_cmd = Some(path),
                Err(e) => {
                    return Err(format!(
                        "Failed to auto-download Java {}: {}",
                        required_java_version, e
                    ))
                }
            }
        }
    }

    let java_cmd = java_cmd.unwrap_or_else(|| {
        if cfg!(windows) {
            "javaw".to_string()
        } else {
            "java".to_string()
        }
    });

    let client_jar_path = std::path::PathBuf::from(&shared_dir)
        .join("versions")
        .join(&blueprint.id)
        .join(format!("{}.jar", blueprint.id))
        .to_string_lossy()
        .to_string();

    // ── Modloader integration ────────────────────────────────────────
    let mut modloader_classpath: Vec<String> = Vec::new();
    let mut modloader_main_class: Option<String> = None;
    let mut modloader_jvm_args: Vec<String> = Vec::new();
    let mut modloader_game_args: Vec<String> = Vec::new();

    if profile.modloader != "none" {
        if let Some(loader_ver) = &profile.modloader_version {
            match profile.modloader.as_str() {
                "fabric" => {
                    let lp = modloaders::fetch_fabric_profile(&profile.version, loader_ver).await?;
                    modloader_main_class = Some(lp.main_class);
                    if let Some(args) = &lp.arguments {
                        if let Some(jvm) = &args.jvm {
                            for arg in jvm {
                                if let Some(s) = arg.as_str() {
                                    modloader_jvm_args.push(s.to_string());
                                }
                            }
                        }
                        if let Some(game) = &args.game {
                            for arg in game {
                                if let Some(s) = arg.as_str() {
                                    modloader_game_args.push(s.to_string());
                                }
                            }
                        }
                    }
                    let loader_libs: Vec<modloaders::LoaderLibrary> = lp.libraries;
                    modloader_classpath =
                        modloaders::download_modloader_libraries(&loader_libs, &shared_dir, &app)
                            .await?;
                }
                "quilt" => {
                    let lp = modloaders::fetch_quilt_profile(&profile.version, loader_ver).await?;
                    modloader_main_class = Some(lp.main_class);
                    if let Some(args) = &lp.arguments {
                        if let Some(jvm) = &args.jvm {
                            for arg in jvm {
                                if let Some(s) = arg.as_str() {
                                    modloader_jvm_args.push(s.to_string());
                                }
                            }
                        }
                        if let Some(game) = &args.game {
                            for arg in game {
                                if let Some(s) = arg.as_str() {
                                    modloader_game_args.push(s.to_string());
                                }
                            }
                        }
                    }
                    let loader_libs: Vec<modloaders::LoaderLibrary> = lp.libraries;
                    modloader_classpath =
                        modloaders::download_modloader_libraries(&loader_libs, &shared_dir, &app)
                            .await?;
                }
                "forge" => {
                    let fp = modloaders::fetch_forge_profile(
                        &profile.version,
                        loader_ver,
                        &shared_dir,
                        &client_jar_path,
                        &java_cmd,
                    )
                    .await?;
                    modloader_main_class = Some(fp.main_class);
                    if let Some(args) = &fp.arguments {
                        if let Some(jvm) = &args.jvm {
                            for arg in jvm {
                                if let Some(s) = arg.as_str() {
                                    modloader_jvm_args.push(s.to_string());
                                }
                            }
                        }
                        if let Some(game) = &args.game {
                            for arg in game {
                                if let Some(s) = arg.as_str() {
                                    modloader_game_args.push(s.to_string());
                                }
                            }
                        }
                    }
                    modloader_classpath =
                        modloaders::download_forge_libraries(&fp.libraries, &shared_dir, &app)
                            .await?;
                }
                "neoforge" => {
                    let fp = modloaders::fetch_neoforge_profile(
                        loader_ver,
                        &shared_dir,
                        &client_jar_path,
                        &java_cmd,
                    )
                    .await?;
                    modloader_main_class = Some(fp.main_class);
                    if let Some(args) = &fp.arguments {
                        if let Some(jvm) = &args.jvm {
                            for arg in jvm {
                                if let Some(s) = arg.as_str() {
                                    modloader_jvm_args.push(s.to_string());
                                }
                            }
                        }
                        if let Some(game) = &args.game {
                            for arg in game {
                                if let Some(s) = arg.as_str() {
                                    modloader_game_args.push(s.to_string());
                                }
                            }
                        }
                    }
                    modloader_classpath =
                        modloaders::download_forge_libraries(&fp.libraries, &shared_dir, &app)
                            .await?;
                }
                _ => {}
            }
        }
    }

    let lib_dir = std::path::PathBuf::from(&shared_dir)
        .join("libraries")
        .to_string_lossy()
        .to_string();
    let cp_sep = if cfg!(windows) { ";" } else { ":" };
    let replace_templates = |s: &str| -> String {
        s.replace("${library_directory}", &lib_dir)
            .replace("${classpath_separator}", cp_sep)
            .replace("${version_name}", &blueprint.id)
    };
    modloader_jvm_args = modloader_jvm_args
        .iter()
        .map(|a| replace_templates(a))
        .collect();
    modloader_game_args = modloader_game_args
        .iter()
        .map(|a| replace_templates(a))
        .collect();

    // ── Classpath ────────────────────────────────────────────────────
    // Modloader libs come first, then vanilla libs, then client jar
    let mut classpath_elements: Vec<String> = Vec::new();

    // Modloader libraries (prepended)
    for path in &modloader_classpath {
        if !classpath_elements.contains(path) {
            classpath_elements.push(path.clone());
        }
    }

    // Vanilla libraries
    for lib in &to_download_libs {
        if let Some(artifact) = &lib.downloads.artifact {
            if let Some(path) = &artifact.path {
                let absolute_path = std::path::PathBuf::from(&shared_dir)
                    .join("libraries")
                    .join(path)
                    .to_string_lossy()
                    .to_string();
                if !classpath_elements.contains(&absolute_path) {
                    classpath_elements.push(absolute_path);
                }
            }
        }
    }

    classpath_elements.push(client_jar_path.clone());

    let separator = if cfg!(windows) { ";" } else { ":" };
    let final_classpath = classpath_elements.join(separator);

    // Determine main class (modloader overrides vanilla)
    let launch_main_class = modloader_main_class
        .as_deref()
        .unwrap_or(&blueprint.main_class);

    // Launch
    let mut mc_process = std::process::Command::new(&java_cmd);
    mc_process.current_dir(&game_dir);

    // JVM args from profile
    for arg in profile.jvm_args.split_whitespace() {
        mc_process.arg(arg);
    }

    // Modloader JVM args
    for arg in &modloader_jvm_args {
        mc_process.arg(arg);
    }

    mc_process.arg(format!("-Djava.library.path={}", natives_path));

    let online = microsoft_auth::get_auth_mode();

    if !online {
        // Offline mode
        let dummy = "https://invalid.invalid";
        mc_process.arg(format!("-Dminecraft.api.auth.host={}", dummy));
        mc_process.arg(format!("-Dminecraft.api.account.host={}", dummy));
        mc_process.arg(format!("-Dminecraft.api.session.host={}", dummy));
        mc_process.arg(format!("-Dminecraft.api.services.host={}", dummy));
        mc_process.arg("-Dminecraft.api.env=custom");
    }

    mc_process.arg("-cp");
    mc_process.arg(&final_classpath);
    mc_process.arg(launch_main_class);

    if online {
        // Online: use real MS account
        let account =
            microsoft_auth::get_active_account().ok_or("No Microsoft account logged in")?;
        mc_process.arg("--username").arg(&account.username);
        mc_process.arg("--version").arg(&blueprint.id);
        mc_process.arg("--gameDir").arg(&game_dir);
        mc_process
            .arg("--assetsDir")
            .arg(format!("{}/assets", shared_dir));
        mc_process
            .arg("--assetIndex")
            .arg(&blueprint.asset_index.id);
        mc_process.arg("--uuid").arg(&account.uuid);
        mc_process.arg("--accessToken").arg(&account.access_token);
        mc_process.arg("--userType").arg("msa");
        mc_process.arg("--versionType").arg("release");
    } else {
        // Offline: use provided username
        let name = username.unwrap_or_else(|| "Steve".to_string());
        mc_process.arg("--username").arg(&name);
        mc_process.arg("--version").arg(&blueprint.id);
        mc_process.arg("--gameDir").arg(&game_dir);
        mc_process
            .arg("--assetsDir")
            .arg(format!("{}/assets", shared_dir));
        mc_process
            .arg("--assetIndex")
            .arg(&blueprint.asset_index.id);
        let offline_uuid = generate_offline_uuid(&name);
        mc_process.arg("--uuid").arg(&offline_uuid);
        mc_process.arg("--accessToken").arg("0");
        mc_process.arg("--userType").arg("legacy");
        mc_process.arg("--versionType").arg("release");
    }

    // Resolution
    mc_process
        .arg("--width")
        .arg(profile.resolution.width.to_string());
    mc_process
        .arg("--height")
        .arg(profile.resolution.height.to_string());

    if config.settings.fullscreen {
        mc_process.arg("--fullscreen");
    }

    // Modloader args
    for arg in &modloader_game_args {
        mc_process.arg(arg);
    }

    // Done downloading
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed: 0,
            total: 0,
            phase: "done".into(),
        },
    );

    mc_process.stdout(std::process::Stdio::piped());
    mc_process.stderr(std::process::Stdio::piped());

    match mc_process.spawn() {
        Ok(mut child) => {
            println!("Game launched successfully with PID: {}", child.id());

            if let Some(stdout) = child.stdout.take() {
                let app_clone = app.clone();
                std::thread::spawn(move || {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        let _ = app_clone.emit(
                            "game-log",
                            GameLogEvent {
                                src: "stdout".into(),
                                line,
                            },
                        );
                    }
                });
            }

            if let Some(stderr) = child.stderr.take() {
                let app_clone = app.clone();
                std::thread::spawn(move || {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().map_while(Result::ok) {
                        let _ = app_clone.emit(
                            "game-log",
                            GameLogEvent {
                                src: "stderr".into(),
                                line,
                            },
                        );
                    }
                });
            }
        }
        Err(e) => {
            eprintln!(
                "CRITICAL FAILURE: Could not start Java. Is it installed? Error: {}",
                e
            );
        }
    }

    Ok(())
}

fn generate_offline_uuid(username: &str) -> String {
    let input = format!("OfflinePlayer:{}", username);
    let mut hash = md5::compute(input.as_bytes()).0;

    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;

    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]),
        u16::from_be_bytes([hash[4], hash[5]]),
        u16::from_be_bytes([hash[6], hash[7]]),
        u16::from_be_bytes([hash[8], hash[9]]),
        u64::from_be_bytes([0, 0, hash[10], hash[11], hash[12], hash[13], hash[14], hash[15]]),
    )
}

fn unzip_jar(file_path: &str, output_dir: &str) -> zip::result::ZipResult<()> {
    let file = std::fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let file_name = match std::path::Path::new(file.name()).file_name() {
            Some(name) => name.to_string_lossy(),
            None => continue,
        };

        if file_name.ends_with(".so")
            || file_name.ends_with(".dll")
            || file_name.ends_with(".dylib")
        {
            let output_path = format!("{}/{}", output_dir, file_name);
            let mut output_file = std::fs::File::create(&output_path)?;
            std::io::copy(&mut file, &mut output_file)?;
        }
    }
    Ok(())
}

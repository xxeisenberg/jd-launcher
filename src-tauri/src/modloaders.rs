use crate::helper::download_file_with_retry;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::io::Read;

// Public types

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModloaderVersion {
    pub version: String,
    pub stable: bool,
}

// Loader list types

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    version: String,
    stable: Option<bool>,
}

// Profile JSON types

#[derive(Debug, Deserialize)]
pub struct LoaderProfileJson {
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<LoaderLibrary>,
    pub arguments: Option<LoaderArguments>,
}

#[derive(Debug, Deserialize)]
pub struct LoaderLibrary {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoaderArguments {
    pub jvm: Option<Vec<serde_json::Value>>,
    pub game: Option<Vec<serde_json::Value>>,
}

// Forge promotions

#[derive(Debug, Deserialize)]
struct ForgePromotions {
    promos: HashMap<String, String>,
}

// Forge versions

#[derive(Debug, Deserialize)]
pub struct ForgeVersionJson {
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<ForgeLibEntry>,
    pub arguments: Option<LoaderArguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForgeLibEntry {
    pub name: String,
    pub downloads: Option<ForgeLibDownloads>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForgeLibDownloads {
    pub artifact: Option<ForgeLibArtifact>,
}

#[derive(Debug, Deserialize)]
pub struct ForgeLibArtifact {
    pub path: Option<String>,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

// Install profiles

#[derive(Debug, Deserialize)]
struct InstallProfile {
    processors: Option<Vec<Processor>>,
    data: Option<HashMap<String, DataEntry>>,
    libraries: Option<Vec<ForgeLibEntry>>,
}

#[derive(Debug, Deserialize)]
struct Processor {
    jar: String,
    classpath: Vec<String>,
    args: Vec<String>,
    sides: Option<Vec<String>>,
    outputs: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct DataEntry {
    client: Option<String>,
    server: Option<String>,
}

// Maven to path

pub fn maven_to_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return name.to_string();
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    if parts.len() >= 4 {
        let classifier = parts[3];
        // Handle classifier with extension (e.g. "mappings@tsrg")
        if let Some((cls, ext)) = classifier.split_once('@') {
            format!(
                "{}/{}/{}/{}-{}-{}.{}",
                group, artifact, version, artifact, version, cls, ext
            )
        } else {
            format!(
                "{}/{}/{}/{}-{}-{}.jar",
                group, artifact, version, artifact, version, classifier
            )
        }
    } else {
        format!(
            "{}/{}/{}/{}-{}.jar",
            group, artifact, version, artifact, version
        )
    }
}

pub fn resolve_library_url(name: &str, repo_url: Option<&str>) -> String {
    let path = maven_to_path(name);
    let base = repo_url.unwrap_or("https://libraries.minecraft.net/");
    let base = base.trim_end_matches('/');
    format!("{}/{}", base, path)
}

// Modloader version cache

fn cache_path(modloader: &str, mc_version: &str) -> String {
    let dir = crate::helper::get_app_dir().join("cache");
    std::fs::create_dir_all(&dir).ok();
    dir.join(format!("modloader_{}_{}.json", modloader, mc_version))
        .to_string_lossy()
        .to_string()
}

fn read_cache(modloader: &str, mc_version: &str) -> Option<Vec<ModloaderVersion>> {
    let path = cache_path(modloader, mc_version);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_cache(modloader: &str, mc_version: &str, versions: &[ModloaderVersion]) {
    let path = cache_path(modloader, mc_version);
    if let Ok(json) = serde_json::to_string(versions) {
        let _ = std::fs::write(&path, json);
    }
}

async fn fetch_versions(
    modloader: &str,
    mc_version: &str,
) -> Result<Vec<ModloaderVersion>, String> {
    match modloader {
        "fabric" => get_fabric_versions().await,
        "quilt" => get_quilt_versions().await,
        "forge" => get_forge_versions(mc_version).await,
        "neoforge" => get_neoforge_versions(mc_version).await,
        _ => Ok(vec![]),
    }
}

#[derive(Debug, Clone, Serialize)]
struct ModloaderVersionsUpdated {
    modloader: String,
    mc_version: String,
    versions: Vec<ModloaderVersion>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_modloader_versions(
    app: tauri::AppHandle,
    modloader: String,
    mc_version: String,
) -> Result<Vec<ModloaderVersion>, String> {
    use tauri::Emitter;

    let cached = read_cache(&modloader, &mc_version);

    if let Some(cached_versions) = cached {
        // Return cached, revalidate
        let ml = modloader.clone();
        let mv = mc_version.clone();
        let app_clone = app.clone();

        // Clone versions for the background task
        let cached_versions_clone = cached_versions.clone();

        tokio::spawn(async move {
            if let Ok(fresh) = fetch_versions(&ml, &mv).await {
                // Compare: check if version lists differ
                let cached_ids: Vec<&str> = cached_versions_clone
                    .iter()
                    .map(|v| v.version.as_str())
                    .collect();
                let fresh_ids: Vec<&str> = fresh.iter().map(|v| v.version.as_str()).collect();
                if cached_ids != fresh_ids {
                    write_cache(&ml, &mv, &fresh);
                    let _ = app_clone.emit(
                        "modloader-versions-updated",
                        ModloaderVersionsUpdated {
                            modloader: ml,
                            mc_version: mv,
                            versions: fresh,
                        },
                    );
                }
            }
        });
        return Ok(cached_versions);
    }

    // Fetch and cache
    let versions = fetch_versions(&modloader, &mc_version).await?;
    write_cache(&modloader, &mc_version, &versions);
    Ok(versions)
}

// Fabric

async fn get_fabric_versions() -> Result<Vec<ModloaderVersion>, String> {
    let url = "https://meta.fabricmc.net/v2/versions/loader";
    let entries: Vec<FabricLoaderEntry> = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(entries
        .into_iter()
        .map(|e| ModloaderVersion {
            version: e.version,
            stable: e.stable.unwrap_or(false),
        })
        .collect())
}

pub async fn fetch_fabric_profile(
    mc_version: &str,
    loader_version: &str,
) -> Result<LoaderProfileJson, String> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        mc_version, loader_version
    );
    let profile: LoaderProfileJson = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch Fabric profile: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Fabric profile JSON: {}", e))?;
    Ok(profile)
}

// Quilt

async fn get_quilt_versions() -> Result<Vec<ModloaderVersion>, String> {
    let url = "https://meta.quiltmc.org/v3/versions/loader";
    let entries: Vec<FabricLoaderEntry> = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(entries
        .into_iter()
        .map(|e| ModloaderVersion {
            version: e.version.clone(),
            stable: !e.version.contains("beta"),
        })
        .collect())
}

pub async fn fetch_quilt_profile(
    mc_version: &str,
    loader_version: &str,
) -> Result<LoaderProfileJson, String> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        mc_version, loader_version
    );
    let profile: LoaderProfileJson = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch Quilt profile: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Quilt profile JSON: {}", e))?;
    Ok(profile)
}

// Forge

async fn get_forge_versions(mc_version: &str) -> Result<Vec<ModloaderVersion>, String> {
    let url = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
    let promos: ForgePromotions = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let prefix_latest = format!("{}-latest", mc_version);
    let prefix_recommended = format!("{}-recommended", mc_version);

    let mut versions: Vec<ModloaderVersion> = Vec::new();
    if let Some(ver) = promos.promos.get(&prefix_recommended) {
        versions.push(ModloaderVersion {
            version: ver.clone(),
            stable: true,
        });
    }
    if let Some(ver) = promos.promos.get(&prefix_latest) {
        if !versions.iter().any(|v| v.version == *ver) {
            versions.push(ModloaderVersion {
                version: ver.clone(),
                stable: false,
            });
        }
    }

    Ok(versions)
}

pub async fn fetch_forge_profile(
    mc_version: &str,
    forge_version: &str,
    shared_dir: &str,
    client_jar_path: &str,
    java_cmd: &str,
) -> Result<ForgeVersionJson, String> {
    let combined = format!("{}-{}", mc_version, forge_version);
    let url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        combined, combined
    );
    download_and_process_installer(&url, shared_dir, client_jar_path, java_cmd, "client").await
}

// NeoForge

async fn get_neoforge_versions(mc_version: &str) -> Result<Vec<ModloaderVersion>, String> {
    let url = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
    let xml_text = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mc_parts: Vec<&str> = mc_version.split('.').collect();
    let neo_prefix = if mc_parts.len() >= 3 {
        format!("{}.{}.", mc_parts[1], mc_parts[2])
    } else if mc_parts.len() == 2 {
        format!("{}.", mc_parts[1])
    } else {
        return Ok(vec![]);
    };

    let versions = parse_maven_versions(&xml_text, &neo_prefix)?;

    Ok(versions
        .into_iter()
        .rev()
        .map(|v| ModloaderVersion {
            stable: !v.contains("beta") && !v.contains("rc"),
            version: v,
        })
        .collect())
}

fn parse_maven_versions(xml: &str, prefix: &str) -> Result<Vec<String>, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut in_version_tag = false;
    let mut in_versions = false;
    let mut versions: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"versions" {
                    in_versions = true;
                } else if in_versions && local.as_ref() == b"version" {
                    in_version_tag = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"versions" {
                    in_versions = false;
                } else if local.as_ref() == b"version" {
                    in_version_tag = false;
                }
            }
            Ok(Event::Text(e)) => {
                if in_version_tag {
                    let text = e
                        .unescape()
                        .map_err(|err| format!("XML parse error: {}", err))?
                        .to_string();
                    if text.starts_with(prefix) {
                        versions.push(text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(versions)
}

pub async fn fetch_neoforge_profile(
    neoforge_version: &str,
    shared_dir: &str,
    client_jar_path: &str,
    java_cmd: &str,
) -> Result<ForgeVersionJson, String> {
    let url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        neoforge_version, neoforge_version
    );
    download_and_process_installer(&url, shared_dir, client_jar_path, java_cmd, "client").await
}

// Download and process

async fn download_and_process_installer(
    url: &str,
    shared_dir: &str,
    client_jar_path: &str,
    java_cmd: &str,
    side: &str,
) -> Result<ForgeVersionJson, String> {
    eprintln!("Downloading installer from {}", url);

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download installer: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} downloading installer", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read installer bytes: {}", e))?;

    let installer_path = format!("{}/installer_tmp.jar", shared_dir);
    std::fs::write(&installer_path, &bytes)
        .map_err(|e| format!("Failed to write installer: {}", e))?;

    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open installer: {}", e))?;

    // 1. Extract version.json
    let version_json_str = {
        let mut f = archive
            .by_name("version.json")
            .map_err(|_| "No version.json in installer".to_string())?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };
    let profile: ForgeVersionJson =
        serde_json::from_str(&version_json_str).map_err(|e| e.to_string())?;

    // 2. Extract bundled maven/ JARs
    let maven_entries: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let f = archive.by_index(i).ok()?;
            let name = f.name().to_string();
            if name.starts_with("maven/") && name.ends_with(".jar") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    for entry_name in &maven_entries {
        let rel = entry_name.strip_prefix("maven/").unwrap_or(entry_name);
        let dest = format!("{}/libraries/{}", shared_dir, rel);
        if std::path::Path::new(&dest).exists() {
            continue;
        }
        if let Some(parent) = std::path::Path::new(&dest).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut f = archive.by_name(entry_name).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        std::fs::write(&dest, &buf).map_err(|e| e.to_string())?;
        eprintln!("Extracted: {}", dest);
    }

    // 3. Extract data/ files for processors
    let data_entries: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let f = archive.by_index(i).ok()?;
            let name = f.name().to_string();
            if name.starts_with("data/") && !name.ends_with('/') {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    // Use a hash of the URL to create a unique data directory per installer
    let url_hash = format!("{:x}", md5::compute(url.as_bytes()));
    let data_dir = format!("{}/installer_data_{}", shared_dir, &url_hash[..8]);
    std::fs::create_dir_all(&data_dir).ok();

    for entry_name in &data_entries {
        let filename = entry_name.strip_prefix("data/").unwrap_or(entry_name);
        let dest = format!("{}/{}", data_dir, filename);
        if !std::path::Path::new(&dest).exists() {
            let mut f = archive.by_name(entry_name).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            std::fs::write(&dest, &buf).map_err(|e| e.to_string())?;
        }
    }

    // 4. Parse install_profile.json
    let install_profile: InstallProfile = {
        let mut f = archive
            .by_name("install_profile.json")
            .map_err(|_| "No install_profile.json".to_string())?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|e| e.to_string())?
    };

    // 5. Download processor libraries
    if let Some(libs) = &install_profile.libraries {
        for lib in libs {
            let rel_path = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.clone())
                .unwrap_or_else(|| maven_to_path(&lib.name));
            let local = format!("{}/libraries/{}", shared_dir, rel_path);
            if std::path::Path::new(&local).exists() {
                continue;
            }
            let dl_url = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.url.clone())
                .unwrap_or_else(|| resolve_library_url(&lib.name, lib.url.as_deref()));
            if dl_url.is_empty() {
                continue;
            }
            let sha1 = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.sha1.clone())
                .unwrap_or_default();
            download_file_with_retry(&dl_url, &local, 3, &sha1).await?;
        }
    }

    // 6. Build data variable map
    let lib_dir = format!("{}/libraries", shared_dir);
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("SIDE".into(), side.into());
    vars.insert("MINECRAFT_JAR".into(), client_jar_path.into());
    vars.insert("INSTALLER".into(), installer_path.clone());
    vars.insert("ROOT".into(), shared_dir.into());
    vars.insert("LIBRARY_DIR".into(), lib_dir.clone());

    if let Some(data) = &install_profile.data {
        for (key, entry) in data {
            let val = if side == "client" {
                entry.client.as_deref().unwrap_or("")
            } else {
                entry.server.as_deref().unwrap_or("")
            };
            let resolved = if val.starts_with('[') && val.ends_with(']') {
                let coord = &val[1..val.len() - 1];
                format!("{}/{}", lib_dir, maven_to_path(coord))
            } else if val.starts_with('/') {
                let filename = val.trim_start_matches('/');
                let filename = filename.strip_prefix("data/").unwrap_or(filename);
                format!("{}/{}", data_dir, filename)
            } else if val.starts_with('\'') && val.ends_with('\'') {
                val[1..val.len() - 1].to_string()
            } else {
                val.to_string()
            };
            vars.insert(key.clone(), resolved);
        }
    }

    // 7. Run processors
    if let Some(processors) = &install_profile.processors {
        for proc in processors {
            // Side filter
            if let Some(sides) = &proc.sides {
                if !sides.contains(&side.to_string()) {
                    continue;
                }
            }

            // Check cached outputs
            if let Some(outputs) = &proc.outputs {
                let all_exist = outputs
                    .keys()
                    .all(|k| std::path::Path::new(&subst(k, &vars, &lib_dir)).exists());
                if all_exist {
                    eprintln!("Processor {} cached, skipping", proc.jar);
                    continue;
                }
            }

            // Build classpath
            let jar_path = format!("{}/{}", lib_dir, maven_to_path(&proc.jar));
            let mut cp_parts = vec![jar_path.clone()];
            for dep in &proc.classpath {
                cp_parts.push(format!("{}/{}", lib_dir, maven_to_path(dep)));
            }
            let sep = if cfg!(windows) { ";" } else { ":" };
            let cp = cp_parts.join(sep);

            let resolved_args: Vec<String> = proc
                .args
                .iter()
                .map(|a| subst(a, &vars, &lib_dir))
                .collect();

            // Download any [maven:coord] artifacts referenced in args that don't exist
            for arg in &resolved_args {
                let path = std::path::Path::new(arg);
                if arg.contains("/libraries/") && !path.exists() {
                    // Try to guess the Maven URL from the path
                    if let Some(rel) = arg.strip_prefix(&format!("{}/", lib_dir)) {
                        // Try multiple Maven repos
                        let repos = [
                            "https://maven.neoforged.net/releases",
                            "https://maven.minecraftforge.net",
                            "https://libraries.minecraft.net",
                            "https://repo1.maven.org/maven2",
                        ];
                        let mut downloaded = false;
                        for repo in &repos {
                            let dl_url = format!("{}/{}", repo, rel);
                            if let Ok(_) = download_file_with_retry(&dl_url, arg, 1, "").await {
                                eprintln!("Downloaded artifact: {} from {}", rel, repo);
                                downloaded = true;
                                break;
                            }
                        }
                        if !downloaded {
                            eprintln!("Warning: could not download artifact {}", rel);
                        }
                    }
                }
            }

            let main_class = get_jar_main_class(&jar_path)?;

            eprintln!("Running: {} {} {:?}", proc.jar, main_class, resolved_args);

            let output = std::process::Command::new(java_cmd)
                .arg("-cp")
                .arg(&cp)
                .arg(&main_class)
                .args(&resolved_args)
                .output()
                .map_err(|e| format!("Processor {} failed to start: {}", proc.jar, e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("Processor {} FAILED:\n{}\n{}", proc.jar, stdout, stderr);
                return Err(format!("Forge processor {} failed: {}", proc.jar, stderr));
            }
            eprintln!("Processor {} OK", proc.jar);
        }
    }

    let _ = std::fs::remove_file(&installer_path);
    Ok(profile)
}

fn subst(s: &str, vars: &HashMap<String, String>, lib_dir: &str) -> String {
    let mut r = s.to_string();
    // Replace {KEY} data variables
    for (k, v) in vars {
        r = r.replace(&format!("{{{}}}", k), v);
    }
    // Replace [maven:coordinate] artifact references with library paths
    if r.starts_with('[') && r.ends_with(']') && r.contains(':') {
        let coord = &r[1..r.len() - 1];
        r = format!("{}/{}", lib_dir, maven_to_path(coord));
    }
    r
}

fn get_jar_main_class(jar_path: &str) -> Result<String, String> {
    let file =
        std::fs::File::open(jar_path).map_err(|e| format!("Can't open {}: {}", jar_path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut mf = archive
        .by_name("META-INF/MANIFEST.MF")
        .map_err(|_| format!("No MANIFEST.MF in {}", jar_path))?;
    let mut text = String::new();
    mf.read_to_string(&mut text).map_err(|e| e.to_string())?;
    for line in text.lines() {
        if let Some(cls) = line.strip_prefix("Main-Class: ") {
            return Ok(cls.trim().to_string());
        }
    }
    Err(format!("No Main-Class in {}", jar_path))
}

// Modloader sync

pub async fn download_modloader_libraries(
    libs: &[LoaderLibrary],
    shared_dir: &str,
    app: &tauri::AppHandle,
) -> Result<Vec<String>, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tauri::Emitter;

    #[derive(Debug, Clone, Serialize)]
    struct DownloadProgress {
        completed: usize,
        total: usize,
        phase: String,
    }

    let mut paths: Vec<String> = Vec::new();
    let mut to_download: Vec<(String, String)> = Vec::new();

    for lib in libs {
        let rel_path = maven_to_path(&lib.name);
        let local_path = format!("{}/libraries/{}", shared_dir, rel_path);
        paths.push(local_path.clone());

        if !std::path::Path::new(&local_path).exists() {
            let url = resolve_library_url(&lib.name, lib.url.as_deref());
            to_download.push((url, local_path));
        }
    }

    if to_download.is_empty() {
        return Ok(paths);
    }

    let total = to_download.len();
    let completed = Arc::new(AtomicUsize::new(0));

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed: 0,
            total,
            phase: "modloader".into(),
        },
    );

    let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
    let mut tasks = Vec::new();

    for (url, path) in to_download {
        let sem = Arc::clone(&semaphore);
        let counter = Arc::clone(&completed);
        let app_clone = app.clone();

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            if let Err(e) = download_file_with_retry(&url, &path, 3, "").await {
                eprintln!("Failed to download modloader lib {}: {:?}", url, e);
            }
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_clone.emit(
                "download-progress",
                DownloadProgress {
                    completed: done,
                    total,
                    phase: "modloader".into(),
                },
            );
        });
        tasks.push(task);
    }

    for task in tasks {
        if let Err(e) = task.await {
            eprintln!("Modloader lib download task panicked: {:?}", e);
        }
    }

    Ok(paths)
}

pub async fn download_forge_libraries(
    libs: &[ForgeLibEntry],
    shared_dir: &str,
    app: &tauri::AppHandle,
) -> Result<Vec<String>, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tauri::Emitter;

    #[derive(Debug, Clone, Serialize)]
    struct DownloadProgress {
        completed: usize,
        total: usize,
        phase: String,
    }

    let mut paths: Vec<String> = Vec::new();
    let mut to_download: Vec<(String, String, String)> = Vec::new();

    for lib in libs {
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let rel_path = artifact
                    .path
                    .clone()
                    .unwrap_or_else(|| maven_to_path(&lib.name));
                let local_path = format!("{}/libraries/{}", shared_dir, rel_path);
                paths.push(local_path.clone());

                let url = artifact
                    .url
                    .clone()
                    .unwrap_or_else(|| resolve_library_url(&lib.name, lib.url.as_deref()));

                if url.is_empty() {
                    continue;
                }

                if !std::path::Path::new(&local_path).exists() {
                    let sha1 = artifact.sha1.clone().unwrap_or_default();
                    to_download.push((url, local_path, sha1));
                }
            }
        } else {
            let rel_path = maven_to_path(&lib.name);
            let local_path = format!("{}/libraries/{}", shared_dir, rel_path);
            paths.push(local_path.clone());

            if !std::path::Path::new(&local_path).exists() {
                let url = resolve_library_url(&lib.name, lib.url.as_deref());
                to_download.push((url, local_path, String::new()));
            }
        }
    }

    if to_download.is_empty() {
        return Ok(paths);
    }

    let total = to_download.len();
    let completed = Arc::new(AtomicUsize::new(0));

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            completed: 0,
            total,
            phase: "modloader".into(),
        },
    );

    let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
    let mut tasks = Vec::new();

    for (url, path, sha1) in to_download {
        let sem = Arc::clone(&semaphore);
        let counter = Arc::clone(&completed);
        let app_clone = app.clone();

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            if let Err(e) = download_file_with_retry(&url, &path, 3, &sha1).await {
                eprintln!("Failed to download forge lib {}: {:?}", url, e);
            }
            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_clone.emit(
                "download-progress",
                DownloadProgress {
                    completed: done,
                    total,
                    phase: "modloader".into(),
                },
            );
        });
        tasks.push(task);
    }

    for task in tasks {
        if let Err(e) = task.await {
            eprintln!("Forge lib download task panicked: {:?}", e);
        }
    }

    Ok(paths)
}

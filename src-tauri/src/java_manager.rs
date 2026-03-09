use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct JavaInstall {
    pub path: String,
    pub version: u8,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgress {
    completed: usize,
    total: usize,
    phase: String,
}

pub fn get_java_version(binary_path: &str) -> Option<u8> {
    let output = Command::new(binary_path).arg("-version").output().ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Some java versions print to stdout instead of stderr
    let full_output = format!("{}\n{}", stderr, stdout);

    parse_java_version(&full_output)
}

fn parse_java_version(output: &str) -> Option<u8> {
    for line in output.lines() {
        if line.to_lowercase().contains("version") {
            let start = line.find('"')? + 1;
            let end = line[start..].find('"')? + start;
            let ver_str = &line[start..end];
            let parts: Vec<&str> = ver_str.split('.').collect();
            if parts.is_empty() {
                return None;
            }
            if parts[0] == "1" && parts.len() > 1 {
                return parts[1].parse::<u8>().ok();
            } else {
                return parts[0].parse::<u8>().ok();
            }
        }
    }
    None
}

#[tauri::command]
#[specta::specta]
pub fn detect_system_javas() -> Vec<JavaInstall> {
    let mut installs: Vec<JavaInstall> = Vec::new();
    let mut searched = std::collections::HashSet::new();

    let mut check = |path: &Path| {
        if !path.exists() {
            return;
        }
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !searched.insert(abs.clone()) {
            return;
        }
        let bin_path = abs.to_string_lossy().to_string();
        if let Some(v) = get_java_version(&bin_path) {
            installs.push(JavaInstall {
                path: bin_path,
                version: v,
            });
        }
    };

    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            let java_exe = p.join(if cfg!(windows) { "java.exe" } else { "java" });
            check(&java_exe);
        }
    }

    if cfg!(unix) && !cfg!(target_os = "macos") {
        let jvm_dir = Path::new("/usr/lib/jvm");
        if let Ok(entries) = std::fs::read_dir(jvm_dir) {
            for entry in entries.flatten() {
                let java_exe = entry.path().join("bin").join("java");
                check(&java_exe);
            }
        }
    }

    if cfg!(target_os = "macos") {
        let jvm_dir = Path::new("/Library/Java/JavaVirtualMachines");
        if let Ok(entries) = std::fs::read_dir(jvm_dir) {
            for entry in entries.flatten() {
                let java_exe = entry.path().join("Contents/Home/bin/java");
                check(&java_exe);
            }
        }
    }

    if cfg!(windows) {
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into());
        let pf86 =
            std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".into());
        let dirs = vec![
            Path::new(&pf).join("Java"),
            Path::new(&pf).join("Eclipse Adoptium"),
            Path::new(&pf86).join("Java"),
        ];

        for root in dirs {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let java_exe = entry.path().join("bin").join("java.exe");
                    check(&java_exe);
                    // Adoptium sometimes puts it in jdk-... / jre-...
                    if entry.path().join("jdk").exists() {
                        let java_exe = entry.path().join("jdk").join("bin").join("java.exe");
                        check(&java_exe);
                    }
                }
            }
        }
    }

    let mc_java_dir = crate::helper::get_app_dir().join("java");
    if let Ok(entries) = std::fs::read_dir(mc_java_dir) {
        for entry in entries.flatten() {
            let java_exe =
                entry
                    .path()
                    .join("bin")
                    .join(if cfg!(windows) { "java.exe" } else { "java" });
            check(&java_exe);

            // Check macOS internal Adoptium structure
            let java_mac = entry
                .path()
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java");
            check(&java_mac);

            // Check if there are subdirs (Adoptium extracts into jdk-...)
            if entry.path().is_dir() {
                if let Ok(sub) = std::fs::read_dir(entry.path()) {
                    for subd in sub.flatten() {
                        let java_exe = subd.path().join("bin").join(if cfg!(windows) {
                            "java.exe"
                        } else {
                            "java"
                        });
                        check(&java_exe);
                        let java_mac = subd
                            .path()
                            .join("Contents")
                            .join("Home")
                            .join("bin")
                            .join("java");
                        check(&java_mac);
                    }
                }
            }
        }
    }

    installs.sort_by_key(|j| j.version);
    installs.reverse();
    installs
}

#[tauri::command]
#[specta::specta]
pub async fn download_java(version: u8, app: AppHandle) -> Result<String, String> {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x32"
    };

    // Use Assets API to get download link and SHA256
    let api_url = format!(
        "https://api.adoptium.net/v3/assets/latest/{}/hotspot?architecture={}&image_type=jdk&os={}&vendor=eclipse",
        version, arch, os
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Adoptium API error: HTTP {}", response.status()));
    }

    let releases: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // assets/latest returns an array
    let release = releases.get(0).ok_or("No releases found")?;

    let binary = release.get("binary").ok_or("No binary info found")?;
    let package = binary.get("package").ok_or("No package info found")?;
    let url = package
        .get("link")
        .and_then(|v| v.as_str())
        .ok_or("No download link")?;
    let sha256 = package
        .get("checksum")
        .and_then(|v| v.as_str())
        .ok_or("No checksum found")?;

    let base_dir = crate::helper::get_app_dir().join("java");
    std::fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

    let dest_folder = base_dir.join(format!("jdk-{}", version));

    // Improved existing Java check
    if dest_folder.exists() {
        if let Some(p) = find_java_in_dir(&dest_folder) {
            // Verify it actually works
            if let Some(v) = get_java_version(&p.to_string_lossy()) {
                if v == version {
                    return Ok(p.to_string_lossy().to_string());
                }
            }
        }
        // If it exists but is broken or wrong version, we will re-download
        let _ = std::fs::remove_dir_all(&dest_folder);
    }

    let _ = app.emit(
        "java-download-progress",
        DownloadProgress {
            completed: 0,
            total: 100,
            phase: "java".to_string(),
        },
    );

    let mut response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let total_size = response.content_length().unwrap_or(0);

    let archive_path = base_dir.join(format!(
        "jdk-{}-archive{}",
        version,
        if cfg!(windows) { ".zip" } else { ".tar.gz" }
    ));

    {
        let mut file = std::fs::File::create(&archive_path).map_err(|e| e.to_string())?;
        let mut downloaded: u64 = 0;
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            use std::io::Write;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;

            let percent = if total_size > 0 {
                (downloaded * 100 / total_size) as usize
            } else {
                0
            };
            let _ = app.emit(
                "java-download-progress",
                DownloadProgress {
                    completed: percent,
                    total: 100,
                    phase: "java".to_string(),
                },
            );
        }
    }

    // Verify SHA256
    let _ = app.emit(
        "java-download-progress",
        DownloadProgress {
            completed: 0,
            total: 1,
            phase: "java-verifying".to_string(),
        },
    );

    if !crate::helper::verify_file_sha256(&archive_path.to_string_lossy(), sha256)
        .map_err(|e| e.to_string())?
    {
        let _ = std::fs::remove_file(&archive_path);
        return Err("Java download checksum mismatch".to_string());
    }

    let _ = app.emit(
        "java-download-progress",
        DownloadProgress {
            completed: 0,
            total: 1,
            phase: "java-extracting".to_string(),
        },
    );

    // Extract to temp folder then rename for atomicity
    let temp_extract_dir = base_dir.join(format!("jdk-{}-tmp", version));
    if temp_extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_extract_dir);
    }
    std::fs::create_dir_all(&temp_extract_dir).map_err(|e| e.to_string())?;

    if cfg!(windows) {
        let zip_file = std::fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;
        archive
            .extract(&temp_extract_dir)
            .map_err(|e| e.to_string())?;
    } else {
        use flate2::read::GzDecoder;
        use tar::Archive;
        let tar_gz = std::fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive
            .unpack(&temp_extract_dir)
            .map_err(|e| e.to_string())?;
    }

    let _ = std::fs::remove_file(&archive_path);

    // Atomic rename
    std::fs::rename(&temp_extract_dir, &dest_folder).map_err(|e| e.to_string())?;

    let p = find_java_in_dir(&dest_folder)
        .ok_or_else(|| "Extracted Java executable not found".to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&p) {
            let mut perms = metadata.permissions();
            perms.set_mode(perms.mode() | 0o111);
            let _ = std::fs::set_permissions(&p, perms);
        }
    }

    let _ = app.emit(
        "java-download-progress",
        DownloadProgress {
            completed: 1,
            total: 1,
            phase: format!("Extracted Java {}", version),
        },
    );

    Ok(p.to_string_lossy().to_string())
}

fn find_java_in_dir(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let java_bin = if cfg!(windows) { "java.exe" } else { "java" };

    // Direct bin/java
    let direct = dir.join("bin").join(java_bin);
    if direct.exists() {
        return Some(direct);
    }

    // macOS structure
    let mac = dir.join("Contents").join("Home").join("bin").join("java");
    if mac.exists() {
        return Some(mac);
    }

    // Adoptium often extracts into a subfolder like jdk-21.0.2+13
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let check = entry.path().join("bin").join(java_bin);
                if check.exists() {
                    return Some(check);
                }
                let check_mac = entry
                    .path()
                    .join("Contents")
                    .join("Home")
                    .join("bin")
                    .join("java");
                if check_mac.exists() {
                    return Some(check_mac);
                }
            }
        }
    }
    None
}

#[tauri::command]
#[specta::specta]
pub fn get_required_java_version(mc_version: &str) -> u8 {
    let parts: Vec<&str> = mc_version.split('.').collect();
    if parts.len() >= 2 {
        let minor = parts[1].parse::<u32>().unwrap_or(0);
        let patch = if parts.len() >= 3 {
            parts[2].parse::<u32>().unwrap_or(0)
        } else {
            0
        };

        if minor >= 21 {
            return 21;
        } else if minor == 20 && patch >= 5 {
            return 21;
        } else if minor >= 17 {
            return 17;
        }
    }
    8
}

#[tauri::command]
#[specta::specta]
pub fn get_local_java_version(path: String) -> Option<u8> {
    get_java_version(&path)
}

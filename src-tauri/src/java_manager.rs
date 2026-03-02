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

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/hotspot/{}/{}/jdk/normal/eclipse",
        version, os, arch
    );

    let base_dir = crate::helper::get_app_dir().join("java");
    std::fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

    let dest_folder = base_dir.join(format!("jdk-{}", version));

    // Try to find an existing valid bin/java in dest_folder
    if let Ok(entries) = std::fs::read_dir(&dest_folder) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let check_path =
                    entry
                        .path()
                        .join("bin")
                        .join(if cfg!(windows) { "java.exe" } else { "java" });
                if check_path.exists() {
                    return Ok(check_path.to_string_lossy().to_string());
                }
                let check_path_mac = entry
                    .path()
                    .join("Contents")
                    .join("Home")
                    .join("bin")
                    .join("java");
                if check_path_mac.exists() {
                    return Ok(check_path_mac.to_string_lossy().to_string());
                }
            }
        }
    }

    let _ = app.emit(
        "java-download-progress",
        DownloadProgress {
            completed: 0,
            total: 100,
            phase: format!("Downloading Java {}", version),
        },
    );

    let mut response = reqwest::get(&url).await.map_err(|e| e.to_string())?;

    // Check if the request failed
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download Java. HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);

    let archive_path = base_dir.join(format!(
        "jdk-{}/archive{}",
        version,
        if cfg!(windows) { ".zip" } else { ".tar.gz" }
    ));
    std::fs::create_dir_all(archive_path.parent().unwrap()).unwrap();

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
                phase: format!("Downloading Java {}", version),
            },
        );
    }

    let _ = app.emit(
        "java-download-progress",
        DownloadProgress {
            completed: 0,
            total: 1,
            phase: format!("Extracting Java {}", version),
        },
    );

    if cfg!(windows) {
        let zip_file = std::fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;
        archive.extract(&dest_folder).map_err(|e| e.to_string())?;
    } else {
        use flate2::read::GzDecoder;
        use tar::Archive;
        let tar_gz = std::fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&dest_folder).map_err(|e| e.to_string())?;
    }

    let _ = std::fs::remove_file(&archive_path);

    let mut java_bin_path = None;
    if let Ok(entries) = std::fs::read_dir(&dest_folder) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let check_path =
                    entry
                        .path()
                        .join("bin")
                        .join(if cfg!(windows) { "java.exe" } else { "java" });
                if check_path.exists() {
                    java_bin_path = Some(check_path);
                    break;
                }

                let check_path_mac = entry
                    .path()
                    .join("Contents")
                    .join("Home")
                    .join("bin")
                    .join("java");
                if check_path_mac.exists() {
                    java_bin_path = Some(check_path_mac);
                    break;
                }
            }
        }
    }

    let p = java_bin_path.ok_or_else(|| "Extracted Java executable not found".to_string())?;

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

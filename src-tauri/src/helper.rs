use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use tokio::fs;

pub fn get_app_dir() -> PathBuf {
    let home = || PathBuf::from(shellexpand::tilde("~").as_ref());
    if cfg!(windows) {
        dirs::data_dir().unwrap_or_else(home).join(".mc")
    } else if cfg!(target_os = "macos") {
        dirs::data_dir().unwrap_or_else(home).join(".mc")
    } else {
        dirs::home_dir().unwrap_or_else(home).join(".mc")
    }
}

// Expand ~ in user-provided paths
pub fn expand_path(path: &str) -> String {
    shellexpand::tilde(path).into_owned()
}

/// Join an untrusted relative path (from a modpack index, zip entry, etc.)
/// onto `base`, refusing anything that could escape it: absolute paths,
/// drive prefixes, and `..` components. Returns `None` for unsafe input.
pub fn safe_join(base: &Path, untrusted: &str) -> Option<PathBuf> {
    use std::path::Component;

    let normalized = untrusted.replace('\\', "/");
    let rel = Path::new(&normalized);
    let mut out = base.to_path_buf();
    let mut pushed_any = false;
    for component in rel.components() {
        match component {
            Component::Normal(part) => {
                out.push(part);
                pushed_any = true;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    pushed_any.then_some(out)
}

/// Write a file atomically: write to a sibling temp file, then rename over
/// the target. A crash mid-write leaves the old file intact.
pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("Invalid file path: {}", path.display()))?;
    let tmp_path = path.with_file_name(format!(".{}.tmp", file_name));
    {
        let mut file = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        file.write_all(contents).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        e.to_string()
    })
}

/// Returns true when `path` is a real instance folder that is safe to delete
/// recursively: it must live strictly inside one of the launcher's
/// `instances` roots, never be a root itself, and never be the home dir.
pub fn is_deletable_instance_dir(path: &Path, extra_roots: &[PathBuf]) -> bool {
    let Ok(target) = path.canonicalize() else {
        return false;
    };

    let mut roots: Vec<PathBuf> = vec![get_app_dir().join("instances")];
    if let Some(home) = dirs::home_dir() {
        // The profile editor historically defaulted to ~/.mc/instances on every OS.
        roots.push(home.join(".mc").join("instances"));
        if target == home.canonicalize().unwrap_or(home) {
            return false;
        }
    }
    roots.extend(extra_roots.iter().map(|r| r.join("instances")));

    roots.iter().any(|root| match root.canonicalize() {
        Ok(root) => target != root && target.starts_with(&root),
        Err(_) => false,
    })
}

pub async fn download_file(url: &str, target_path: &str, sha1: &str) -> Result<(), String> {
    let path = Path::new(target_path);

    // Skip verification
    if path.exists() {
        if sha1.is_empty() {
            return Ok(());
        }
        if tokio::task::spawn_blocking({
            let p = target_path.to_string();
            let h = sha1.to_string();
            move || verify_file(&p, &h).unwrap_or(false)
        })
        .await
        .unwrap_or(false)
        {
            return Ok(());
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directories for {}: {}", target_path, e))?;
        }
    }

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Network request failed for {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP Error {}: Could not download {}",
            response.status(),
            url
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read bytes from response: {}", e))?;

    fs::write(path, bytes)
        .await
        .map_err(|e| format!("Failed to write file to disk at {}: {}", target_path, e))?;

    Ok(())
}

pub async fn download_file_with_retry(
    url: &str,
    target_path: &str,
    max_retries: u32,
    sha1: &str,
) -> Result<(), String> {
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        match download_file(url, target_path, sha1).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = e;
                if attempt < max_retries {
                    eprintln!(
                        "Download attempt {}/{} failed for {}: {}. Retrying...",
                        attempt, max_retries, url, last_error
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(u64::from(attempt))).await;
                }
            }
        }
    }

    Err(format!(
        "Failed after {} attempts for {}: {}",
        max_retries, url, last_error
    ))
}

pub fn verify_file(file_path: &str, hash: &str) -> Result<bool, String> {
    let mut hasher = Sha1::new();
    let file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = hasher.finalize();
    let hex_result = format!("{:x}", result);
    Ok(hex_result == hash)
}

pub fn verify_file_sha256(file_path: &str, hash: &str) -> Result<bool, String> {
    let mut hasher = Sha256::new();
    let file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = hasher.finalize();
    let hex_result = format!("{:x}", result);
    Ok(hex_result == hash)
}

#[tauri::command]
#[specta::specta]
pub fn get_system_memory_mb() -> u32 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    (sys.total_memory() / (1024 * 1024)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_accepts_normal_paths() {
        let base = Path::new("/base");
        assert_eq!(
            safe_join(base, "mods/a.jar"),
            Some(PathBuf::from("/base/mods/a.jar"))
        );
        assert_eq!(
            safe_join(base, "./config\\b.toml"),
            Some(PathBuf::from("/base/config/b.toml"))
        );
    }

    #[test]
    fn safe_join_rejects_escapes() {
        let base = Path::new("/base");
        assert_eq!(safe_join(base, "../evil"), None);
        assert_eq!(safe_join(base, "mods/../../evil"), None);
        assert_eq!(safe_join(base, "/etc/passwd"), None);
        assert_eq!(safe_join(base, "..\\..\\evil"), None);
        assert_eq!(safe_join(base, ""), None);
        assert_eq!(safe_join(base, "."), None);
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jd-test-{}-{}", name, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn deletable_only_inside_instances_root() {
        let root = temp_root("del");
        let instances = root.join("instances");
        let inst = instances.join("abc");
        std::fs::create_dir_all(&inst).unwrap();
        let outside = root.join("worlds");
        std::fs::create_dir_all(&outside).unwrap();

        let roots = vec![root.clone()];
        assert!(is_deletable_instance_dir(&inst, &roots));
        assert!(
            !is_deletable_instance_dir(&instances, &roots),
            "root itself"
        );
        assert!(!is_deletable_instance_dir(&outside, &roots));
        assert!(!is_deletable_instance_dir(&root, &roots));
        assert!(
            !is_deletable_instance_dir(&instances.join("abc/../../worlds"), &roots),
            "traversal resolves outside"
        );
        if let Some(home) = dirs::home_dir() {
            assert!(!is_deletable_instance_dir(&home, &[home.clone()]));
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}

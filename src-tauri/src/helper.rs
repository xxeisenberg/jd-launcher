use sha1::{Digest, Sha1};
use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use tokio::fs;

pub fn get_app_dir() -> PathBuf {
    if cfg!(windows) {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mc")
    } else if cfg!(target_os = "macos") {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mc")
    } else {
        // Linux and others - use ~/.mc directly for compatibility
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mc")
    }
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

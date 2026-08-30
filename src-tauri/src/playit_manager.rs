use crate::helper::{download_file_with_retry, get_app_dir, verify_file_sha256};
use playit_ipc::ipc::IpcClient;
use playit_ipc::model::{AgentLifecycle, AgentState, ServiceUpdate};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const PLAYIT_VERSION: &str = "v1.0.10";

#[derive(Debug, Clone, Serialize)]
struct LanPortOpened {
    port: u16,
}

#[derive(Debug, Clone, Serialize)]
struct PlayitActionUrl {
    url: String,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
struct PlayitTunnelReady {
    address: String,
}

#[derive(Debug, Clone, Serialize)]
struct PlayitStatus {
    status: String,
    message: String,
}

struct PlayitProcess {
    id: u64,
    port: u16,
    child: Child,
    setup_child: Option<Child>,
}

impl PlayitProcess {
    fn terminate(mut self) {
        if let Some(mut setup_child) = self.setup_child.take() {
            let _ = setup_child.kill();
            let _ = setup_child.wait();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

static PLAYIT_PROCESS: OnceLock<Mutex<Option<PlayitProcess>>> = OnceLock::new();
static START_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static REQUEST_ID: AtomicU64 = AtomicU64::new(0);
static STARTING_PORT: AtomicU16 = AtomicU16::new(0);

fn process_slot() -> &'static Mutex<Option<PlayitProcess>> {
    PLAYIT_PROCESS.get_or_init(|| Mutex::new(None))
}

fn start_lock() -> &'static tokio::sync::Mutex<()> {
    START_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn emit_status(app: &AppHandle, status: &str, message: impl Into<String>) {
    let _ = app.emit(
        "playit-status",
        PlayitStatus {
            status: status.into(),
            message: message.into(),
        },
    );
}

pub fn emit_lan_port_opened(app: &AppHandle, port: u16) {
    let _ = app.emit("lan-port-opened", LanPortOpened { port });
}

pub async fn start_for_lan_port(app: AppHandle, port: u16) {
    if let Err(error) = start_for_lan_port_inner(app.clone(), port).await {
        emit_status(&app, "error", error);
    }
}

async fn start_for_lan_port_inner(app: AppHandle, port: u16) -> Result<(), String> {
    let _start_guard = start_lock().lock().await;

    let stale_process = {
        let mut guard = process_slot()
            .lock()
            .map_err(|_| "Playit process lock is poisoned".to_string())?;
        match guard.as_mut() {
            Some(process) if process.port == port => match process.child.try_wait() {
                Ok(None) => return Ok(()),
                Ok(Some(_)) | Err(_) => guard.take(),
            },
            _ => None,
        }
    };
    if let Some(process) = stale_process {
        process.terminate();
    }

    let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst) + 1;
    STARTING_PORT.store(port, Ordering::SeqCst);
    let paths = match ensure_playit_binaries().await {
        Ok(paths) => paths,
        Err(error) => {
            clear_starting_port(port);
            return Err(error);
        }
    };
    if REQUEST_ID.load(Ordering::SeqCst) != request_id {
        clear_starting_port(port);
        return Ok(());
    }

    emit_status(
        &app,
        "starting",
        format!("Starting Playit for LAN port {port}"),
    );

    let _ = std::fs::remove_file(&paths.socket_path);

    let previous_process = process_slot()
        .lock()
        .map_err(|_| "Playit process lock is poisoned".to_string())?
        .take();
    if let Some(process) = previous_process {
        process.terminate();
    }

    if REQUEST_ID.load(Ordering::SeqCst) != request_id {
        clear_starting_port(port);
        return Ok(());
    }

    let mut command = Command::new(&paths.daemon);
    command
        .arg("--socket-path")
        .arg(&paths.socket_path)
        .arg("--secret-path")
        .arg(&paths.secret_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            clear_starting_port(port);
            return Err(format!("Failed to start Playit: {error}"));
        }
    };

    if let Some(stdout) = child.stdout.take() {
        let app_clone = app.clone();
        std::thread::spawn(move || read_playit_output(stdout, app_clone));
    }
    if let Some(stderr) = child.stderr.take() {
        let app_clone = app.clone();
        std::thread::spawn(move || read_playit_output(stderr, app_clone));
    }

    {
        let mut guard = process_slot()
            .lock()
            .map_err(|_| "Playit process lock is poisoned".to_string())?;
        *guard = Some(PlayitProcess {
            id: request_id,
            port,
            child,
            setup_child: None,
        });
    }
    clear_starting_port(port);

    let monitor_app = app.clone();
    let socket_path = paths.socket_path.clone();
    tauri::async_runtime::spawn(async move {
        monitor_playit(monitor_app, socket_path, request_id, port).await;
    });

    if !paths.secret_path.exists() {
        if let Err(error) = wait_for_daemon(request_id, &paths.socket_path).await {
            stop_playit_for_port(&app, port);
            return Err(error);
        }
        if REQUEST_ID.load(Ordering::SeqCst) != request_id {
            return Ok(());
        }

        let setup_child = match start_setup_flow(&app, &paths) {
            Ok(child) => child,
            Err(error) => {
                stop_playit_for_port(&app, port);
                return Err(error);
            }
        };
        let mut guard = process_slot()
            .lock()
            .map_err(|_| "Playit process lock is poisoned".to_string())?;
        if let Some(process) = guard.as_mut().filter(|process| process.id == request_id) {
            process.setup_child = Some(setup_child);
        } else {
            drop(guard);
            let mut setup_child = setup_child;
            let _ = setup_child.kill();
            let _ = setup_child.wait();
        }
    }

    Ok(())
}

fn clear_starting_port(port: u16) {
    let _ = STARTING_PORT.compare_exchange(port, 0, Ordering::SeqCst, Ordering::SeqCst);
}

async fn wait_for_daemon(request_id: u64, socket_path: &Path) -> Result<(), String> {
    let socket_path = socket_path.to_string_lossy();
    for _ in 0..50 {
        if REQUEST_ID.load(Ordering::SeqCst) != request_id {
            return Ok(());
        }
        if IpcClient::is_running(&socket_path).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("Playit daemon did not become ready for setup".into())
}

pub fn stop_playit_for_port(app: &AppHandle, port: u16) {
    let should_stop = STARTING_PORT.load(Ordering::SeqCst) == port
        || process_slot()
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|process| process.port == port))
            .unwrap_or(false);
    if should_stop {
        stop_playit();
        emit_status(app, "stopped", "Playit stopped");
    }
}

pub fn stop_playit() {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst);
    STARTING_PORT.store(0, Ordering::SeqCst);
    if let Ok(mut guard) = process_slot().lock() {
        if let Some(process) = guard.take() {
            process.terminate();
        }
    }
}

struct PlayitPaths {
    daemon: PathBuf,
    cli: PathBuf,
    socket_path: PathBuf,
    secret_path: PathBuf,
}

async fn ensure_playit_binaries() -> Result<PlayitPaths, String> {
    let (daemon_asset, daemon_hash, cli_asset, cli_hash) = playit_assets()?;
    let playit_dir = get_app_dir().join("playit").join(PLAYIT_VERSION);
    let daemon_path = playit_dir.join("playitd");
    let cli_path = playit_dir.join("playit-cli");

    ensure_binary(daemon_asset, daemon_hash, &daemon_path).await?;
    ensure_binary(cli_asset, cli_hash, &cli_path).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&daemon_path, &cli_path] {
            let metadata = std::fs::metadata(path)
                .map_err(|e| format!("Failed to read Playit permissions: {e}"))?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(path, permissions)
                .map_err(|e| format!("Failed to mark Playit executable: {e}"))?;
        }
    }

    Ok(PlayitPaths {
        daemon: daemon_path,
        cli: cli_path,
        socket_path: playit_dir.join("playit.sock"),
        secret_path: playit_dir.join("playit.toml"),
    })
}

async fn ensure_binary(asset: &str, sha256: &str, path: &Path) -> Result<(), String> {
    if path.exists() && !verify_file_sha256(&path.to_string_lossy(), sha256)? {
        std::fs::remove_file(path)
            .map_err(|e| format!("Failed to replace invalid Playit binary: {e}"))?;
    }

    if !path.exists() {
        let url = format!(
            "https://github.com/playit-cloud/playit-agent/releases/download/{PLAYIT_VERSION}/{asset}"
        );
        download_file_with_retry(&url, &path.to_string_lossy(), 3, "").await?;
    }

    if !verify_file_sha256(&path.to_string_lossy(), sha256)? {
        let _ = std::fs::remove_file(path);
        return Err(format!(
            "Downloaded Playit binary failed SHA-256 verification: {asset}"
        ));
    }
    Ok(())
}

fn playit_assets() -> Result<(&'static str, &'static str, &'static str, &'static str), String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok((
            "playit-linux-amd64",
            "2df7d9f10227ab312b1ad341853db4e8a8243df5cfcdbae58713a4271711c339",
            "playit-cli-linux-amd64",
            "6fd54d147ae1d3232b22c1c1f4aa3d13cf16d889e840ca2d3f90b4f50a2e7301",
        )),
        ("linux", "aarch64") => Ok((
            "playit-linux-aarch64",
            "4c0db3e7b3a8158e249441c2f0b73f54e83429395890c7b1ca45fd7a6303d763",
            "playit-cli-linux-aarch64",
            "b126b4164c03838598c8f33f209d76f6acf1c257d07900c0af2d461b9647099f",
        )),
        ("linux", "arm") => Ok((
            "playit-linux-armv7",
            "92ec60988b1246e07ac090c663128bd04bdc0d7ff388db520e1ff7bb4e5003e0",
            "playit-cli-linux-armv7",
            "2e1140a838b42f00233065432ed36fbfe8af34e9aa22585bcb2e01fcdad282a6",
        )),
        ("linux", "x86") => Ok((
            "playit-linux-i686",
            "d7215f3995e486bc231b3b542aa5f1ac6b0d604f8dae97bb14a9a64b49b3ed50",
            "playit-cli-linux-i686",
            "e8e4bd663d0781e3d168be2a4e45d3642a38bc7946f507ba6116e8687b8a678f",
        )),
        (os, arch) => Err(format!(
            "Playit portable mode is currently supported on Linux only (detected {os} {arch})"
        )),
    }
}

fn start_setup_flow(app: &AppHandle, paths: &PlayitPaths) -> Result<Child, String> {
    let mut command = Command::new(&paths.cli);
    command
        .arg("--socket-path")
        .arg(&paths.socket_path)
        .arg("setup")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to start Playit setup: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        let app_clone = app.clone();
        std::thread::spawn(move || read_playit_output(stdout, app_clone));
    }
    if let Some(stderr) = child.stderr.take() {
        let app_clone = app.clone();
        std::thread::spawn(move || read_playit_output(stderr, app_clone));
    }

    Ok(child)
}

async fn monitor_playit(app: AppHandle, socket_path: PathBuf, request_id: u64, port: u16) {
    let socket_path = socket_path.to_string_lossy().to_string();
    let mut client = {
        let mut connected = None;
        for _ in 0..50 {
            if REQUEST_ID.load(Ordering::SeqCst) != request_id {
                return;
            }
            match IpcClient::connect_with_path(&socket_path).await {
                Ok(client) => {
                    connected = Some(client);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
        match connected {
            Some(client) => client,
            None => {
                emit_status(&app, "error", "Playit daemon did not become reachable");
                return;
            }
        }
    };

    let subscription = match client.subscribe().await {
        Ok(subscription) => subscription,
        Err(error) => {
            emit_status(&app, "error", format!("Failed to monitor Playit: {error}"));
            return;
        }
    };
    handle_lifecycle(&app, port, subscription.snapshot.lifecycle);

    loop {
        if REQUEST_ID.load(Ordering::SeqCst) != request_id {
            return;
        }
        match client.recv_update().await {
            Ok(ServiceUpdate::Lifecycle(lifecycle)) => handle_lifecycle(&app, port, lifecycle),
            Ok(_) => {}
            Err(error) => {
                if REQUEST_ID.load(Ordering::SeqCst) == request_id {
                    emit_status(
                        &app,
                        "error",
                        format!("Playit stopped unexpectedly: {error}"),
                    );
                }
                return;
            }
        }
    }
}

fn handle_lifecycle(app: &AppHandle, port: u16, lifecycle: AgentLifecycle) {
    match lifecycle {
        AgentLifecycle::WaitingForSecret => emit_status(
            app,
            "setup-required",
            "Complete Playit setup in your browser",
        ),
        AgentLifecycle::Starting => emit_status(app, "starting", "Connecting to Playit"),
        AgentLifecycle::Running(state) => handle_running_state(app, port, state),
        AgentLifecycle::HasInvalidSecret(error)
        | AgentLifecycle::DisabledOverLimit(error)
        | AgentLifecycle::Error(error) => emit_status(app, "error", error.message),
        AgentLifecycle::Stopping => emit_status(app, "stopping", "Stopping Playit"),
    }
}

fn handle_running_state(app: &AppHandle, port: u16, state: AgentState) {
    if let Some(url) = state.login_link {
        emit_action_url(app, url, "Configure");
    }

    let matching = state
        .tunnels
        .iter()
        .find(|tunnel| destination_port(&tunnel.destination) == Some(port));

    match matching {
        Some(tunnel) if !tunnel.is_disabled => {
            let _ = app.emit(
                "playit-tunnel-ready",
                PlayitTunnelReady {
                    address: tunnel.display_address.clone(),
                },
            );
            emit_status(app, "ready", "Playit tunnel is ready");
        }
        Some(tunnel) => emit_status(
            app,
            "error",
            tunnel
                .disabled_reason
                .clone()
                .unwrap_or_else(|| "The matching Playit tunnel is disabled".into()),
        ),
        None if state.tunnels.is_empty() => emit_status(
            app,
            "configuration-required",
            format!("Configure a Minecraft Java tunnel to 127.0.0.1:{port}"),
        ),
        None => emit_status(
            app,
            "configuration-required",
            format!("No Playit tunnel targets LAN port {port}; update it in Playit"),
        ),
    }
}

fn destination_port(destination: &str) -> Option<u16> {
    destination.rsplit_once(':')?.1.parse().ok()
}

fn read_playit_output<R: std::io::Read>(stream: R, app: AppHandle) {
    use std::io::{BufRead, BufReader};

    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        parse_playit_line(&app, &line);
    }
}

fn parse_playit_line(app: &AppHandle, line: &str) {
    if let Some((url, label)) = extract_action_url(line) {
        emit_action_url(app, url, label);
    }

    // Retain compatibility with older agent log formats. Current v1 tunnel state
    // is read from IPC instead of relying on human-readable output.
    if let Some(address) = extract_playit_address(line) {
        let _ = app.emit("playit-tunnel-ready", PlayitTunnelReady { address });
    }
}

fn emit_action_url(app: &AppHandle, url: String, label: &str) {
    let _ = app.emit(
        "playit-action-url",
        PlayitActionUrl {
            url,
            label: label.into(),
        },
    );
}

fn extract_action_url(line: &str) -> Option<(String, &'static str)> {
    line.split_whitespace().map(trim_token).find_map(|token| {
        if token.starts_with("https://playit.gg/claim/") {
            Some((token.to_string(), "Claim"))
        } else if token.starts_with("https://playit.gg/login/guest-account/") {
            Some((token.to_string(), "Configure"))
        } else {
            None
        }
    })
}

fn extract_playit_address(line: &str) -> Option<String> {
    line.split_whitespace()
        .map(trim_token)
        .find(|token| !token.starts_with("http") && token.contains(".playit.gg"))
        .map(str::to_string)
}

fn trim_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | ';' | '.' | ')' | '(' | '[' | ']' | '{' | '}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{destination_port, extract_action_url, extract_playit_address};

    #[test]
    fn extracts_claim_and_guest_login_urls() {
        assert_eq!(
            extract_action_url("Open https://playit.gg/claim/abc123 to continue"),
            Some(("https://playit.gg/claim/abc123".into(), "Claim"))
        );
        assert_eq!(
            extract_action_url("Guest login: https://playit.gg/login/guest-account/key"),
            Some((
                "https://playit.gg/login/guest-account/key".into(),
                "Configure"
            ))
        );
    }

    #[test]
    fn extracts_legacy_public_address() {
        assert_eq!(
            extract_playit_address("tunnel available at example.gl.joinmc.link.playit.gg:25565"),
            Some("example.gl.joinmc.link.playit.gg:25565".into())
        );
    }

    #[test]
    fn extracts_destination_ports() {
        assert_eq!(destination_port("127.0.0.1:54321"), Some(54321));
        assert_eq!(destination_port("[::1]:54321"), Some(54321));
        assert_eq!(destination_port("invalid"), None);
    }
}

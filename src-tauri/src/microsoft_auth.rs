use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;

const CLIENT_ID: &str = "e5226706-5096-431d-9516-ae48fe263401";
const AUTH_SCOPE: &str = "XboxLive.signin offline_access";

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for c in s.bytes() {
        match c {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'.' | b'_' | b'~' => {
                out.push(c as char);
            }
            _ => out.push_str(&format!("%{:02X}", c)),
        }
    }
    out
}

// Microsoft auth responses

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    message: String,
    interval: u64,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MsTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MsTokenError {
    error: String,
}

// Xbox Live structs

#[derive(Debug, Serialize)]
struct XblAuthRequest {
    #[serde(rename = "Properties")]
    properties: XblProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XblProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XblXui>,
}

#[derive(Debug, Deserialize)]
struct XblXui {
    uhs: String,
}

// XSTS structs

#[derive(Debug, Serialize)]
struct XstsAuthRequest {
    #[serde(rename = "Properties")]
    properties: XstsProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XstsProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

// Minecraft auth structs

#[derive(Debug, Serialize)]
struct McAuthRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Deserialize)]
struct McAuthResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
    skins: Option<Vec<McSkin>>,
}

#[derive(Debug, Deserialize)]
struct McSkin {
    url: String,
}

// Stored account

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MinecraftAccount {
    pub uuid: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub skin_url: Option<String>,
    pub active: bool,
}

// Frontend-facing types

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AccountsStore {
    accounts: Vec<MinecraftAccount>,
    device_code: Option<String>,
    device_code_interval: Option<u64>,
    device_code_expires: Option<u64>,
}

fn accounts_path() -> PathBuf {
    let base = crate::helper::get_app_dir();

    base.join("accounts.json")
}

/// Serializes read-modify-write of accounts.json (see profiles::CONFIG_LOCK).
static ACCOUNTS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Prevents two token refreshes racing each other (each would consume the
/// same single-use refresh token).
static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn accounts_guard() -> std::sync::MutexGuard<'static, ()> {
    ACCOUNTS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn try_load_accounts() -> Result<AccountsStore, String> {
    let path = accounts_path();
    if !path.exists() {
        return Ok(AccountsStore::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read {}: {}", path.display(), e))?;
    match serde_json::from_str(&raw) {
        Ok(store) => Ok(store),
        Err(parse_err) => {
            let backup = path.with_file_name("accounts.corrupt.json");
            std::fs::copy(&path, &backup).map_err(|e| {
                format!(
                    "accounts.json is unreadable ({}) and could not be backed up: {}",
                    parse_err, e
                )
            })?;
            eprintln!(
                "accounts.json could not be parsed ({}); backed up to {}",
                parse_err,
                backup.display()
            );
            Ok(AccountsStore::default())
        }
    }
}

fn load_accounts() -> AccountsStore {
    let _guard = accounts_guard();
    try_load_accounts().unwrap_or_else(|e| {
        eprintln!("{}", e);
        AccountsStore::default()
    })
}

fn modify_accounts<R>(
    f: impl FnOnce(&mut AccountsStore) -> Result<R, String>,
) -> Result<R, String> {
    let _guard = accounts_guard();
    let mut store = try_load_accounts()?;
    let result = f(&mut store)?;
    let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    crate::helper::atomic_write(&accounts_path(), json.as_bytes())?;
    Ok(result)
}

/// Turn an XSTS error code into something a player can act on.
fn xsts_error_message(xerr: u64) -> String {
    match xerr {
        2148916233 => "This Microsoft account has no Xbox profile. Sign in once at xbox.com to create one, then try again.".into(),
        2148916235 => "Xbox Live is not available in your country/region.".into(),
        2148916236 | 2148916237 => "This account needs adult verification on xbox.com before it can play.".into(),
        2148916238 => "This is a child account. An adult must add it to a Microsoft family before it can play.".into(),
        other => format!("Xbox sign-in was rejected (XErr {}).", other),
    }
}

// Auth flow helpers

async fn exchange_for_xbox_live(ms_token: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let body = XblAuthRequest {
        properties: XblProperties {
            auth_method: "RPS".to_string(),
            site_name: "user.auth.xboxlive.com".to_string(),
            rps_ticket: format!("d={}", ms_token),
        },
        relying_party: "http://auth.xboxlive.com".to_string(),
        token_type: "JWT".to_string(),
    };

    let resp: XblResponse = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Xbox Live auth failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Xbox Live parse failed: {}", e))?;

    let uhs = resp
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or("No UHS in Xbox Live response")?;

    Ok((resp.token, uhs))
}

async fn exchange_for_xsts(xbl_token: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let body = XstsAuthRequest {
        properties: XstsProperties {
            sandbox_id: "RETAIL".to_string(),
            user_tokens: vec![xbl_token.to_string()],
        },
        relying_party: "rp://api.minecraftservices.com/".to_string(),
        token_type: "JWT".to_string(),
    };

    let http = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("XSTS auth failed: {}", e))?;
    if !http.status().is_success() {
        let status = http.status();
        let body: serde_json::Value = http.json().await.unwrap_or_default();
        return Err(match body.get("XErr").and_then(|v| v.as_u64()) {
            Some(xerr) => xsts_error_message(xerr),
            None => format!("XSTS auth failed: HTTP {}", status),
        });
    }
    let resp: XblResponse = http
        .json()
        .await
        .map_err(|e| format!("XSTS parse failed: {}", e))?;

    let uhs = resp
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or("No UHS in XSTS response")?;

    Ok((resp.token, uhs))
}

async fn exchange_for_minecraft(xsts_token: &str, uhs: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let body = McAuthRequest {
        identity_token: format!("XBL3.0 x={};{}", uhs, xsts_token),
    };

    let http = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Minecraft auth failed: {}", e))?;
    if !http.status().is_success() {
        return Err(format!("Minecraft auth failed: HTTP {}", http.status()));
    }
    let resp: McAuthResponse = http
        .json()
        .await
        .map_err(|e| format!("Minecraft auth parse failed: {}", e))?;

    Ok(resp.access_token)
}

enum ProfileFetchError {
    /// The access token was rejected (expired or revoked).
    Unauthorized,
    /// Signed in fine, but this account does not own Minecraft.
    NotOwned,
    Other(String),
}

impl From<ProfileFetchError> for String {
    fn from(e: ProfileFetchError) -> String {
        match e {
            ProfileFetchError::Unauthorized => "Minecraft session expired".into(),
            ProfileFetchError::NotOwned => {
                "This Microsoft account does not own Minecraft: Java Edition.".into()
            }
            ProfileFetchError::Other(msg) => msg,
        }
    }
}

async fn fetch_mc_profile(mc_token: &str) -> Result<McProfileResponse, ProfileFetchError> {
    let client = reqwest::Client::new();
    let http = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_token)
        .send()
        .await
        .map_err(|e| ProfileFetchError::Other(format!("MC profile fetch failed: {}", e)))?;
    match http.status().as_u16() {
        200..=299 => {}
        401 => return Err(ProfileFetchError::Unauthorized),
        404 => return Err(ProfileFetchError::NotOwned),
        code => {
            return Err(ProfileFetchError::Other(format!(
                "MC profile fetch failed: HTTP {}",
                code
            )))
        }
    }
    let resp: McProfileResponse = http
        .json()
        .await
        .map_err(|e| ProfileFetchError::Other(format!("MC profile parse failed: {}", e)))?;
    Ok(resp)
}

// Full token exchange
async fn full_token_exchange(ms_token: &str) -> Result<MinecraftAccount, String> {
    let (xbl_token, _uhs) = exchange_for_xbox_live(ms_token).await?;
    let (xsts_token, uhs) = exchange_for_xsts(&xbl_token).await?;
    let mc_token = exchange_for_minecraft(&xsts_token, &uhs).await?;
    let profile = fetch_mc_profile(&mc_token).await?;

    let skin_url = profile
        .skins
        .as_ref()
        .and_then(|s| s.first())
        .map(|s| s.url.clone());

    // Format UUID with dashes
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &profile.id[0..8],
        &profile.id[8..12],
        &profile.id[12..16],
        &profile.id[16..20],
        &profile.id[20..32]
    );

    Ok(MinecraftAccount {
        uuid,
        username: profile.name,
        access_token: mc_token,
        refresh_token: String::new(), // set by caller
        skin_url,
        active: true,
    })
}

// Refresh using stored refresh token
async fn refresh_ms_token(refresh_token: &str) -> Result<MsTokenResponse, String> {
    let client = reqwest::Client::new();
    let body = format!(
        "client_id={}&refresh_token={}&grant_type=refresh_token&scope={}",
        urlencode(CLIENT_ID),
        urlencode(refresh_token),
        urlencode(AUTH_SCOPE)
    );

    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token refresh failed: {}", e))?;
    if !resp.status().is_success() {
        return Err("Your Microsoft sign-in has expired. Please sign in again.".to_string());
    }
    let resp: MsTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("Token refresh parse failed: {}", e))?;

    Ok(resp)
}

// Tauri commands

#[tauri::command]
#[specta::specta]
pub fn get_auth_mode() -> bool {
    crate::profiles::load_profiles().settings.online_mode
}

#[tauri::command]
#[specta::specta]
pub async fn start_ms_login() -> Result<DeviceCodeInfo, String> {
    let client = reqwest::Client::new();
    let body = format!(
        "client_id={}&scope={}",
        urlencode(CLIENT_ID),
        urlencode(AUTH_SCOPE)
    );

    let resp: DeviceCodeResponse = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Device code request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Device code parse failed: {}", e))?;

    // Stash device code for polling
    modify_accounts(|store| {
        store.device_code = Some(resp.device_code);
        store.device_code_interval = Some(resp.interval);
        store.device_code_expires = Some(resp.expires_in);
        Ok(())
    })?;

    Ok(DeviceCodeInfo {
        user_code: resp.user_code,
        verification_uri: resp.verification_uri,
        message: resp.message,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn poll_ms_login() -> Result<MinecraftAccount, String> {
    let store = load_accounts();
    let device_code = store
        .device_code
        .as_ref()
        .ok_or("No pending login")?
        .clone();
    let interval = store.device_code_interval.unwrap_or(5);
    let expires = store.device_code_expires.unwrap_or(900);

    let client = reqwest::Client::new();
    let max_attempts = expires / interval;

    for _ in 0..max_attempts {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        let body = format!(
            "grant_type={}&client_id={}&device_code={}",
            urlencode("urn:ietf:params:oauth:grant-type:device_code"),
            urlencode(CLIENT_ID),
            urlencode(&device_code)
        );

        let resp = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Poll failed: {}", e))?;

        let body = resp.text().await.map_err(|e| e.to_string())?;

        // Check if error (still pending)
        if let Ok(err) = serde_json::from_str::<MsTokenError>(&body) {
            if err.error == "authorization_pending" {
                continue;
            }
            if err.error == "expired_token" {
                return Err("Login expired. Please try again.".to_string());
            }
            if err.error == "authorization_declined" {
                return Err("Login was declined.".to_string());
            }
        }

        // Success
        let token_resp: MsTokenResponse =
            serde_json::from_str(&body).map_err(|e| format!("Token parse failed: {}", e))?;

        let mut account = full_token_exchange(&token_resp.access_token).await?;
        account.refresh_token = token_resp.refresh_token.unwrap_or_default();

        // Save account
        modify_accounts(|store| {
            store.device_code = None;
            store.device_code_interval = None;
            store.device_code_expires = None;

            // Deactivate others, add this one
            for a in &mut store.accounts {
                a.active = false;
            }
            // Replace if same UUID
            if let Some(pos) = store.accounts.iter().position(|a| a.uuid == account.uuid) {
                store.accounts[pos] = account.clone();
            } else {
                store.accounts.push(account.clone());
            }
            Ok(())
        })?;

        return Ok(account);
    }

    Err("Login timed out".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_active_account() -> Option<MinecraftAccount> {
    let store = load_accounts();
    store.accounts.into_iter().find(|a| a.active)
}

#[tauri::command]
#[specta::specta]
pub fn list_accounts() -> Vec<MinecraftAccount> {
    load_accounts().accounts
}

#[tauri::command]
#[specta::specta]
pub fn switch_account(uuid: String) -> Result<(), String> {
    modify_accounts(|store| {
        for a in &mut store.accounts {
            a.active = a.uuid == uuid;
        }
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn logout_account(uuid: String) -> Result<(), String> {
    modify_accounts(|store| {
        store.accounts.retain(|a| a.uuid != uuid);
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_active_account() -> Result<MinecraftAccount, String> {
    let _refresh = REFRESH_LOCK.lock().await;
    refresh_active_account_locked().await
}

async fn refresh_active_account_locked() -> Result<MinecraftAccount, String> {
    let account = load_accounts()
        .accounts
        .into_iter()
        .find(|a| a.active)
        .ok_or("No active account")?;

    if account.refresh_token.is_empty() {
        return Err("No refresh token stored. Please sign in again.".to_string());
    }

    let ms_resp = refresh_ms_token(&account.refresh_token).await?;
    let mut refreshed = full_token_exchange(&ms_resp.access_token).await?;
    refreshed.refresh_token = ms_resp
        .refresh_token
        .unwrap_or(account.refresh_token.clone());
    refreshed.active = true;

    modify_accounts(|store| {
        match store.accounts.iter().position(|a| a.uuid == account.uuid) {
            Some(pos) => store.accounts[pos] = refreshed.clone(),
            None => return Err("Account was signed out during refresh".to_string()),
        }
        Ok(())
    })?;

    Ok(refreshed)
}

/// Return the active account with a working Minecraft access token,
/// refreshing it through Microsoft if the stored one has expired.
/// Minecraft tokens only last ~24h, so this must run before every online launch.
pub async fn ensure_valid_active_account() -> Result<MinecraftAccount, String> {
    let _refresh = REFRESH_LOCK.lock().await;
    let account =
        get_active_account().ok_or("No Microsoft account signed in. Sign in to play online.")?;

    match fetch_mc_profile(&account.access_token).await {
        Ok(_) => Ok(account),
        Err(ProfileFetchError::Unauthorized) => refresh_active_account_locked().await,
        Err(ProfileFetchError::NotOwned) => Err(ProfileFetchError::NotOwned.into()),
        // Network trouble: launch with the stored token rather than block
        // the player; single-player works even with an expired session.
        Err(ProfileFetchError::Other(e)) => {
            eprintln!(
                "Could not validate Minecraft session ({}); using stored token",
                e
            );
            Ok(account)
        }
    }
}

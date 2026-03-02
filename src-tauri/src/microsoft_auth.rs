use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;

const ONLINE_MODE: bool = false;

const CLIENT_ID: &str = "YOUR_AZURE_CLIENT_ID";
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
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

fn load_accounts() -> AccountsStore {
    let path = accounts_path();
    if !path.exists() {
        return AccountsStore::default();
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return AccountsStore::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_accounts(store: &AccountsStore) -> Result<(), String> {
    let path = accounts_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
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

    let resp: XblResponse = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("XSTS auth failed: {}", e))?
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

    let resp: McAuthResponse = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Minecraft auth failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Minecraft auth parse failed: {}", e))?;

    Ok(resp.access_token)
}

async fn fetch_mc_profile(mc_token: &str) -> Result<McProfileResponse, String> {
    let client = reqwest::Client::new();
    let resp: McProfileResponse = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_token)
        .send()
        .await
        .map_err(|e| format!("MC profile fetch failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("MC profile parse failed: {}", e))?;
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

    let resp: MsTokenResponse = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token refresh failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Token refresh parse failed: {}", e))?;

    Ok(resp)
}

// Tauri commands

#[tauri::command]
#[specta::specta]
pub fn get_auth_mode() -> bool {
    ONLINE_MODE
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
    let mut store = load_accounts();
    store.device_code = Some(resp.device_code);
    store.device_code_interval = Some(resp.interval);
    store.device_code_expires = Some(resp.expires_in);
    save_accounts(&store)?;

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
        let mut store = load_accounts();
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
        save_accounts(&store)?;

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
    let mut store = load_accounts();
    for a in &mut store.accounts {
        a.active = a.uuid == uuid;
    }
    save_accounts(&store)
}

#[tauri::command]
#[specta::specta]
pub fn logout_account(uuid: String) -> Result<(), String> {
    let mut store = load_accounts();
    store.accounts.retain(|a| a.uuid != uuid);
    save_accounts(&store)
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_active_account() -> Result<MinecraftAccount, String> {
    let mut store = load_accounts();
    let account = store
        .accounts
        .iter_mut()
        .find(|a| a.active)
        .ok_or("No active account")?;

    if account.refresh_token.is_empty() {
        return Err("No refresh token stored".to_string());
    }

    let ms_resp = refresh_ms_token(&account.refresh_token).await?;
    let mut refreshed = full_token_exchange(&ms_resp.access_token).await?;
    refreshed.refresh_token = ms_resp
        .refresh_token
        .unwrap_or(account.refresh_token.clone());
    refreshed.active = true;

    // Update store
    if let Some(pos) = store.accounts.iter().position(|a| a.active) {
        store.accounts[pos] = refreshed.clone();
    }
    save_accounts(&store)?;

    Ok(refreshed)
}

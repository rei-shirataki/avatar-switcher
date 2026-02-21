use anyhow::{anyhow, Result};
use base64::Engine;
use once_cell::sync::OnceCell;
use reqwest::Client;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::path::PathBuf;
use std::sync::Arc;
use crate::vrchat::models::*;

pub const VRCHAT_API: &str = "https://api.vrchat.cloud/api/1";
pub const USER_AGENT: &str = "AvatarSwitcher/0.1.0 (admin@rei-shirataki.com)";

static HTTP_CLIENT: OnceCell<Client> = OnceCell::new();
static COOKIE_STORE: OnceCell<Arc<CookieStoreMutex>> = OnceCell::new();
static COOKIES_PATH: OnceCell<PathBuf> = OnceCell::new();

pub fn get_client() -> &'static Client {
    HTTP_CLIENT.get().expect("HTTP client not initialized")
}

pub async fn init(app_data_dir: PathBuf) {
    let cookies_path = app_data_dir.join("cookies.json");
    let _ = COOKIES_PATH.set(cookies_path.clone());

    #[allow(deprecated)]
    let store = if cookies_path.exists() {
        let content = std::fs::read_to_string(&cookies_path).unwrap_or_default();
        CookieStore::load_json(content.as_bytes())
            .unwrap_or_else(|_| CookieStore::new(None))
    } else {
        CookieStore::new(None)
    };

    let store = Arc::new(CookieStoreMutex::new(store));
    let _ = COOKIE_STORE.set(store.clone());

    let client = Client::builder()
        .cookie_provider(store)
        .user_agent(USER_AGENT)
        .https_only(false)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let _ = HTTP_CLIENT.set(client);
}

fn persist_cookies() {
    if let (Some(store), Some(path)) = (COOKIE_STORE.get(), COOKIES_PATH.get()) {
        let mut buf = Vec::new();
        {
            let store = store.lock().unwrap();
            #[allow(deprecated)]
            store.save_json(&mut buf).ok();
        }
        std::fs::write(path, &buf).ok();
    }
}

pub async fn login(username: &str, password: &str) -> Result<LoginResult> {
    let encoded = format!(
        "{}:{}",
        urlencoding::encode(username),
        urlencoding::encode(password)
    );
    let b64 = base64::engine::general_purpose::STANDARD.encode(encoded.as_bytes());

    let client = get_client();
    let resp = client
        .get(format!("{}/auth/user", VRCHAT_API))
        .header("Authorization", format!("Basic {}", b64))
        .send()
        .await?;

    let status = resp.status();
    let body: AuthUserResponse = resp.json().await?;

    if status == 401 {
        return Ok(LoginResult {
            success: false,
            requires_2fa: false,
            method: None,
            user: None,
            error: Some("Invalid username or password".into()),
        });
    }

    persist_cookies();

    if let Some(methods) = body.requires_two_factor_auth {
        let method = methods.first().map(|m| {
            if m == "emailOtp" { "emailOtp".to_string() } else { "totp".to_string() }
        });
        return Ok(LoginResult {
            success: false,
            requires_2fa: true,
            method,
            user: None,
            error: None,
        });
    }

    let user = VRCUser {
        id: body.id.unwrap_or_default(),
        display_name: body.display_name.unwrap_or_default(),
        current_avatar_image_url: body.current_avatar_image_url.unwrap_or_default(),
        user_icon: body.user_icon.unwrap_or_default(),
        profile_pic_override: body.profile_pic_override.unwrap_or_default(),
        status: body.status.unwrap_or_default(),
        state: body.state.unwrap_or_default(),
    };

    Ok(LoginResult {
        success: true,
        requires_2fa: false,
        method: None,
        user: Some(user),
        error: None,
    })
}

pub async fn verify_2fa(code: &str, method: &str) -> Result<bool> {
    let endpoint = if method == "emailOtp" {
        format!("{}/auth/twofactorauth/emailotp/verify", VRCHAT_API)
    } else {
        format!("{}/auth/twofactorauth/totp/verify", VRCHAT_API)
    };

    let client = get_client();
    let resp = client
        .post(&endpoint)
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("2FA verification failed"));
    }

    let body: TwoFactorVerifyResponse = resp.json().await?;
    persist_cookies();
    Ok(body.verified)
}

pub async fn get_current_user() -> Result<Option<VRCUser>> {
    let client = get_client();
    let resp = client
        .get(format!("{}/auth/user", VRCHAT_API))
        .send()
        .await?;

    if resp.status() == 401 {
        return Ok(None);
    }

    let body: AuthUserResponse = resp.json().await?;

    if body.id.is_none() || body.requires_two_factor_auth.is_some() {
        return Ok(None);
    }

    Ok(Some(VRCUser {
        id: body.id.unwrap_or_default(),
        display_name: body.display_name.unwrap_or_default(),
        current_avatar_image_url: body.current_avatar_image_url.unwrap_or_default(),
        user_icon: body.user_icon.unwrap_or_default(),
        profile_pic_override: body.profile_pic_override.unwrap_or_default(),
        status: body.status.unwrap_or_default(),
        state: body.state.unwrap_or_default(),
    }))
}

pub async fn logout() -> Result<()> {
    let client = get_client();
    client
        .put(format!("{}/logout", VRCHAT_API))
        .send()
        .await?;

    // Clear cookies
    if let Some(store) = COOKIE_STORE.get() {
        let mut store = store.lock().unwrap();
        *store = CookieStore::new(None);
    }
    if let Some(path) = COOKIES_PATH.get() {
        std::fs::remove_file(path).ok();
    }
    Ok(())
}

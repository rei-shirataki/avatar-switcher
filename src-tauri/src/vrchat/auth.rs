use anyhow::{anyhow, Result};
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use once_cell::sync::OnceCell;
use rand::{rngs::OsRng, RngCore};
use reqwest::Client;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::path::PathBuf;
use std::sync::Arc;
use crate::vrchat::models::*;

pub const VRCHAT_API: &str = "https://api.vrchat.cloud/api/1";
pub const USER_AGENT: &str = "AvatarSwitcher/0.1.0 (admin@rei-shirataki.com)";

static HTTP_CLIENT: OnceCell<Client> = OnceCell::new();
static COOKIE_STORE: OnceCell<Arc<CookieStoreMutex>> = OnceCell::new();
/// Primary storage path: `cookies.enc` (encrypted binary).
/// Falls back to `cookies.json` when the OS keychain is unavailable.
static COOKIES_PATH: OnceCell<PathBuf> = OnceCell::new();
/// Primary path for the encryption key (reliable across restarts).
static KEY_FILE_PATH: OnceCell<PathBuf> = OnceCell::new();
/// Per-installation encryption key, cached after first retrieval.
/// Primary source: `key.bin` file (always written on first use).
/// Secondary source: OS keychain (migration / additional backup).
static ENCRYPTION_KEY: OnceCell<Option<[u8; 32]>> = OnceCell::new();

const KEYRING_SERVICE: &str = "avatar-switcher";
const KEYRING_ACCOUNT: &str = "cookie-encryption-key";
/// ChaCha20-Poly1305 nonce length (96 bits).
const NONCE_LEN: usize = 12;

pub fn get_client() -> &'static Client {
    HTTP_CLIENT.get().expect("HTTP client not initialized")
}

// ── Encryption helpers ────────────────────────────────────────────────────────

/// Retrieve the per-installation encryption key.
///
/// Priority:
/// 1. OS keychain — primary, most secure.
/// 2. `key.bin` file — fallback for environments without a keychain.
///
/// Returns `None` only when all sources fail; cookies are then stored as
/// plain JSON.
fn get_encryption_key() -> Option<&'static [u8; 32]> {
    ENCRYPTION_KEY.get_or_init(init_encryption_key).as_ref()
}

fn init_encryption_key() -> Option<[u8; 32]> {
    // Try keyring first (more secure), then key.bin as offline backup.
    let key = try_keyring_key_load().or_else(|| try_file_key_load());

    if let Some(key) = key {
        // Always persist to BOTH locations so that a future keyring failure
        // does not invalidate cookies.enc.  If one location already has the
        // correct value this is a cheap no-op.
        let _ = try_keyring_save(&key);
        let _ = save_key_to_file(&key);
        return Some(key);
    }

    // No persisted key found — generate a fresh one and save to both.
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    log::info!("Generated new encryption key.");

    let in_keyring = try_keyring_save(&key);
    let in_file = save_key_to_file(&key);

    if in_keyring || in_file {
        Some(key)
    } else {
        log::warn!("Cannot persist encryption key. Cookies will be stored unencrypted.");
        None
    }
}

/// Load an existing key from `key.bin`. Returns `None` if the file is absent
/// or malformed (does NOT generate a new key).
fn try_file_key_load() -> Option<[u8; 32]> {
    let key_path = KEY_FILE_PATH.get()?;
    let bytes = std::fs::read(key_path).ok()?;
    if bytes.len() != 32 {
        log::warn!("key.bin has unexpected length {}; regenerating key.", bytes.len());
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    log::info!("Loaded encryption key from key.bin.");
    Some(key)
}

/// Load an existing key from the OS keychain. Returns `None` if the keychain
/// is unavailable or no valid key is stored (does NOT generate a new key).
fn try_keyring_key_load() -> Option<[u8; 32]> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| log::warn!("Keyring unavailable: {e}."))
        .ok()?;
    let b64 = entry.get_password().ok()?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| log::warn!("Keyring key is not valid base64: {e}."))
        .ok()?;
    if bytes.len() != 32 {
        log::warn!("Keyring key has unexpected length {}; ignoring.", bytes.len());
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Some(key)
}

/// Write `key` to `key.bin`. Returns `true` on success.
fn save_key_to_file(key: &[u8; 32]) -> bool {
    let Some(key_path) = KEY_FILE_PATH.get() else {
        return false;
    };
    match std::fs::write(key_path, key.as_slice()) {
        Ok(_) => { log::info!("Encryption key saved to key.bin."); true }
        Err(e) => { log::warn!("Cannot write key.bin: {e}."); false }
    }
}

/// Save `key` to the OS keychain (best-effort; failures are only logged).
fn try_keyring_save(key: &[u8; 32]) -> bool {
    let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT) else {
        return false;
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(key);
    match entry.set_password(&b64) {
        Ok(_) => {
            log::info!("Encryption key saved to OS keyring.");
            true
        }
        Err(e) => {
            log::warn!("Cannot persist key to keyring: {e}.");
            false
        }
    }
}

/// Encrypt `plaintext` with ChaCha20-Poly1305.
/// Output layout: `nonce (12 bytes) || ciphertext`.
fn encrypt_cookies(plaintext: &[u8]) -> Option<Vec<u8>> {
    let key = get_encryption_key()?;
    let cipher = ChaCha20Poly1305::new_from_slice(key).ok()?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).ok()?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Some(out)
}

/// Decrypt data previously produced by `encrypt_cookies`.
fn decrypt_cookies(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() <= NONCE_LEN {
        return None;
    }
    let key = get_encryption_key()?;
    let nonce = Nonce::from_slice(&data[..NONCE_LEN]);
    let cipher = ChaCha20Poly1305::new_from_slice(key).ok()?;
    cipher.decrypt(nonce, &data[NONCE_LEN..]).ok()
}

// ── Cookie persistence ────────────────────────────────────────────────────────

/// Cookie ストアをディスクから読み込む。
///
/// 優先順位:
/// 1. `cookies.enc` — 暗号化バイナリ（通常）。
/// 2. `cookies.json` — 暗号化不可時のフォールバック。
///
/// `cookies.json` はここでは削除しない。`persist_cookies()` が暗号化書き込み
/// 成功後に削除することで、書き込み途中の失敗でクッキーを失わないようにする。
fn load_cookie_store(enc_path: &PathBuf, legacy_path: &PathBuf) -> CookieStore {
    // Try encrypted store first.
    if enc_path.exists() {
        if let Ok(data) = std::fs::read(enc_path) {
            if let Some(json) = decrypt_cookies(&data) {
                return deserialize_cookie_store(&json);
            }
        }
        // cookies.enc exists but failed to load — fall through to the plaintext backup.
        log::warn!("cookies.enc の読み込みに失敗しました。cookies.json にフォールバックします。");
    }

    // Plaintext fallback (migration path or keyring-unavailable path).
    if legacy_path.exists() {
        let content = std::fs::read_to_string(legacy_path).unwrap_or_default();
        return deserialize_cookie_store(content.as_bytes());
        // Deletion of cookies.json is deferred to persist_cookies().
    }

    CookieStore::new(None)
}

/// Cookie ストアをバイト列（JSON 配列 `[{"n":"name","v":"value"}, ...]`）から復元する。
fn deserialize_cookie_store(data: &[u8]) -> CookieStore {
    let Ok(entries) = serde_json::from_slice::<Vec<serde_json::Value>>(data) else {
        return CookieStore::new(None);
    };
    let url = reqwest::Url::parse("https://api.vrchat.cloud/api/1/auth/user")
        .expect("hardcoded URL is always valid");
    let mut store = CookieStore::new(None);
    for entry in &entries {
        if let (Some(name), Some(value)) = (entry["n"].as_str(), entry["v"].as_str()) {
            store.parse(&format!("{}={}; Path=/", name, value), &url).ok();
        }
    }
    store
}

fn persist_cookies() {
    let (Some(store), Some(enc_path)) = (COOKIE_STORE.get(), COOKIES_PATH.get()) else {
        return;
    };
    let mut buf = Vec::new();
    {
        let store = store.lock().unwrap();
        // `save_json()` (deprecated) only serializes persistent cookies (those with an explicit
        // Expires / Max-Age).  VRChat's `auth` cookie is a *session* cookie with no expiry, so
        // it was silently dropped every time — causing the user to be logged out on each restart.
        //
        // `iter_any()` returns all cookies including session (non-persistent) ones.
        // We serialize them manually as a JSON array of {n, v} objects.
        let cookies: Vec<serde_json::Value> = store
            .iter_any()
            .map(|c| serde_json::json!({"n": c.name(), "v": c.value()}))
            .collect();
        serde_json::to_writer(&mut buf, &cookies).ok();
    }

    let plain_path = enc_path.with_extension("json");

    if let Some(encrypted) = encrypt_cookies(&buf) {
        // Verify the round-trip in-process before committing to disk.
        // This catches cases where the key is available now but the
        // encrypt→decrypt cycle has a bug.
        let round_trip_ok = decrypt_cookies(&encrypted)
            .map(|dec| dec == buf)
            .unwrap_or(false);

        if round_trip_ok && std::fs::write(enc_path, &encrypted).is_ok() {
            // Encrypted file confirmed good: it is now safe to remove
            // the plaintext backup (if any).
            std::fs::remove_file(&plain_path).ok();
            return;
        }
    }

    // Encryption unavailable or round-trip failed: persist as plain JSON.
    std::fs::write(&plain_path, &buf).ok();
}

// ── Initialisation ────────────────────────────────────────────────────────────

pub async fn init(app_data_dir: PathBuf) {
    let enc_path = app_data_dir.join("cookies.enc");
    let legacy_path = app_data_dir.join("cookies.json");
    let key_path = app_data_dir.join("key.bin");

    let _ = COOKIES_PATH.set(enc_path.clone());
    let _ = KEY_FILE_PATH.set(key_path);

    let store = load_cookie_store(&enc_path, &legacy_path);
    let store = Arc::new(CookieStoreMutex::new(store));
    let _ = COOKIE_STORE.set(store.clone());

    let client = Client::builder()
        .cookie_provider(store)
        .user_agent(USER_AGENT)
        .https_only(true)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    // Set HTTP_CLIENT FIRST so that any IPC command that arrives before
    // cookie migration finishes can still call get_client() without panicking.
    let _ = HTTP_CLIENT.set(client);

    // Persist cookies in a blocking thread so we don't starve the async
    // executor.  This handles the migration cookies.json → cookies.enc and
    // ensures an already-logged-in session survives the next restart even
    // when the user never triggers login() in this session.
    tokio::task::spawn_blocking(persist_cookies).await.ok();
}

// ── Auth API ──────────────────────────────────────────────────────────────────

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

    // Check authentication failure before attempting JSON parse so that a
    // non-standard 401 body never causes a misleading parse error.
    if status == 401 {
        return Ok(LoginResult {
            success: false,
            requires_2fa: false,
            method: None,
            user: None,
            error: Some("Invalid username or password".into()),
        });
    }

    let body: AuthUserResponse = resp.json().await?;
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
    // init() runs as a background task and touches Windows Credential Manager
    // (blocking) before HTTP_CLIENT is set.  Poll until it is ready so that a
    // fast frontend call during startup never hits the get_client() panic.
    for _ in 0..100 {
        if HTTP_CLIENT.get().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let client = HTTP_CLIENT
        .get()
        .ok_or_else(|| anyhow!("HTTP client initialization timed out"))?;
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
    // Best-effort: invalidate session server-side. Ignore errors so that a network
    // failure cannot prevent local cookie cleanup.
    let _ = client
        .put(format!("{}/logout", VRCHAT_API))
        .send()
        .await;

    // Always clear local cookies regardless of server response.
    if let Some(store) = COOKIE_STORE.get() {
        let mut store = store.lock().unwrap();
        *store = CookieStore::new(None);
    }
    if let Some(enc_path) = COOKIES_PATH.get() {
        std::fs::remove_file(enc_path).ok();
        // Also remove the plain-text fallback file if it was ever written.
        let plain_path = enc_path.with_extension("json");
        std::fs::remove_file(plain_path).ok();
    }
    Ok(())
}

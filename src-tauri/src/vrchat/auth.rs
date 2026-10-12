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
use crate::vrchat::cache::atomic_write;
use crate::vrchat::models::*;

pub const VRCHAT_API: &str = "https://api.vrchat.cloud/api/1";
// VRChat API は User-Agent に連絡先の明示を要求するが、平文メールはスパム標的になる。
// ドメインのみ記載し、必要に応じて当該ドメインの contact ページから連絡可能にしておく。
pub const USER_AGENT: &str = "AvatarSwitcher/0.1.0 (rei-shirataki.com)";

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

// ── ユーティリティ ────────────────────────────────────────────────────────────

/// `s` を最大 `max` 文字（char 単位）まで切り詰める。
/// `&s[..n]` は n がマルチバイト境界でないと panic するため、ログや anyhow!
/// メッセージにレスポンス本文を埋め込む際は必ず本関数を使う。
pub fn truncate_for_log(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

// ── レート制限対応リトライ ────────────────────────────────────────────────────

/// VRChat / S3 への HTTP リクエストを 429 や一過性の 5xx で再試行する。
///
/// VRChat 公式ガイドラインに沿い、初回 1 秒・最大 30 秒の指数バックオフで最大
/// `MAX_ATTEMPTS` 回まで再試行する。429 以外（4xx 一般）は即座に呼び出し側へ
/// 返し、判断を委ねる。
///
/// `send` クロージャは毎回新しい `RequestBuilder` を組み立てる必要があるため、
/// 呼び出し側でリクエストごとに closure 内で `client.method(...).send()` を
/// 呼び直すこと。
pub async fn send_with_retry<F, Fut>(send: F, context: &str) -> reqwest::Result<reqwest::Response>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = reqwest::Result<reqwest::Response>>,
{
    const MAX_ATTEMPTS: u32 = 5;
    let mut delay_ms: u64 = 1000;
    let mut last_err: Option<reqwest::Error> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        match send().await {
            Ok(resp) => {
                let status = resp.status();
                let retryable =
                    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
                if !retryable || attempt == MAX_ATTEMPTS {
                    return Ok(resp);
                }
                log::warn!(
                    "{}: HTTP {} (試行 {}/{}), {} ms 待機後に再試行します。",
                    context,
                    status.as_u16(),
                    attempt,
                    MAX_ATTEMPTS,
                    delay_ms
                );
            }
            Err(e) => {
                if attempt == MAX_ATTEMPTS {
                    return Err(e);
                }
                log::warn!(
                    "{}: 通信エラー (試行 {}/{}): {}。{} ms 待機後に再試行します。",
                    context,
                    attempt,
                    MAX_ATTEMPTS,
                    e,
                    delay_ms
                );
                last_err = Some(e);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        // 指数バックオフ。上限 30 秒。
        delay_ms = delay_ms.saturating_mul(2).min(30_000);
    }
    // ループは必ず return するが、保険として最後のエラーがあれば返す。
    Err(last_err.expect("retry loop exited without sending"))
}

// ── Encryption helpers ────────────────────────────────────────────────────────

/// Retrieve the per-installation encryption key.
///
/// Priority:
/// 1. OS keychain — primary, most secure.
/// 2. `key.bin` file — fallback for environments without a working keychain
///    (removed once the key is verified readable from the keychain).
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
        persist_key(&key);
        return Some(key);
    }

    // No persisted key found — generate a fresh one and save it.
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    log::info!("Generated new encryption key.");

    if persist_key(&key) {
        Some(key)
    } else {
        log::warn!("Cannot persist encryption key. Cookies will be stored unencrypted.");
        None
    }
}

/// `key` を永続化する。キーチェーンへ保存して読み戻せた場合のみ `key.bin` を
/// 削除する。`key.bin` は cookies.enc と同じディレクトリに平文で置かれるため、
/// 残すと暗号化が無意味になる。一方でキーチェーンが保存に成功しても再起動後に
/// 読めない環境があり得るので、読み戻し検証に通らない場合は `key.bin` を
/// フォールバックとして残す（鍵を失うとログインセッションも失うため）。
fn persist_key(key: &[u8; 32]) -> bool {
    if try_keyring_save(key) && try_keyring_key_load().as_ref() == Some(key) {
        if let Some(key_path) = KEY_FILE_PATH.get() {
            std::fs::remove_file(key_path).ok();
        }
        return true;
    }
    save_key_to_file(key)
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

        if round_trip_ok && atomic_write(enc_path, &encrypted).is_ok() {
            // Encrypted file confirmed good: it is now safe to remove
            // the plaintext backup (if any).
            std::fs::remove_file(&plain_path).ok();
            return;
        }
    }

    // Encryption unavailable or round-trip failed: persist as plain JSON.
    atomic_write(&plain_path, &buf).ok();
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
    let auth_header = format!("Basic {}", b64);
    let resp = send_with_retry(
        || {
            client
                .get(format!("{}/auth/user", VRCHAT_API))
                .header("Authorization", auth_header.clone())
                .send()
        },
        "GET /auth/user (login)",
    )
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

    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "ログインに失敗しました: {} - {}",
            status,
            truncate_for_log(&text, 500)
        ));
    }

    let body: AuthUserResponse = resp.json().await?;
    persist_cookies();

    if let Some(methods) = body.requires_two_factor_auth {
        // VRChat は通常 `["totp"]` か `["emailOtp"]` を返す。emailOtp が
        // 優先されるべきユーザー（メール認証中のアカウント）が稀に両方含む
        // 場合があるため、emailOtp を優先する。リカバリーコード（`otp`）は
        // ユーザーが UI 上で明示的に選択するので、初期 method には含めない。
        let method = if methods.iter().any(|m| m == "emailOtp") {
            Some("emailOtp".to_string())
        } else {
            Some("totp".to_string())
        };
        return Ok(LoginResult {
            success: false,
            requires_2fa: true,
            method,
            user: None,
            error: None,
        });
    }

    // 利用規約未同意やメール未確認は API 側で 200 を返すが、後続の API 呼び出しが
    // 403 で全滅するため、ログイン時点でユーザーに通知できるようログに残す。
    if body.email_verified == Some(false) {
        log::warn!("VRChat アカウントのメールアドレスが未確認です。");
    }
    if body.accepted_tos_version.unwrap_or(0) == 0 {
        log::warn!("VRChat の利用規約に同意していない可能性があります。");
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
    // `otp` はリカバリーコード（TOTP デバイス紛失時のバックアップ）。
    let endpoint = match method {
        "emailOtp" => format!("{}/auth/twofactorauth/emailotp/verify", VRCHAT_API),
        "otp" => format!("{}/auth/twofactorauth/otp/verify", VRCHAT_API),
        _ => format!("{}/auth/twofactorauth/totp/verify", VRCHAT_API),
    };

    let client = get_client();
    let body_json = serde_json::json!({ "code": code });
    let resp = send_with_retry(
        || {
            client
                .post(&endpoint)
                .json(&body_json)
                .send()
        },
        "POST /auth/twofactorauth/*/verify",
    )
    .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "2FA 認証に失敗しました: {} - {}",
            status,
            truncate_for_log(&text, 500)
        ));
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
    let resp = send_with_retry(
        || client.get(format!("{}/auth/user", VRCHAT_API)).send(),
        "GET /auth/user (current)",
    )
    .await?;

    if resp.status() == 401 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        // 一時的な 5xx 等を「未ログイン」と誤認させないよう Err で返す。
        return Err(anyhow!("ユーザー情報の取得に失敗しました: {}", resp.status()));
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

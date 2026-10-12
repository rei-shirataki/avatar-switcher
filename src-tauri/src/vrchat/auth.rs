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
/// Fallback path for the encryption key, used only when the OS keychain cannot
/// hold it (and as the migration source from older versions).
static KEY_FILE_PATH: OnceCell<PathBuf> = OnceCell::new();
/// Per-installation encryption key, cached after first retrieval.
/// Primary source: OS keychain. Fallback: `key.bin`.
static ENCRYPTION_KEY: OnceCell<Option<[u8; 32]>> = OnceCell::new();

const KEYRING_SERVICE: &str = "avatar-switcher";
const KEYRING_ACCOUNT: &str = "cookie-encryption-key";
/// ChaCha20-Poly1305 nonce length (96 bits).
const NONCE_LEN: usize = 12;

/// `init()` はバックグラウンドで走り、Credential Manager（ブロッキング）に触れてから
/// HTTP_CLIENT を設定する。起動直後にフロントが IPC を呼んでも panic しないよう、
/// 初期化完了を最大 5 秒待つ。
pub async fn get_client() -> Result<&'static Client> {
    for _ in 0..100 {
        if let Some(c) = HTTP_CLIENT.get() {
            return Ok(c);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Err(anyhow!("HTTP クライアントの初期化がタイムアウトしました"))
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
///
/// 429/5xx/通信エラーを再試行する。冪等でない POST（`POST /file` 等）には
/// [`send_with_retry_rate_limited`] を使うこと。
pub async fn send_with_retry<F, Fut>(send: F, context: &str) -> reqwest::Result<reqwest::Response>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = reqwest::Result<reqwest::Response>>,
{
    send_with_policy(send, context, RetryPolicy::Idempotent).await
}

/// 冪等でない POST 向け。429（リクエスト未処理が保証される）のみ再試行し、
/// 5xx・通信エラーは処理済みの可能性があるため再送せず呼び出し側へ返す。
/// 再送すると VRChat 側に重複した file / version を作りうる。
pub async fn send_with_retry_rate_limited<F, Fut>(
    send: F,
    context: &str,
) -> reqwest::Result<reqwest::Response>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = reqwest::Result<reqwest::Response>>,
{
    send_with_policy(send, context, RetryPolicy::RateLimitOnly).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RetryPolicy {
    Idempotent,
    RateLimitOnly,
}

fn should_retry_status(policy: RetryPolicy, status: reqwest::StatusCode) -> bool {
    match policy {
        RetryPolicy::Idempotent => {
            status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
        }
        RetryPolicy::RateLimitOnly => status == reqwest::StatusCode::TOO_MANY_REQUESTS,
    }
}

/// `Retry-After`（秒指定のみ。HTTP-date 形式は無視）を待機ミリ秒へ。上限 30 秒。
fn parse_retry_after_ms(value: &str) -> Option<u64> {
    let secs: u64 = value.trim().parse().ok()?;
    Some(secs.saturating_mul(1000).min(30_000))
}

async fn send_with_policy<F, Fut>(
    send: F,
    context: &str,
    policy: RetryPolicy,
) -> reqwest::Result<reqwest::Response>
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
                if !should_retry_status(policy, status) || attempt == MAX_ATTEMPTS {
                    return Ok(resp);
                }
                // サーバーが待機時間を指示している場合はそれを優先する。
                if let Some(ms) = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_retry_after_ms)
                {
                    delay_ms = ms.max(delay_ms);
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
                if attempt == MAX_ATTEMPTS || policy == RetryPolicy::RateLimitOnly {
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
/// Returns `None` when no key can be persisted, or when the keychain is
/// unreachable and there is no `key.bin` (see `KeyPlan::NoKeyThisSession`);
/// cookies are then stored as plain JSON.
fn get_encryption_key() -> Option<&'static [u8; 32]> {
    ENCRYPTION_KEY.get_or_init(init_encryption_key).as_ref()
}

/// キーチェーンからの鍵読み込み結果。
#[derive(Debug, Clone, Copy, PartialEq)]
enum KeyringLoad {
    Found([u8; 32]),
    /// エントリが無い（初回起動・旧バージョンからの移行）、または中身が壊れていて使えない。
    Missing,
    /// キーチェーン自体にアクセスできない。一時的な障害で、保存済みの鍵が
    /// 残っている可能性があるため、新しい鍵で上書きしてはならない。
    Unavailable,
}

/// 鍵の取得方針。キーチェーンと `key.bin` の状態から決める純粋関数で、
/// I/O は `init_encryption_key` が行う。
#[derive(Debug, PartialEq)]
enum KeyPlan {
    /// キーチェーンの鍵を使う。`key.bin` が同じ鍵なら不要なので削除する。
    UseKeyring([u8; 32]),
    /// `key.bin` の鍵をキーチェーンへ移す（旧バージョンからの移行）。
    Migrate([u8; 32]),
    /// 鍵がどこにも無いので新しく生成して保存する。
    Generate,
    /// キーチェーンが使えないので `key.bin` の鍵をそのまま使い、何も書き換えない。
    UseFileOnly([u8; 32]),
    /// キーチェーンが使えず `key.bin` も無い。キーチェーンに鍵が残っている
    /// 可能性があるため、この起動では鍵を生成せず暗号化を諦める。
    NoKeyThisSession,
}

fn plan_key(keyring: KeyringLoad, file: Option<[u8; 32]>) -> KeyPlan {
    match (keyring, file) {
        (KeyringLoad::Found(key), _) => KeyPlan::UseKeyring(key),
        (KeyringLoad::Missing, Some(key)) => KeyPlan::Migrate(key),
        (KeyringLoad::Missing, None) => KeyPlan::Generate,
        (KeyringLoad::Unavailable, Some(key)) => KeyPlan::UseFileOnly(key),
        (KeyringLoad::Unavailable, None) => KeyPlan::NoKeyThisSession,
    }
}

fn init_encryption_key() -> Option<[u8; 32]> {
    let file_key = try_file_key_load();
    match plan_key(try_keyring_key_load(), file_key) {
        KeyPlan::UseKeyring(key) => {
            // key.bin は平文なので、キーチェーンと同じ鍵なら残す理由が無い。
            // 異なる場合はどちらで cookies.enc を暗号化したか判断できないため残す。
            if file_key.is_some_and(|file_key| file_key != key) {
                log::warn!("key.bin differs from the keyring key; keeping both.");
            } else {
                remove_key_file();
            }
            Some(key)
        }
        KeyPlan::Migrate(key) => {
            persist_key(&key);
            Some(key)
        }
        KeyPlan::Generate => {
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
        KeyPlan::UseFileOnly(key) => {
            log::warn!("Keyring unavailable; using key.bin without touching the keyring.");
            Some(key)
        }
        KeyPlan::NoKeyThisSession => {
            log::warn!(
                "Keyring unavailable and no key.bin. Not generating a new key so that a key \
                 still stored in the keyring is not overwritten; cookies are unencrypted this session."
            );
            None
        }
    }
}

fn remove_key_file() {
    if let Some(key_path) = KEY_FILE_PATH.get() {
        std::fs::remove_file(key_path).ok();
    }
}

/// `key` を永続化する。キーチェーンへ保存して読み戻せた場合のみ `key.bin` を
/// 削除する。`key.bin` は cookies.enc と同じディレクトリに平文で置かれるため、
/// 残すと暗号化が無意味になる。一方でキーチェーンが保存に成功しても再起動後に
/// 読めない環境があり得るので、読み戻し検証に通らない場合は `key.bin` を
/// フォールバックとして残す（鍵を失うとログインセッションも失うため）。
fn persist_key(key: &[u8; 32]) -> bool {
    if try_keyring_save(key) && try_keyring_key_load() == KeyringLoad::Found(*key) {
        remove_key_file();
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

/// Load an existing key from the OS keychain (does NOT generate a new key).
/// "No entry" and "keychain unreachable" are reported separately so that a
/// transient failure never leads to overwriting the stored key.
fn try_keyring_key_load() -> KeyringLoad {
    let entry = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT) {
        Ok(entry) => entry,
        Err(e) => {
            log::warn!("Keyring unavailable: {e}.");
            return KeyringLoad::Unavailable;
        }
    };
    let b64 = match entry.get_password() {
        Ok(b64) => b64,
        Err(keyring::Error::NoEntry) => return KeyringLoad::Missing,
        Err(e) => {
            log::warn!("Cannot read key from keyring: {e}.");
            return KeyringLoad::Unavailable;
        }
    };
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) else {
        log::warn!("Keyring key is not valid base64; ignoring.");
        return KeyringLoad::Missing;
    };
    let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) else {
        log::warn!("Keyring key has unexpected length {}; ignoring.", bytes.len());
        return KeyringLoad::Missing;
    };
    KeyringLoad::Found(key)
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
///
/// 戻り値の `bool` は「`cookies.enc` が存在するのに復号できなかった」ことを示す。
/// キーチェーンの一時障害が原因なら次回起動で読める可能性があるため、呼び出し側は
/// この場合に起動時の `persist_cookies()` で `cookies.enc` を上書きしてはならない。
fn load_cookie_store(enc_path: &PathBuf, legacy_path: &PathBuf) -> (CookieStore, bool) {
    let mut enc_unreadable = false;
    // Try encrypted store first.
    if enc_path.exists() {
        if let Ok(data) = std::fs::read(enc_path) {
            if let Some(json) = decrypt_cookies(&data) {
                return (deserialize_cookie_store(&json), false);
            }
        }
        // cookies.enc exists but failed to load — fall through to the plaintext backup.
        log::warn!("cookies.enc の読み込みに失敗しました。cookies.json にフォールバックします。");
        enc_unreadable = true;
    }

    // Plaintext fallback (migration path or keyring-unavailable path).
    if legacy_path.exists() {
        let content = std::fs::read_to_string(legacy_path).unwrap_or_default();
        // Deletion of cookies.json is deferred to persist_cookies().
        return (deserialize_cookie_store(content.as_bytes()), enc_unreadable);
    }

    (CookieStore::new(None), enc_unreadable)
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
    // persist_cookies() は常に最新のメモリ上の状態を書くので、平文で書けたなら
    // 残っている cookies.enc は古い。消さないと次回起動時に cookies.enc が優先され、
    // 新しいセッションが古いもので上書きされてしまう。
    if atomic_write(&plain_path, &buf).is_ok() {
        std::fs::remove_file(enc_path).ok();
    }
}

// ── Initialisation ────────────────────────────────────────────────────────────

pub async fn init(app_data_dir: PathBuf) {
    let enc_path = app_data_dir.join("cookies.enc");
    let legacy_path = app_data_dir.join("cookies.json");
    let key_path = app_data_dir.join("key.bin");

    let _ = COOKIES_PATH.set(enc_path.clone());
    let _ = KEY_FILE_PATH.set(key_path);

    let (store, enc_unreadable) = load_cookie_store(&enc_path, &legacy_path);
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
    //
    // cookies.enc を復号できなかった場合（キーチェーンの一時障害など）は書き込まない。
    // ここで書くと空のストアで cookies.enc を置き換え、キーチェーンが復旧しても
    // ログイン状態を取り戻せなくなる。ログインし直せば login() 側で保存される。
    if enc_unreadable {
        log::warn!("cookies.enc を復号できなかったため、起動時の保存をスキップします。");
        return;
    }
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

    let client = get_client().await?;
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

    let client = get_client().await?;
    let body_json = serde_json::json!({ "code": code });
    let resp = send_with_retry_rate_limited(
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
    let client = get_client().await?;
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
    let client = get_client().await?;
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

#[cfg(test)]
mod key_plan_tests {
    use super::*;

    const A: [u8; 32] = [1; 32];
    const B: [u8; 32] = [2; 32];

    #[test]
    fn keyring_key_wins_over_key_file() {
        assert_eq!(plan_key(KeyringLoad::Found(A), Some(B)), KeyPlan::UseKeyring(A));
        assert_eq!(plan_key(KeyringLoad::Found(A), None), KeyPlan::UseKeyring(A));
    }

    #[test]
    fn missing_entry_migrates_key_file_or_generates() {
        assert_eq!(plan_key(KeyringLoad::Missing, Some(B)), KeyPlan::Migrate(B));
        assert_eq!(plan_key(KeyringLoad::Missing, None), KeyPlan::Generate);
    }

    #[test]
    fn unavailable_keyring_never_generates_a_new_key() {
        // 一時障害でキーチェーンが読めないときに鍵を生成すると、保存済みの鍵を
        // 上書きして cookies.enc を復号できなくなる。
        assert_eq!(plan_key(KeyringLoad::Unavailable, Some(B)), KeyPlan::UseFileOnly(B));
        assert_eq!(plan_key(KeyringLoad::Unavailable, None), KeyPlan::NoKeyThisSession);
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn idempotent_policy_retries_429_and_5xx() {
        assert!(should_retry_status(RetryPolicy::Idempotent, StatusCode::TOO_MANY_REQUESTS));
        assert!(should_retry_status(RetryPolicy::Idempotent, StatusCode::BAD_GATEWAY));
        assert!(!should_retry_status(RetryPolicy::Idempotent, StatusCode::NOT_FOUND));
    }

    #[test]
    fn rate_limit_only_policy_does_not_retry_5xx() {
        assert!(should_retry_status(RetryPolicy::RateLimitOnly, StatusCode::TOO_MANY_REQUESTS));
        assert!(!should_retry_status(RetryPolicy::RateLimitOnly, StatusCode::INTERNAL_SERVER_ERROR));
    }

    #[test]
    fn retry_after_seconds_are_parsed_and_capped() {
        assert_eq!(parse_retry_after_ms("3"), Some(3000));
        assert_eq!(parse_retry_after_ms(" 120 "), Some(30_000));
        assert_eq!(parse_retry_after_ms("Wed, 21 Oct 2026 07:28:00 GMT"), None);
    }
}

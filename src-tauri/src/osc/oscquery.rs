use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SERVICE_TYPE: &str = "_oscjson._tcp.local.";
const APP_NAME: &str = "AvatarSwitcher";
const DEFAULT_VRCHAT_OSC_PORT: u16 = 9000;
const HTTP_QUERY_TIMEOUT: Duration = Duration::from_secs(3);
const HTTP_BODY_LIMIT_BYTES: u64 = 64 * 1024;
const HTTP_HANDLE_TIMEOUT: Duration = Duration::from_secs(5);
const MDNS_RETRY_DELAY: Duration = Duration::from_secs(60);

/// OSCQuery で発見した VRChat の OSC 受信ポート。
/// デフォルト 9000。mDNS 探索成功時に動的更新される。
pub static VRCHAT_OSC_PORT: AtomicU16 = AtomicU16::new(DEFAULT_VRCHAT_OSC_PORT);

/// 最後に発見した VRChat の mDNS フルネーム。
/// `ServiceRemoved` が同じ名前で来たときだけポートをデフォルトに戻す。
static CURRENT_VRCHAT_FULLNAME: Mutex<Option<String>> = Mutex::new(None);

pub fn get_vrchat_osc_port() -> u16 {
    VRCHAT_OSC_PORT.load(Ordering::Relaxed)
}

/// OSCQuery を初期化する。
///
/// - `_oscjson._tcp` として自アプリを mDNS 広告
/// - VRChat の `_oscjson._tcp` サービスを探索し、OSC ポートを自動取得
/// - OSCQuery HTTP サーバーを起動して VRChat からの問い合わせに応答
///
/// エラー時はログを残してフォールバック（ポート 9000）のまま続行する。
pub async fn start(our_osc_port: u16) -> anyhow::Result<()> {
    // 受信ソケットの bind に失敗した場合（port 0）は不正な広告を避けて中止する。
    // 探索だけは継続したいが、自アプリの広告は出さない。
    if our_osc_port == 0 {
        log::warn!("[OSCQuery] 自アプリの OSC 受信ポートが 0 のため広告をスキップします");
    }

    let http_port = start_http_server(our_osc_port).await?;
    log::info!("[OSCQuery] HTTP サーバー起動: ポート {}", http_port);

    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            log::warn!("[OSCQuery] mDNS デーモン起動失敗（OSCQuery 無効、ポート 9000 固定）: {}", e);
            return Ok(());
        }
    };

    if our_osc_port != 0 {
        register_service(&mdns, http_port, our_osc_port);
    }
    browse_for_vrchat(mdns);

    Ok(())
}

fn register_service(mdns: &ServiceDaemon, http_port: u16, our_osc_port: u16) {
    let instance_name = format!("{}-{}", APP_NAME, std::process::id());
    let host_name = get_hostname();
    let local_ip = "127.0.0.1";
    let props: &[(String, String)] = &[("oscPort".to_string(), our_osc_port.to_string())];

    let info = match ServiceInfo::new(
        SERVICE_TYPE,
        &instance_name,
        &host_name,
        local_ip,
        http_port,
        props,
    ) {
        Ok(i) => i,
        Err(e) => {
            log::warn!("[OSCQuery] ServiceInfo 作成失敗: {}", e);
            return;
        }
    };

    match mdns.register(info) {
        Ok(_) => log::info!(
            "[OSCQuery] mDNS 登録: {} (HTTP: {}, OSC: {})",
            instance_name, http_port, our_osc_port
        ),
        Err(e) => log::warn!("[OSCQuery] mDNS 登録失敗: {}", e),
    }
}

fn browse_for_vrchat(mdns: ServiceDaemon) {
    tokio::spawn(async move {
        loop {
            let receiver = match mdns.browse(SERVICE_TYPE) {
                Ok(r) => r,
                Err(e) => {
                    log::warn!(
                        "[OSCQuery] mDNS ブラウズ失敗: {} ({}秒後に再試行)",
                        e, MDNS_RETRY_DELAY.as_secs()
                    );
                    tokio::time::sleep(MDNS_RETRY_DELAY).await;
                    continue;
                }
            };
            loop {
                match receiver.recv_async().await {
                    Ok(event) => on_mdns_event(event),
                    Err(e) => {
                        log::warn!(
                            "[OSCQuery] mDNS チャンネル切断: {} ({}秒後に再 browse)",
                            e, MDNS_RETRY_DELAY.as_secs()
                        );
                        break;
                    }
                }
            }
            tokio::time::sleep(MDNS_RETRY_DELAY).await;
        }
    });
}

fn on_mdns_event(event: ServiceEvent) {
    match event {
        ServiceEvent::ServiceResolved(info) => {
            let name = info.get_fullname().to_string();
            if !name.to_lowercase().contains("vrchat") {
                return;
            }

            if let Ok(mut guard) = CURRENT_VRCHAT_FULLNAME.lock() {
                *guard = Some(name.clone());
            }

            // TXT レコード "oscPort" から直接取得
            if let Some(port_str) = info.get_properties().get_property_val_str("oscPort") {
                if let Ok(port) = port_str.parse::<u16>() {
                    VRCHAT_OSC_PORT.store(port, Ordering::Relaxed);
                    log::info!("[OSCQuery] VRChat 発見: {} → OSC ポート {}", name, port);
                    return;
                }
            }

            // TXT になければ HTTP HOST_INFO を問い合わせる
            // 同一マシン前提のため宛先は 127.0.0.1 固定。
            let http_port = info.get_port();
            let expected_name = name.clone();

            tokio::spawn(async move {
                match query_vrchat_osc_port("127.0.0.1", http_port).await {
                    Ok(port) => {
                        // 完了時点で別のインスタンスが新たに登録されていれば古い結果は捨てる。
                        let still_current = CURRENT_VRCHAT_FULLNAME
                            .lock()
                            .ok()
                            .and_then(|g| g.clone())
                            .as_deref()
                            == Some(expected_name.as_str());
                        if !still_current {
                            log::debug!(
                                "[OSCQuery] HOST_INFO 結果を破棄（より新しい発見あり）: {} → {}",
                                expected_name, port
                            );
                            return;
                        }
                        VRCHAT_OSC_PORT.store(port, Ordering::Relaxed);
                        log::info!(
                            "[OSCQuery] VRChat HOST_INFO 取得: {} → OSC ポート {}",
                            expected_name, port
                        );
                    }
                    Err(e) => log::warn!("[OSCQuery] HOST_INFO 取得失敗 ({}): {}", expected_name, e),
                }
            });
        }
        ServiceEvent::ServiceRemoved(_, name) => {
            if !name.to_lowercase().contains("vrchat") {
                return;
            }
            let matched = match CURRENT_VRCHAT_FULLNAME.lock() {
                Ok(mut guard) => {
                    if guard.as_deref() == Some(name.as_str()) {
                        *guard = None;
                        true
                    } else {
                        false
                    }
                }
                Err(_) => false,
            };
            if matched {
                log::info!(
                    "[OSCQuery] VRChat 切断: {} → OSC ポートをデフォルト({})に戻す",
                    name, DEFAULT_VRCHAT_OSC_PORT
                );
                VRCHAT_OSC_PORT.store(DEFAULT_VRCHAT_OSC_PORT, Ordering::Relaxed);
            }
        }
        _ => {}
    }
}

async fn query_vrchat_osc_port(ip: &str, http_port: u16) -> anyhow::Result<u16> {
    let url = format!("http://{}:{}/?HOST_INFO", ip, http_port);
    let client = reqwest::Client::builder()
        .timeout(HTTP_QUERY_TIMEOUT)
        .build()?;
    let resp = client.get(&url).send().await?;
    if let Some(len) = resp.content_length() {
        if len > HTTP_BODY_LIMIT_BYTES {
            anyhow::bail!("HOST_INFO レスポンスが大きすぎます: {} bytes", len);
        }
    }
    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > HTTP_BODY_LIMIT_BYTES {
        anyhow::bail!("HOST_INFO レスポンスが大きすぎます: {} bytes", bytes.len());
    }
    let v: Value = serde_json::from_slice(&bytes)?;
    let port = v["OSC_PORT"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("HOST_INFO に OSC_PORT が存在しません"))? as u16;
    Ok(port)
}

// ── HTTP サーバー ────────────────────────────────────────────────────────────

async fn start_http_server(osc_port: u16) -> anyhow::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let http_port = listener.local_addr()?.port();

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        // 接続単位で全体タイムアウトを掛けて slowloris を防ぐ。
                        let _ = tokio::time::timeout(
                            HTTP_HANDLE_TIMEOUT,
                            handle_http_request(stream, osc_port),
                        )
                        .await;
                    });
                }
                Err(e) => log::warn!("[OSCQuery] HTTP accept エラー: {}", e),
            }
        }
    });

    Ok(http_port)
}

/// 暴走防止のためのリクエストヘッダー上限。
const MAX_HTTP_HEADER_BYTES: usize = 8 * 1024;

async fn handle_http_request(mut stream: tokio::net::TcpStream, osc_port: u16) {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut tmp = [0u8; 1024];
    loop {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if buf.len() > MAX_HTTP_HEADER_BYTES {
                    return;
                }
            }
            Err(_) => return,
        }
    }

    let request = std::str::from_utf8(&buf).unwrap_or("");
    let request_line = request.lines().next().unwrap_or("");
    // リクエストライン: "METHOD URI HTTP/1.1" の URI 部のみで判定する。
    let uri = request_line.split_whitespace().nth(1).unwrap_or("");
    let query = uri.split_once('?').map(|(_, q)| q).unwrap_or("");
    let want_host_info = query
        .split('&')
        .any(|kv| kv == "HOST_INFO" || kv.starts_with("HOST_INFO="));
    let body = if want_host_info {
        serde_json::to_string(&build_host_info(osc_port)).unwrap_or_default()
    } else {
        serde_json::to_string(&build_root_node()).unwrap_or_default()
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

// ── ユーティリティ ────────────────────────────────────────────────────────────

fn get_hostname() -> String {
    let raw = std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string());
    format!("{}.local.", raw)
}

// ── OSCQuery JSON レスポンス ────────────────────────────────────────────────

fn build_host_info(osc_port: u16) -> Value {
    json!({
        "NAME": APP_NAME,
        "OSC_IP": "127.0.0.1",
        "OSC_PORT": osc_port,
        "OSC_TRANSPORT": "UDP",
        "EXTENSIONS": {
            "ACCESS": true,
            "CLIPMODE": false,
            "RANGE": false,
            "TYPE": true,
            "VALUE": false
        }
    })
}

/// OSCQuery のルートノード。CONTENTS は意図的に空。
///
/// 本アプリは `/avatar/change` を VRChat へ「送信」するだけで、現状は受信ハンドラを
/// 持たないため、受信エンドポイントを広告しない。将来 Avatar Parameter / Avatar
/// Scaling 等の受信を実装したらここに該当アドレスを足す。
fn build_root_node() -> Value {
    json!({
        "DESCRIPTION": "root node",
        "FULL_PATH": "/",
        "ACCESS": 0,
        "CONTENTS": {}
    })
}

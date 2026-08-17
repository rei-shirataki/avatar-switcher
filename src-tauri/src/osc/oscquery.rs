use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SERVICE_TYPE: &str = "_oscjson._tcp.local.";
const OSC_SERVICE_TYPE: &str = "_osc._udp.local.";
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

pub fn is_vrchat_detected() -> bool {
    CURRENT_VRCHAT_FULLNAME
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
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
    // OS のマシン名 (COMPUTERNAME.local.) は使わない。Windows 標準の mDNS
    // レスポンダー (Dnscache) が同じホスト名に全インターフェース分の
    // アドレスで応答してくるため、レコードが混ざり合い解決アドレス集合が
    // 数十秒おきに変化し続けてしまう。
    let host_name = format!("{}.local.", instance_name);
    let props: &[(String, String)] = &[("oscPort".to_string(), our_osc_port.to_string())];

    // mDNS の A レコードには実 LAN IP + ループバックの両方を載せる。
    // enable_addr_auto() は register 前の get_addresses() に反映されず、
    // 実環境で `ips=[]` になっていたため、ここで明示的に列挙する。
    // mdns-sd の host_ipv4 引数はカンマ区切りで複数 IP を指定できる。
    let ips_str = collect_ips_for_mdns()
        .iter()
        .map(|ip| ip.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // VRChat は _oscjson._tcp と _osc._udp の両方が mDNS に登録されている
    // OSCQuery リスナーにのみパラメータを送信する。片方だけだと無視される。
    register_one(mdns, SERVICE_TYPE, &instance_name, &host_name, &ips_str, http_port, props);
    register_one(mdns, OSC_SERVICE_TYPE, &instance_name, &host_name, &ips_str, our_osc_port, props);
}

fn register_one(
    mdns: &ServiceDaemon,
    service_type: &str,
    instance_name: &str,
    host_name: &str,
    ips_str: &str,
    port: u16,
    props: &[(String, String)],
) {
    let info = match ServiceInfo::new(service_type, instance_name, host_name, ips_str, port, props) {
        Ok(i) => i,
        Err(e) => {
            log::warn!("[OSCQuery] ServiceInfo 作成失敗 ({}): {}", service_type, e);
            return;
        }
    };
    let advertised = info
        .get_addresses()
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(",");
    match mdns.register(info) {
        Ok(_) => log::info!(
            "[OSCQuery] mDNS 登録: type={} name={} host={} ips=[{}] port={}",
            service_type, instance_name, host_name, advertised, port
        ),
        Err(e) => log::warn!("[OSCQuery] mDNS 登録失敗 ({}): {}", service_type, e),
    }
}

/// mDNS A レコードに載せる IPv4 群を返す。実 LAN IP + 127.0.0.1。
fn collect_ips_for_mdns() -> Vec<Ipv4Addr> {
    let mut ips = Vec::new();
    if let Some(lan) = detect_local_ipv4() {
        ips.push(lan);
    }
    ips.push(Ipv4Addr::LOCALHOST);
    ips
}

/// 外部接続用ソケットの local_addr から実 LAN IPv4 を取得する。
/// 取得できなければ None。受信ソースフィルタからも参照される。
pub fn detect_local_ipv4() -> Option<Ipv4Addr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    // 実際に送らない。route 決定のために宛先を設定するだけ。
    sock.connect("8.8.8.8:80").ok()?;
    let local = sock.local_addr().ok()?;
    match local.ip() {
        IpAddr::V4(v4) if !v4.is_unspecified() && !v4.is_loopback() => Some(v4),
        _ => None,
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
    // OSCQuery の HTTP も全インターフェイスで受ける。VRChat は mDNS の
    // A レコードに載った IP（実 LAN IP）に対して HTTP を投げてくるため。
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
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
    let (path, query) = match uri.split_once('?') {
        Some((p, q)) => (p, q),
        None => (uri, ""),
    };
    let want_host_info = query
        .split('&')
        .any(|kv| kv == "HOST_INFO" || kv.starts_with("HOST_INFO="));
    // VRChat はサブパスを直接問い合わせる場合がある (/avatar/parameters/EyeHeightAsMeters 等)。
    // 未宣言のサブパスに対しては OSCQuery 仕様どおり 404 を返す。
    // ここでルートに fallback すると、要求されたパスと異なる FULL_PATH を含む
    // JSON を 200 OK で返してしまい、VRChat が応答を破棄して以降の問い合わせを
    // 止めてしまう（HOST_INFO の EXTENSIONS が想定外のときと同様の挙動）。
    let response = if want_host_info {
        let body = serde_json::to_string(&build_host_info(osc_port)).unwrap_or_default();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    } else {
        let root = build_root_node();
        match lookup_node(&root, path) {
            Some(node) => {
                let body = serde_json::to_string(node).unwrap_or_default();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            }
            None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
        }
    };

    let _ = stream.write_all(response.as_bytes()).await;
}

// ── ユーティリティ ────────────────────────────────────────────────────────────

/// OSCQuery ツリーから path (例: "/avatar/parameters/EyeHeightAsMeters") に
/// 対応するノードを引き出す。見つからなければ None。
fn lookup_node<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path == "/" || path.is_empty() {
        return Some(root);
    }
    let mut node = root;
    for seg in path.trim_start_matches('/').split('/') {
        if seg.is_empty() {
            continue;
        }
        node = node.get("CONTENTS")?.get(seg)?;
    }
    Some(node)
}

// ── OSCQuery JSON レスポンス ────────────────────────────────────────────────

fn build_host_info(osc_port: u16) -> Value {
    // OyasumiVR / VRChat 公式 vrc-oscquery-lib と同じ EXTENSIONS 値を返す。
    // VRChat は EXTENSIONS の真偽値が想定と違うと HOST_INFO を破棄し
    // GET / を投げてこなくなる。RANGE/VALUE を true にするのが鍵。
    json!({
        "NAME": APP_NAME,
        "OSC_IP": "127.0.0.1",
        "OSC_PORT": osc_port,
        "OSC_TRANSPORT": "UDP",
        "EXTENSIONS": {
            "ACCESS":   true,
            "CLIPMODE": false,
            "RANGE":    true,
            "TYPE":     true,
            "VALUE":    true
        }
    })
}

/// OSCQuery のルートノード。
///
/// OyasumiVR と同じ最小構造。VRChat 側のパラメータ名前空間 (`/avatar/parameters/*`)
/// は VRChat 自身が公開する側なので、こちらから個別宣言する必要はない。
/// `/avatar` を Write(2) として宣言しておけば、VRChat はその配下のアドレスを
/// こちらへ送るようになる。
fn build_root_node() -> Value {
    json!({
        "FULL_PATH": "/",
        "ACCESS": 0,
        "CONTENTS": {
            "avatar": {
                "FULL_PATH": "/avatar",
                "ACCESS": 2
            }
        }
    })
}

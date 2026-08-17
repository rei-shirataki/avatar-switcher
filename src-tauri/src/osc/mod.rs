pub mod commands;
pub mod oscquery;

use once_cell::sync::Lazy;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket as TokioUdpSocket;

const DEFAULT_OSC_HOST: &str = "127.0.0.1";

/// VRChat → 本アプリへの OSC で受け取る現在のアイハイト(m)。
/// VRChat 公式の Built-in パラメータと Avatar Scaling 専用エンドポイントの両方を
/// 購読する。どちらが先に来るかは VRChat のバージョン・設定により異なる。
const OSC_ADDR_EYE_HEIGHT_PARAM: &str = "/avatar/parameters/EyeHeightAsMeters";
const OSC_ADDR_EYE_HEIGHT_DIRECT: &str = "/avatar/eyeheight";
/// Avatar Scaling の Built-in Parameters。アバター作成者が Animator の
/// Playable Layer に登録している場合のみ送信されてくる。これらが揃えば
/// `EyeHeightAsMeters / ScaleFactor` でアバター本来のプレハブ身長を逆算できる。
const OSC_ADDR_SCALE_FACTOR: &str = "/avatar/parameters/ScaleFactor";
const OSC_ADDR_SCALE_MODIFIED: &str = "/avatar/parameters/ScaleModified";
/// VRChat からのアバター切替通知。
const OSC_ADDR_AVATAR_CHANGE: &str = "/avatar/change";

const EVENT_EYE_HEIGHT: &str = "osc:eye-height";
const EVENT_SCALE_FACTOR: &str = "osc:scale-factor";
const EVENT_SCALE_MODIFIED: &str = "osc:scale-modified";
const EVENT_AVATAR_CHANGE: &str = "osc:avatar-change";

/// 送信用 UDP ソケット。プロセス全体で 1 つだけ bind して再利用する。
/// 同一マシン上の VRChat に向けて送るだけなので 127.0.0.1 に bind する。
static OSC_SOCKET: Lazy<Option<UdpSocket>> = Lazy::new(|| {
    match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => Some(s),
        Err(e) => {
            log::error!("OSC 送信ソケットの bind に失敗しました: {}", e);
            None
        }
    }
});

static OSC_HOST_OVERRIDE: Lazy<Option<String>> =
    Lazy::new(|| std::env::var("AVATAR_SWITCHER_OSC_HOST").ok());
static OSC_PORT_OVERRIDE: Lazy<Option<u16>> = Lazy::new(|| {
    std::env::var("AVATAR_SWITCHER_OSC_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
});

/// VRChat への OSC 送信先アドレスを返す。
///
/// 優先順位:
/// 1. 環境変数 `AVATAR_SWITCHER_OSC_HOST` / `AVATAR_SWITCHER_OSC_PORT`
/// 2. OSCQuery で発見した VRChat のポート（host は常に 127.0.0.1）
/// 3. デフォルト (127.0.0.1:9000)
fn get_osc_addr() -> String {
    let host = OSC_HOST_OVERRIDE
        .clone()
        .unwrap_or_else(|| DEFAULT_OSC_HOST.to_string());
    let port = OSC_PORT_OVERRIDE.unwrap_or_else(oscquery::get_vrchat_osc_port);
    format!("{}:{}", host, port)
}

pub fn send_avatar_change(avatar_id: &str) -> anyhow::Result<()> {
    let Some(socket) = OSC_SOCKET.as_ref() else {
        return Err(anyhow::anyhow!("OSC ソケットが初期化されていません"));
    };
    let packet = OscPacket::Message(OscMessage {
        addr: "/avatar/change".to_string(),
        args: vec![OscType::String(avatar_id.to_string())],
    });
    let encoded = encoder::encode(&packet)?;
    let addr = get_osc_addr();
    log::debug!("OSC /avatar/change → {} (id: {})", addr, avatar_id);
    socket.send_to(&encoded, &addr)?;
    Ok(())
}

/// VRChat の `/avatar/eyeheight` に Float 値（メートル）を送信する。
/// VRChat 側で Avatar Scaling 機能が有効な場合に視点の高さに反映される。
pub fn send_avatar_eye_height(value: f32) -> anyhow::Result<()> {
    send_float(OSC_ADDR_EYE_HEIGHT_DIRECT, value)
}

fn send_float(addr: &str, value: f32) -> anyhow::Result<()> {
    if !value.is_finite() {
        return Err(anyhow::anyhow!("値が不正です (NaN/Infinity)"));
    }
    let Some(socket) = OSC_SOCKET.as_ref() else {
        return Err(anyhow::anyhow!("OSC ソケットが初期化されていません"));
    };
    let packet = OscPacket::Message(OscMessage {
        addr: addr.to_string(),
        args: vec![OscType::Float(value)],
    });
    let encoded = encoder::encode(&packet)?;
    let dest = get_osc_addr();
    log::debug!("OSC {} → {} (value: {})", addr, dest, value);
    socket.send_to(&encoded, &dest)?;
    Ok(())
}

/// OSC 受信ソケットを bind し、受信ループを spawn する。
/// 戻り値は確保された受信ポート (OSCQuery 広告用)。失敗時は呼び出し側でフォールバック。
pub async fn init_receiver(app: AppHandle) -> anyhow::Result<u16> {
    // 0.0.0.0 で bind。OSCQuery は実 LAN IP を広告するため、VRChat からの
    // OSC パケットは LAN インターフェイス経由で届く（同一マシンでも）。
    let socket = TokioUdpSocket::bind("0.0.0.0:0").await?;
    let port = socket.local_addr()?.port();
    tokio::spawn(receive_loop(socket, app));
    Ok(port)
}

async fn receive_loop(socket: TokioUdpSocket, app: AppHandle) {
    // OSC バンドルが大きい場合に備えて十分なサイズを確保。
    // 標準的な OSC メッセージは数百バイトだが、bundle で複数まとめて来る場合がある。
    let mut buf = vec![0u8; 8192];
    // 永続的な socket エラー（NIC ダウン等）が起きたとき、毎秒の警告で
    // ログを埋めないように 30 秒に 1 回だけ吐く。
    const ERR_LOG_INTERVAL: Duration = Duration::from_secs(30);
    let mut last_err_log: Option<Instant> = None;
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, src)) => {
                // 0.0.0.0 で bind しているため LAN の任意ホストからパケットが届き得る。
                // 同一マシン前提のアプリなので、loopback と自マシンの LAN IP 以外は捨てる。
                // 外部からの偽 OSC で UI 状態・localStorage を書き換えられるのを防ぐ。
                if !is_trusted_src(&src) {
                    continue;
                }
                if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..n]) {
                    dispatch_packet(packet, &app);
                }
            }
            Err(e) => {
                let now = Instant::now();
                if last_err_log.map_or(true, |t| now.duration_since(t) >= ERR_LOG_INTERVAL) {
                    log::warn!("[OSC] recv エラー（1 秒後リトライ）: {}", e);
                    last_err_log = Some(now);
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// 受信ソースが同一マシン由来か判定する。loopback または自マシンの LAN IPv4 のみ通す。
/// 自マシンの LAN IP は起動時に 1 回だけ取得してキャッシュする。
fn is_trusted_src(src: &SocketAddr) -> bool {
    if src.ip().is_loopback() {
        return true;
    }
    static LOCAL_LAN_IP: Lazy<Option<IpAddr>> =
        Lazy::new(|| oscquery::detect_local_ipv4().map(IpAddr::V4));
    match *LOCAL_LAN_IP {
        Some(ip) => src.ip() == ip,
        None => false,
    }
}

fn dispatch_packet(packet: OscPacket, app: &AppHandle) {
    match packet {
        OscPacket::Message(msg) => dispatch_message(msg, app),
        OscPacket::Bundle(bundle) => {
            for inner in bundle.content {
                dispatch_packet(inner, app);
            }
        }
    }
}

fn dispatch_message(msg: OscMessage, app: &AppHandle) {
    match msg.addr.as_str() {
        OSC_ADDR_EYE_HEIGHT_PARAM | OSC_ADDR_EYE_HEIGHT_DIRECT => {
            if let Some(OscType::Float(v)) = msg.args.first() {
                let _ = app.emit(EVENT_EYE_HEIGHT, *v);
            }
        }
        OSC_ADDR_SCALE_FACTOR => {
            if let Some(OscType::Float(v)) = msg.args.first() {
                let _ = app.emit(EVENT_SCALE_FACTOR, *v);
            }
        }
        OSC_ADDR_SCALE_MODIFIED => {
            // VRChat は bool を OSC True/False 型タグで送るのが原則だが、
            // バージョン・経路によって Int(0/1) / Float(0.0/1.0) で届く変種に
            // 備えて寛容に受ける。取りこぼすと TS 側の ScaleModified===false
            // フォールバックが無言で死ぬため、ここの厳格さに価値はない。
            let b = match msg.args.first() {
                Some(OscType::Bool(b)) => *b,
                Some(OscType::Int(i)) => *i != 0,
                Some(OscType::Float(f)) => *f != 0.0,
                _ => return,
            };
            let _ = app.emit(EVENT_SCALE_MODIFIED, b);
        }
        OSC_ADDR_AVATAR_CHANGE => {
            if let Some(OscType::String(id)) = msg.args.first() {
                let _ = app.emit(EVENT_AVATAR_CHANGE, id.clone());
            }
        }
        _ => {}
    }
}

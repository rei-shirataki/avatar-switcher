pub mod commands;
pub mod oscquery;

use once_cell::sync::Lazy;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket as TokioUdpSocket;

const DEFAULT_OSC_HOST: &str = "127.0.0.1";

/// VRChat → 本アプリへの OSC で受け取る現在のアイハイト(m)。
const OSC_ADDR_EYE_HEIGHT_METERS: &str = "/avatar/parameters/EyeHeightAsMeters";
/// VRChat からのアバター切替通知。
const OSC_ADDR_AVATAR_CHANGE: &str = "/avatar/change";

const EVENT_EYE_HEIGHT: &str = "osc:eye-height";
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
    if !value.is_finite() {
        return Err(anyhow::anyhow!("値が不正です (NaN/Infinity)"));
    }
    let Some(socket) = OSC_SOCKET.as_ref() else {
        return Err(anyhow::anyhow!("OSC ソケットが初期化されていません"));
    };
    let packet = OscPacket::Message(OscMessage {
        addr: "/avatar/eyeheight".to_string(),
        args: vec![OscType::Float(value)],
    });
    let encoded = encoder::encode(&packet)?;
    let addr = get_osc_addr();
    log::debug!("OSC /avatar/eyeheight → {} (value: {})", addr, value);
    socket.send_to(&encoded, &addr)?;
    Ok(())
}

/// OSC 受信ソケットを bind し、受信ループを spawn する。
/// 戻り値は確保された受信ポート (OSCQuery 広告用)。失敗時は呼び出し側でフォールバック。
pub async fn init_receiver(app: AppHandle) -> anyhow::Result<u16> {
    let socket = TokioUdpSocket::bind("127.0.0.1:0").await?;
    let port = socket.local_addr()?.port();
    log::info!("[OSC] 受信ソケット bind: ポート {}", port);
    tokio::spawn(receive_loop(socket, app));
    Ok(port)
}

async fn receive_loop(socket: TokioUdpSocket, app: AppHandle) {
    // OSC バンドルが大きい場合に備えて十分なサイズを確保。
    // 標準的な OSC メッセージは数百バイトだが、bundle で複数まとめて来る場合がある。
    let mut buf = vec![0u8; 8192];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, _src)) => {
                match rosc::decoder::decode_udp(&buf[..n]) {
                    Ok((_, packet)) => dispatch_packet(packet, &app),
                    Err(e) => log::debug!("[OSC] パケットデコード失敗: {:?}", e),
                }
            }
            Err(e) => {
                log::error!("[OSC] 受信エラー: {}", e);
                // 連続エラー時の暴走防止
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
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
        OSC_ADDR_EYE_HEIGHT_METERS => {
            if let Some(OscType::Float(v)) = msg.args.first() {
                if let Err(e) = app.emit(EVENT_EYE_HEIGHT, *v) {
                    log::warn!("[OSC] eye-height イベント emit 失敗: {}", e);
                }
            }
        }
        OSC_ADDR_AVATAR_CHANGE => {
            if let Some(OscType::String(id)) = msg.args.first() {
                if let Err(e) = app.emit(EVENT_AVATAR_CHANGE, id.clone()) {
                    log::warn!("[OSC] avatar-change イベント emit 失敗: {}", e);
                }
            }
        }
        _ => {}
    }
}

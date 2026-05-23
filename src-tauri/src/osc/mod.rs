pub mod commands;
pub mod oscquery;

use once_cell::sync::Lazy;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;

const DEFAULT_OSC_HOST: &str = "127.0.0.1";

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

/// OSC 受信用ソケット。OSCQuery で広告する自アプリの受信ポート確保用。
/// 将来 Avatar Parameter / Scaling 等の受信機能を実装するための土台。
static OSC_RECEIVE_SOCKET: Lazy<Option<UdpSocket>> = Lazy::new(|| {
    match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => Some(s),
        Err(e) => {
            log::error!("OSC 受信ソケットの bind に失敗しました: {}", e);
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

/// OSCQuery に広告する自アプリの OSC 受信ポートを返す。
pub fn get_osc_receive_port() -> u16 {
    OSC_RECEIVE_SOCKET
        .as_ref()
        .and_then(|s| s.local_addr().ok())
        .map(|a| a.port())
        .unwrap_or(0)
}

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

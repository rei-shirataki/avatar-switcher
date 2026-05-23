pub mod commands;

use once_cell::sync::Lazy;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;

/// VRChat の標準 OSC 受信ポート。
const DEFAULT_OSC_HOST: &str = "127.0.0.1";
const DEFAULT_OSC_PORT: u16 = 9000;

/// 送信用 UDP ソケット。プロセス全体で 1 つだけ bind して再利用する。
/// bind 失敗時は `None` を返し、OSC 送信は no-op にする（REST 経由の
/// アバター切替は影響を受けない）。
static OSC_SOCKET: Lazy<Option<UdpSocket>> = Lazy::new(|| {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => Some(s),
        Err(e) => {
            log::error!("OSC ソケットの bind に失敗しました: {}", e);
            None
        }
    }
});

/// 送信先アドレス。`AVATAR_SWITCHER_OSC_HOST` / `AVATAR_SWITCHER_OSC_PORT` で
/// 上書きできる（VRChat を `--osc=` でポート変更しているユーザー向け）。
/// 起動時に 1 回だけ評価し、以降のプロセス生存期間中は固定。
static OSC_ADDR: Lazy<String> = Lazy::new(|| {
    let host = std::env::var("AVATAR_SWITCHER_OSC_HOST")
        .unwrap_or_else(|_| DEFAULT_OSC_HOST.to_string());
    let port = std::env::var("AVATAR_SWITCHER_OSC_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_OSC_PORT);
    log::info!("OSC 送信先: {}:{}", host, port);
    format!("{}:{}", host, port)
});

pub fn send_avatar_change(avatar_id: &str) -> anyhow::Result<()> {
    let Some(socket) = OSC_SOCKET.as_ref() else {
        return Err(anyhow::anyhow!("OSC ソケットが初期化されていません"));
    };
    let packet = OscPacket::Message(OscMessage {
        addr: "/avatar/change".to_string(),
        args: vec![OscType::String(avatar_id.to_string())],
    });
    let encoded = encoder::encode(&packet)?;
    socket.send_to(&encoded, OSC_ADDR.as_str())?;
    Ok(())
}

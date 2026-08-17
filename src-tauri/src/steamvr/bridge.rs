//! overlay-sidecar (.NET, 別プロセス) と通信する WebSocket ブリッジサーバー。
//!
//! `osc::is_trusted_src` と同じ「信頼境界を明示チェックする」流儀を踏襲する:
//! loopback 以外からの接続は即座に拒否し、さらに接続直後の最初のメッセージで
//! トークン認証を行う（トークンはサイドカー起動時の CLI 引数でのみ渡される）。

use crate::steamvr::protocol::SidecarMessage;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

/// SteamVR の稼働状態。overlay-sidecar からの `sidecar.steamvr-status` で更新される。
/// メインUI側の設定画面等が将来参照する可能性を見込んで pub(crate) にしている。
pub(crate) static STEAMVR_RUNNING: AtomicBool = AtomicBool::new(false);

/// hello メッセージの受信を待つ最大時間。これを過ぎたら未認証とみなし切断する。
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// WS サーバーを 127.0.0.1 の空きポートに bind し、受け入れループを spawn する。
/// 戻り値は bind したポート番号（サイドカー起動時の引数に渡す）。
pub(crate) async fn start(token: Arc<str>) -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(accept_loop(listener, token));
    Ok(port)
}

async fn accept_loop(listener: TcpListener, token: Arc<str>) {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                log::warn!("[steamvr] WSブリッジ accept 失敗: {}", e);
                continue;
            }
        };
        // 同一マシン上のサイドカーからの接続のみ受け付ける。
        if !peer.ip().is_loopback() {
            log::warn!("[steamvr] loopback 以外からの接続を拒否: {}", peer);
            continue;
        }
        let token = token.clone();
        tokio::spawn(async move {
            handle_connection(stream, peer, token).await;
        });
    }
}

async fn handle_connection(stream: TcpStream, peer: SocketAddr, token: Arc<str>) {
    let mut ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[steamvr] WSハンドシェイク失敗 ({}): {}", peer, e);
            return;
        }
    };

    let hello_text = match tokio::time::timeout(HELLO_TIMEOUT, ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => {
            log::warn!("[steamvr] hello メッセージの受信に失敗、切断します: {}", peer);
            return;
        }
    };
    let pid = match serde_json::from_str::<SidecarMessage>(&hello_text) {
        Ok(SidecarMessage::Hello { pid, token: t }) if t == token.as_ref() => pid,
        _ => {
            log::warn!("[steamvr] トークン認証に失敗、切断します: {}", peer);
            return;
        }
    };
    log::info!("[steamvr] overlay-sidecar 接続確立: {} (pid={})", peer, pid);

    while let Some(Ok(msg)) = ws_stream.next().await {
        if let Message::Text(text) = msg {
            handle_message(&text);
        }
    }
    log::info!("[steamvr] overlay-sidecar 切断: {}", peer);
}

fn handle_message(text: &str) {
    match serde_json::from_str::<SidecarMessage>(text) {
        Ok(SidecarMessage::SteamvrStatus { running }) => {
            STEAMVR_RUNNING.store(running, Ordering::Relaxed);
            log::info!("[steamvr] SteamVR 稼働状態: {}", running);
        }
        Ok(SidecarMessage::Hello { .. }) => {
            // 認証済み接続で再度 hello が来ても無視する。
        }
        Err(e) => {
            log::warn!("[steamvr] 不正なメッセージを受信: {} ({})", e, text);
        }
    }
}

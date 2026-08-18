//! overlay-sidecar (.NET, 別プロセス) / overlay-ui (サイドカー内 CEF で動く
//! Angular) と通信する WebSocket ブリッジサーバー。
//!
//! `osc::is_trusted_src` と同じ「信頼境界を明示チェックする」流儀を踏襲する:
//! loopback 以外からの接続は即座に拒否し、さらに接続直後の最初のメッセージで
//! トークン認証を行う（トークンはサイドカー起動時の CLI 引数、および
//! overlay-ui へは URL クエリ経由でのみ渡される）。

use crate::steamvr::protocol::{ClientMessage, ServerMessage};
use crate::vrchat::models::VRCAvatar;
use crate::{storage, vrchat};
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

/// SteamVR の稼働状態。overlay-sidecar からの `sidecar.steamvr-status` で更新される。
/// メインUI側の設定画面等が将来参照する可能性を見込んで pub(crate) にしている。
pub(crate) static STEAMVR_RUNNING: AtomicBool = AtomicBool::new(false);

/// `avatar-changed` 等、Rust側から能動的に全WSクライアントへ push するための
/// チャンネル。各接続がこれを subscribe し、自分の書き込み側へ転送する。
static PUSH_TX: Lazy<broadcast::Sender<ServerMessage>> = Lazy::new(|| broadcast::channel(16).0);

/// `osc/mod.rs` の `/avatar/change` 受信ハンドラから呼ばれる。
/// 購読者がいない（overlay-ui未接続）場合の送信失敗は無視してよい。
pub(crate) fn broadcast_avatar_changed(avatar_id: String) {
    let _ = PUSH_TX.send(ServerMessage::AvatarChanged { avatar_id });
}

/// hello メッセージの受信を待つ最大時間。これを過ぎたら未認証とみなし切断する。
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// WS サーバーを 127.0.0.1 の空きポートに bind し、受け入れループを spawn する。
/// 戻り値は bind したポート番号（サイドカー起動時の引数、overlay-ui への
/// URL クエリに渡す）。
pub(crate) async fn start(app: AppHandle, token: Arc<str>) -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(accept_loop(listener, app, token));
    Ok(port)
}

async fn accept_loop(listener: TcpListener, app: AppHandle, token: Arc<str>) {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                log::warn!("[steamvr] WSブリッジ accept 失敗: {}", e);
                continue;
            }
        };
        // 同一マシン上のサイドカー/overlay-uiからの接続のみ受け付ける。
        if !peer.ip().is_loopback() {
            log::warn!("[steamvr] loopback 以外からの接続を拒否: {}", peer);
            continue;
        }
        let token = token.clone();
        let app = app.clone();
        tokio::spawn(async move {
            handle_connection(stream, peer, app, token).await;
        });
    }
}

async fn handle_connection(stream: TcpStream, peer: SocketAddr, app: AppHandle, token: Arc<str>) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[steamvr] WSハンドシェイク失敗 ({}): {}", peer, e);
            return;
        }
    };
    let (mut write, mut read) = ws_stream.split();

    let hello_text = match tokio::time::timeout(HELLO_TIMEOUT, read.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => {
            log::warn!("[steamvr] hello メッセージの受信に失敗、切断します: {}", peer);
            return;
        }
    };
    let pid = match serde_json::from_str::<ClientMessage>(&hello_text) {
        Ok(ClientMessage::Hello { pid, token: t }) if t == token.as_ref() => pid,
        _ => {
            log::warn!("[steamvr] トークン認証に失敗、切断します: {}", peer);
            return;
        }
    };
    log::info!("[steamvr] WSクライアント接続確立: {} (pid={})", peer, pid);

    let mut push_rx = PUSH_TX.subscribe();
    loop {
        tokio::select! {
            incoming = read.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(reply) = handle_message(&app, &text).await {
                            if send(&mut write, &reply).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(_)) => {}
                    _ => break,
                }
            }
            pushed = push_rx.recv() => {
                match pushed {
                    Ok(msg) => {
                        if send(&mut write, &msg).await.is_err() {
                            break;
                        }
                    }
                    // 受信が追いつかず古いイベントが破棄された場合はスキップして継続する
                    // （avatar-changed は最新状態だけ分かれば十分なため取りこぼしを許容）。
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    log::info!("[steamvr] WSクライアント切断: {}", peer);
}

async fn send(
    write: &mut (impl futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    msg: &ServerMessage,
) -> Result<(), ()> {
    let payload = match serde_json::to_string(msg) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("[steamvr] メッセージのシリアライズに失敗: {}", e);
            return Err(());
        }
    };
    write.send(Message::Text(payload)).await.map_err(|_| ())
}

async fn handle_message(app: &AppHandle, text: &str) -> Option<ServerMessage> {
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::SteamvrStatus { running }) => {
            STEAMVR_RUNNING.store(running, Ordering::Relaxed);
            log::info!("[steamvr] SteamVR 稼働状態: {}", running);
            None
        }
        Ok(ClientMessage::Hello { .. }) => {
            // 認証済み接続で再度 hello が来ても無視する。
            None
        }
        Ok(ClientMessage::AvatarsList) => {
            let reply = handle_avatars_list(app).await;
            if let ServerMessage::AvatarsListResult { avatars } = &reply {
                log::info!("[steamvr] avatars.list 応答: {}件", avatars.len());
            }
            Some(reply)
        }
        Ok(ClientMessage::AvatarsSelect { avatar_id }) => {
            Some(handle_avatars_select(&avatar_id).await)
        }
        Ok(ClientMessage::FoldersList) => Some(handle_folders_list(app).await),
        Err(e) => {
            log::warn!("[steamvr] 不正なメッセージを受信: {} ({})", e, text);
            Some(ServerMessage::Error {
                message: format!("invalid message: {e}"),
            })
        }
    }
}

/// キャッシュ済みアバター(自前+お気に入り、重複除去) にローカルオーバーライドを
/// 適用して返す。`avatar.service.ts::allAvatarsWithOverrides` と同じマージ規則。
/// overlay-ui はディスクキャッシュを持たないため、ネットワーク待ちなしで
/// 即座に返せるキャッシュ済みデータのみを使う（本体側の fresh フェッチが
/// 完了していればそちらが自動的に反映される）。
async fn handle_avatars_list(app: &AppHandle) -> ServerMessage {
    let mut avatars: Vec<VRCAvatar> = vrchat::cache::load_avatars();
    let mut seen: HashSet<String> = avatars.iter().map(|a| a.id.clone()).collect();
    for fav in vrchat::cache::load_favorites() {
        if seen.insert(fav.id.clone()) {
            avatars.push(fav);
        }
    }

    let overrides = storage::commands::avatar_overrides_get_all(app.clone())
        .await
        .unwrap_or_default();
    let override_map: HashMap<String, _> = overrides
        .into_iter()
        .map(|o| (o.avatar_id.clone(), o))
        .collect();

    for avatar in &mut avatars {
        if let Some(o) = override_map.get(&avatar.id) {
            if let Some(name) = &o.custom_name {
                avatar.name = name.clone();
            }
            if let Some(thumb) = &o.custom_thumbnail {
                avatar.thumbnail_image_url = thumb.clone();
            }
        }
    }

    ServerMessage::AvatarsListResult { avatars }
}

/// フォルダ一覧を返す（#26）。表示・切り替えのみが目的のため
/// `storage::commands::folders_get_all` をそのまま呼ぶだけで十分（作成/編集/
/// 削除系のコマンドは公開しない）。
async fn handle_folders_list(app: &AppHandle) -> ServerMessage {
    let folders = storage::commands::folders_get_all(app.clone())
        .await
        .unwrap_or_default();
    ServerMessage::FoldersListResult { folders }
}

/// REST (装着) と OSC (即時反映) を送信する。`avatar.service.ts::switchAvatar` と同じ意図だが、
/// OSC送信はローカルUDPソケットへの同期呼び出しで実質一瞬のため、
/// 素直に先に送ってから非同期のREST呼び出しを待てば十分（tokio::joinで
/// 並行化する実利は無い）。
async fn handle_avatars_select(avatar_id: &str) -> ServerMessage {
    log::info!("[steamvr] avatars.select 受信: avatar_id={}", avatar_id);
    if let Err(e) = crate::osc::send_avatar_change(avatar_id) {
        log::warn!("[steamvr] OSC送信失敗: {}", e);
    }
    match vrchat::avatars::select_avatar(avatar_id).await {
        Ok(()) => {
            log::info!("[steamvr] avatars.select 成功: avatar_id={}", avatar_id);
            ServerMessage::AvatarsSelectResult {
                avatar_id: avatar_id.to_string(),
            }
        }
        Err(e) => {
            log::warn!("[steamvr] avatars.select 失敗: {}", e);
            ServerMessage::Error {
                message: e.to_string(),
            }
        }
    }
}

//! SteamVR ダッシュボードオーバーレイ連携（Phase 1 MVP）。
//!
//! Rust 本体は overlay-sidecar (.NET, 別プロセス) の起動・監視と、
//! WebSocket ブリッジサーバーの提供のみを担当する。OpenVR 呼び出し・
//! CefSharp によるオフスクリーン描画・コントローラー入力の翻訳は
//! すべてサイドカー側 (`overlay-sidecar/`) に閉じている。

pub mod bridge;
pub mod protocol;
pub mod sidecar;

use tauri::AppHandle;

/// SteamVR オーバーレイ機能を初期化する。WS ブリッジを起動し、overlay-sidecar を spawn する。
///
/// SteamVR 未起動・.NET ランタイム未導入・サイドカー未ビルドのいずれの場合でも
/// 本体アプリの起動を妨げないよう、失敗はすべて warn/error ログに留めて
/// 呼び出し元 (`lib.rs::setup`) には伝播させない。
pub(crate) async fn init(app: &AppHandle) {
    let token = uuid::Uuid::new_v4().to_string();
    match bridge::start(app.clone(), std::sync::Arc::from(token.as_str())).await {
        Ok(port) => {
            log::info!("[steamvr] WSブリッジ起動 (port={})", port);
            sidecar::spawn(app.clone(), port, token).await;
        }
        Err(e) => {
            log::error!("[steamvr] WSブリッジの起動に失敗しました: {}", e);
        }
    }
}

/// アプリ終了時に呼ぶ。サイドカープロセスを確実に終了させる。
pub(crate) fn shutdown() {
    sidecar::shutdown();
}

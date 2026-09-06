//! SteamVR ダッシュボードオーバーレイ連携（Phase 1 MVP）。
//!
//! Rust 本体は overlay-sidecar (.NET, 別プロセス) の起動・監視と、
//! WebSocket ブリッジサーバー・overlay-ui 用の静的ファイルサーバーの
//! 提供のみを担当する。OpenVR 呼び出し・CefSharp によるオフスクリーン
//! 描画・コントローラー入力の翻訳はすべてサイドカー側 (`overlay-sidecar/`)
//! に閉じている。

pub mod bridge;
pub mod protocol;
pub mod sidecar;
pub mod static_server;

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// SteamVR オーバーレイ機能を初期化する。WS ブリッジ・overlay-ui 静的配信を起動し、
/// overlay-sidecar を spawn する。
///
/// SteamVR 未起動・.NET ランタイム未導入・サイドカー未ビルド・overlay-ui未ビルド
/// のいずれの場合でも本体アプリの起動を妨げないよう、失敗はすべて warn/error
/// ログに留めて呼び出し元 (`lib.rs::setup`) には伝播させない。
pub(crate) async fn init(app: &AppHandle) {
    let token = uuid::Uuid::new_v4().to_string();
    // #28: パネル配置方式はサイドカー起動時のCLI引数でのみ渡せる（WS経由のホット
    // リロードはv1スコープ外）。ここで一度だけ読み込む。
    let placement_mode = crate::storage::commands::overlay_settings_get(app.clone())
        .await
        .map(|s| s.placement_mode)
        .unwrap_or_default();
    // 身長変更の上限適用設定（#55）。以降の変更は
    // `storage::commands::eye_height_settings_set` からの broadcast で追随する。
    let eye_height_limit_enabled = crate::storage::commands::eye_height_settings_get(app.clone())
        .await
        .map(|s| s.limit_enabled)
        .unwrap_or(true);
    crate::osc::EYE_HEIGHT_LIMIT_ENABLED.store(eye_height_limit_enabled, std::sync::atomic::Ordering::Relaxed);
    match bridge::start(app.clone(), std::sync::Arc::from(token.as_str())).await {
        Ok(port) => {
            log::info!("[steamvr] WSブリッジ起動 (port={})", port);
            let ui_port = start_overlay_ui_static_server(app).await;
            sidecar::spawn(app.clone(), port, ui_port, token, placement_mode).await;
        }
        Err(e) => {
            log::error!("[steamvr] WSブリッジの起動に失敗しました: {}", e);
        }
    }
}

/// overlay-ui (Angular) のビルド成果物が見つかれば静的ファイルサーバーを起動する。
/// 見つからない場合は None を返し、サイドカー側はM1テストページ等へフォールバックする。
///
/// 起動時に一度だけ解決する（サイドカーのexe解決と違い、サーバー自体の起動は
/// 一度成功すればよく、後から overlay-ui をビルドし直した場合はアプリ再起動が
/// 必要という制約を受け入れる）。
async fn start_overlay_ui_static_server(app: &AppHandle) -> Option<u16> {
    let dir = resolve_overlay_ui_dist_dir(app)?;
    match static_server::start(dir.clone()).await {
        Ok(port) => {
            log::info!(
                "[steamvr] overlay-ui 静的ファイルサーバー起動 (port={}, dir={})",
                port,
                dir.display()
            );
            Some(port)
        }
        Err(e) => {
            log::warn!("[steamvr] overlay-ui 静的ファイルサーバーの起動に失敗: {}", e);
            None
        }
    }
}

/// overlay-ui の `dist/overlay-ui/browser` 相当のディレクトリを解決する。
/// 1. 環境変数 `AVATAR_SWITCHER_OVERLAY_UI_DIST_PATH`（開発用オーバーライド。
///    `sidecar::resolve_sidecar_path` と同じ流儀で開発用オーバーライドを最優先する）
/// 2. Tauri のリソースディレクトリ配下（本番、tauri.conf.json の bundle.resources 経由で配置）
fn resolve_overlay_ui_dist_dir(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AVATAR_SWITCHER_OVERLAY_UI_DIST_PATH") {
        let path = PathBuf::from(p);
        if path.join("index.html").exists() {
            return Some(path);
        }
        log::warn!(
            "[steamvr] AVATAR_SWITCHER_OVERLAY_UI_DIST_PATH が指す先に index.html がありません: {}",
            path.display()
        );
    }
    let resource_path = app.path().resource_dir().ok()?.join("overlay-ui");
    resource_path
        .join("index.html")
        .exists()
        .then_some(resource_path)
}

/// アプリ終了時に呼ぶ。サイドカープロセスを確実に終了させる。
pub(crate) fn shutdown() {
    sidecar::shutdown();
}

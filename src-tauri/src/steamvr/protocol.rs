//! Rust core ↔ overlay-sidecar (.NET) 間の WebSocket メッセージスキーマ。
//!
//! Phase 1 M0 時点ではサイドカーのライフサイクル通知のみを扱う。
//! アバター一覧取得/切替メッセージ (`avatars.list` / `avatars.select`) は
//! overlay-ui 側の実装と合わせて M2 で追加する。

use serde::Deserialize;

/// overlay-sidecar → Rust core。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SidecarMessage {
    /// 接続確立直後に送られる認証メッセージ。`token` が WS サーバー起動時に
    /// 発行したものと一致しない限り、以降のメッセージは受け付けない。
    #[serde(rename = "sidecar.hello")]
    Hello { pid: u32, token: String },
    /// OpenVR `VR_Init` の成否。SteamVR の起動/終了のたびに送られる。
    #[serde(rename = "sidecar.steamvr-status")]
    SteamvrStatus { running: bool },
}

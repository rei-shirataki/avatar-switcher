//! Rust core ↔ WS クライアント（overlay-sidecar 本体 / overlay-sidecar 内 CEF で
//! 動く overlay-ui の両方）間のメッセージスキーマ。
//!
//! WS サーバーは同一ポートに複数接続を受け付ける。overlay-sidecar は
//! ライフサイクル通知（`sidecar.hello` / `sidecar.steamvr-status`）のみ、
//! overlay-ui はアバター一覧取得/切替（`avatars.list` / `avatars.select`）を
//! 使う。どちらも接続直後に `sidecar.hello` でトークン認証する点は共通。

use crate::vrchat::models::VRCAvatar;
use serde::{Deserialize, Serialize};

/// WS クライアント → Rust core。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// 接続確立直後に送られる認証メッセージ。`token` が WS サーバー起動時に
    /// 発行したものと一致しない限り、以降のメッセージは受け付けない。
    /// `pid` は overlay-sidecar プロセスの場合のみ意味を持ち、overlay-ui
    /// (ブラウザ内JS) からは 0 固定で構わない。
    #[serde(rename = "sidecar.hello")]
    Hello { pid: u32, token: String },
    /// OpenVR `VR_Init` の成否。SteamVR の起動/終了のたびに送られる。
    #[serde(rename = "sidecar.steamvr-status")]
    SteamvrStatus { running: bool },
    /// アバター一覧（自前+お気に入り、ローカルオーバーライド適用済み）を要求する。
    #[serde(rename = "avatars.list")]
    AvatarsList,
    /// アバターを装着する（REST + OSC を並行送信）。
    #[serde(rename = "avatars.select")]
    AvatarsSelect { avatar_id: String },
}

/// Rust core → WS クライアント。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "avatars.list-result")]
    AvatarsListResult { avatars: Vec<VRCAvatar> },
    #[serde(rename = "avatars.select-result")]
    AvatarsSelectResult { avatar: VRCAvatar },
    /// VRChat 側でアバターが切り替わったこと（自分の操作/他端末経由問わず）を
    /// 接続中の全クライアントへ push する。`osc/mod.rs` の `/avatar/change`
    /// 受信ハンドラから broadcast される。
    #[serde(rename = "avatar-changed")]
    AvatarChanged { avatar_id: String },
    #[serde(rename = "error")]
    Error { message: String },
}

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
///
/// `rename_all = "camelCase"` は必須。overlay-ui (TypeScript) 側は
/// `avatarId` のようなJS慣習のcamelCaseでJSONを送ってくるため、これが無いと
/// Rustのフィールド名(`avatar_id`)と一致せず "missing field `avatar_id`" で
/// デシリアライズに失敗する（実機確認で発覚：クリックはCEFまで届き
/// WS送信もされていたが、Rust側で不正なメッセージとして黙って弾かれていた）。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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

/// Rust core → WS クライアント。overlay-ui (TypeScript) 側が `avatarId` の
/// ようなcamelCaseで受け取る前提のため ClientMessage と同様 rename_all が必須。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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

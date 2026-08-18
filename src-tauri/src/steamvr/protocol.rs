//! Rust core ↔ WS クライアント（overlay-sidecar 本体 / overlay-sidecar 内 CEF で
//! 動く overlay-ui の両方）間のメッセージスキーマ。
//!
//! WS サーバーは同一ポートに複数接続を受け付ける。overlay-sidecar は
//! ライフサイクル通知（`sidecar.hello` / `sidecar.steamvr-status`）のみ、
//! overlay-ui はアバター一覧取得/切替（`avatars.list` / `avatars.select`）を
//! 使う。どちらも接続直後に `sidecar.hello` でトークン認証する点は共通。

use crate::storage::AvatarFolder;
use crate::vrchat::models::VRCAvatar;
use serde::{Deserialize, Serialize};

/// WS クライアント → Rust core。
///
/// 各バリアントに `rename_all = "camelCase"` が必須。overlay-ui (TypeScript)
/// 側は `avatarId` のようなJS慣習のcamelCaseでJSONを送ってくるため、これが
/// 無いとRustのフィールド名(`avatar_id`)と一致せず "missing field `avatar_id`"
/// でデシリアライズに失敗する（実機確認で発覚：クリックはCEFまで届き
/// WS送信もされていたが、Rust側で不正なメッセージとして黙って弾かれていた）。
/// **注意**: `rename_all` を enum 直下（`#[serde(tag = "type", rename_all = ...)]`）
/// に置いても効かない。内部タグ付きenumではフィールド名の大文字小文字変換は
/// 各バリアント個別の属性としてしか効かない（serdeの実際の挙動、一度
/// enumレベルで試して直らずハマった。examples/serde_probe.rs で単体検証して確定）。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// 接続確立直後に送られる認証メッセージ。`token` が WS サーバー起動時に
    /// 発行したものと一致しない限り、以降のメッセージは受け付けない。
    /// `pid` は overlay-sidecar プロセスの場合のみ意味を持ち、overlay-ui
    /// (ブラウザ内JS) からは 0 固定で構わない。
    #[serde(rename = "sidecar.hello", rename_all = "camelCase")]
    Hello { pid: u32, token: String },
    /// OpenVR `VR_Init` の成否。SteamVR の起動/終了のたびに送られる。
    #[serde(rename = "sidecar.steamvr-status", rename_all = "camelCase")]
    SteamvrStatus { running: bool },
    /// アバター一覧（自前+お気に入り、ローカルオーバーライド適用済み）を要求する。
    #[serde(rename = "avatars.list")]
    AvatarsList,
    /// フォルダ一覧を要求する（#26、フォルダタブ切り替え用）。
    /// overlay-uiは表示・切り替えのみ行い、作成/編集/削除は行わない。
    #[serde(rename = "folders.list")]
    FoldersList,
    /// アバターを装着する（REST + OSC を並行送信）。
    #[serde(rename = "avatars.select", rename_all = "camelCase")]
    AvatarsSelect { avatar_id: String },
    /// アイハイト(EyeHeight)をOSC経由で設定する（#27）。v1スコープは
    /// 現在値表示+ステップボタンのみで、リセット用のプレハブ身長逆算・
    /// EyeHeight/ScaleFactor/ScaleModifiedの異時点ペアリングraceは対象外
    /// （`EyeHeightService`のそれはリセット機能専用のロジックのため）。
    /// fire-and-forgetで送信結果は返さず、実際に反映された値は
    /// `EyeHeightUpdate` broadcastで受け取る。
    #[serde(rename = "eyeheight.set", rename_all = "camelCase")]
    EyeHeightSet { value: f32 },
    /// 接続直後の初回同期用（#27フォローアップ）。受動OSCイベントは
    /// アバターロード時の一瞬のダンプを取りこぼすと、身長変更が一度も
    /// 起きない限り二度と届かず表示が「—」のまま固まってしまう。
    /// VRChatのOSCQuery HTTPサーバーへ現在値を能動的に問い合わせる
    /// （`oscquery::query_avatar_scale_snapshot`のeye_heightのみ使用、
    /// ScaleFactor/ScaleModifiedのペアリングraceはリセット機能専用の
    /// ロジックのため引き続き不使用）。
    #[serde(rename = "eyeheight.query")]
    EyeHeightQuery,
}

/// Rust core → WS クライアント。overlay-ui (TypeScript) 側が `avatarId` の
/// ようなcamelCaseで受け取る前提のため ClientMessage と同様、各バリアントに
/// `rename_all` が必要（enum直下では効かない。ClientMessage側のコメント参照）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "avatars.list-result", rename_all = "camelCase")]
    AvatarsListResult { avatars: Vec<VRCAvatar> },
    #[serde(rename = "folders.list-result", rename_all = "camelCase")]
    FoldersListResult { folders: Vec<AvatarFolder> },
    /// VRChatの装着APIは「更新後のアバター情報」ではなくユーザープロフィールを
    /// 返す実装のため（`vrchat::avatars::select_avatar`のコメント参照）、
    /// avatar情報は積めない。クライアント側は要求時に渡したavatar_idを
    /// 既に知っているので、成功可否の通知のみで十分。
    #[serde(rename = "avatars.select-result", rename_all = "camelCase")]
    AvatarsSelectResult { avatar_id: String },
    /// VRChat 側でアバターが切り替わったこと（自分の操作/他端末経由問わず）を
    /// 接続中の全クライアントへ push する。`osc/mod.rs` の `/avatar/change`
    /// 受信ハンドラから broadcast される。
    #[serde(rename = "avatar-changed", rename_all = "camelCase")]
    AvatarChanged { avatar_id: String },
    /// VRChat側のEyeHeightAsMeters/eyeheight OSC受信を全クライアントへpushする
    /// （#27、`AvatarChanged`と同じ仕組み）。
    #[serde(rename = "eyeheight-update", rename_all = "camelCase")]
    EyeHeightUpdate { value: f32 },
    #[serde(rename = "error", rename_all = "camelCase")]
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::AvatarFolder;

    #[test]
    fn folders_list_roundtrip() {
        let client_json = r#"{"type":"folders.list"}"#;
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(client_json),
            Ok(ClientMessage::FoldersList)
        ));

        let server_msg = ServerMessage::FoldersListResult {
            folders: vec![AvatarFolder {
                id: "f1".into(),
                name: "テスト".into(),
                avatar_ids: vec!["a1".into(), "a2".into()],
                color: None,
                order: 0,
            }],
        };
        let json = serde_json::to_string(&server_msg).unwrap();
        assert!(json.contains("\"avatarIds\""), "avatarIdsがcamelCaseでない: {json}");
        assert!(json.contains("\"folders.list-result\""), "typeタグが不正: {json}");
    }

    #[test]
    fn eye_height_roundtrip() {
        let client_json = r#"{"type":"eyeheight.set","value":1.5}"#;
        match serde_json::from_str::<ClientMessage>(client_json) {
            Ok(ClientMessage::EyeHeightSet { value }) => assert_eq!(value, 1.5),
            other => panic!("デシリアライズ失敗: {other:?}"),
        }

        let json = serde_json::to_string(&ServerMessage::EyeHeightUpdate { value: 1.5 }).unwrap();
        assert!(json.contains("\"eyeheight-update\""), "typeタグが不正: {json}");
        assert!(json.contains("\"value\":1.5"), "valueフィールドが不正: {json}");

        let query_json = r#"{"type":"eyeheight.query"}"#;
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(query_json),
            Ok(ClientMessage::EyeHeightQuery)
        ));
    }
}

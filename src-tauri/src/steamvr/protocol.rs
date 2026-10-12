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
    /// アイハイトをアバター本来のプレハブ身長にリセットする（新規、リクエスト機能）。
    /// `oscquery::compute_prefab_height`で計算し(#27の`EyeHeightQuery`と同じ
    /// `query_avatar_scale_snapshot`を再利用、3値を`tokio::join!`で同時取得する
    /// ためデスクトップ側のペアリングrace対策は不要)、計算不能なら
    /// `oscquery::EYE_HEIGHT_DEFAULT`にフォールバックする。fire-and-forgetで、
    /// 結果は`EyeHeightUpdate` broadcastで受け取る（`EyeHeightSet`と同じ設計）。
    #[serde(rename = "eyeheight.reset")]
    EyeHeightReset,
    /// 接続直後の初回同期用。身長変更の上限適用オン/オフ設定
    /// （デスクトップUIの設定画面でのみ変更可能、overlay-ui側は表示のみ）を取得する。
    /// 応答・変更時のpushはどちらも `ServerMessage::EyeHeightSettingsUpdate` で届く
    /// （`ui-state.get`とは異なり、変更元がoverlay-ui自身ではなくデスクトップUI側の
    /// ため、get専用の応答型を分けず設定変更時のbroadcastと同じ型に統一している）。
    #[serde(rename = "eyeheight-settings.get")]
    EyeHeightSettingsGet,
    /// overlay-uiのUI状態（ソートモード・選択中タブ）を取得する。overlay-sidecarは
    /// CEFプロセスごとに新しいキャッシュディレクトリを使う（`Program.cs::InitCef`の
    /// コメント参照）ため、ブラウザのlocalStorageは再起動をまたいで永続化できない。
    /// Rust側の`storage::OverlayUiState`に保存された値を接続直後に取得する。
    #[serde(rename = "ui-state.get")]
    UiStateGet,
    /// ソート/タブ切り替えのたびに送られる。fire-and-forgetで応答不要
    /// （`eyeheight.set`と同じ、単純な上書き保存のため成否を待つ必要がない）。
    #[serde(rename = "ui-state.set", rename_all = "camelCase")]
    UiStateSet {
        sort_mode: String,
        selected_folder_id: Option<String>,
    },
}

/// Rust core → WS クライアント。overlay-ui (TypeScript) 側が `avatarId` の
/// ようなcamelCaseで受け取る前提のため ClientMessage と同様、各バリアントに
/// `rename_all` が必要（enum直下では効かない。ClientMessage側のコメント参照）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// `favoriteIds`/`uploadedIds` はoverlay-ui側のお気に入り/アップロード済みタブ
    /// フィルタ用（#26のフォルダタブと同じ仕組みで判定できるよう、avatarsは
    /// 自前+お気に入りを重複除去したマージ済み一覧、idセットは所属元の判定に使う）。
    #[serde(rename = "avatars.list-result", rename_all = "camelCase")]
    AvatarsListResult {
        avatars: Vec<VRCAvatar>,
        favorite_ids: Vec<String>,
        uploaded_ids: Vec<String>,
    },
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
    /// VRChat側のEyeHeightAsMeters/eyeheight OSC受信、`EyeHeightQuery`/`EyeHeightReset`
    /// の応答を全クライアントへpushする（#27、`AvatarChanged`と同じ仕組み）。
    /// `is_reset`は`EyeHeightReset`自身の応答である場合のみ`true`。3つの発生源
    /// （受動OSCエコー・`EyeHeightQuery`応答・`EyeHeightReset`応答）が同じ
    /// メッセージ型を共有しており、値だけでは見分けが付かないため、受信側
    /// (overlay-eye-height.service.ts)がリセット応答だけを確実に識別できるように
    /// 明示的なフラグとして持たせている。
    #[serde(rename = "eyeheight-update", rename_all = "camelCase")]
    EyeHeightUpdate { value: f32, is_reset: bool },
    /// `EyeHeightSettingsGet` の応答、およびデスクトップUIでの設定変更時に
    /// 接続中の全クライアントへ push される（#55）。
    #[serde(rename = "eyeheight-settings.update", rename_all = "camelCase")]
    EyeHeightSettingsUpdate { limit_enabled: bool },
    #[serde(rename = "ui-state.get-result", rename_all = "camelCase")]
    UiStateGetResult {
        sort_mode: String,
        selected_folder_id: Option<String>,
    },
    #[serde(rename = "error", rename_all = "camelCase")]
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::AvatarFolder;

    #[test]
    fn avatars_list_result_roundtrip() {
        let server_msg = ServerMessage::AvatarsListResult {
            avatars: vec![],
            favorite_ids: vec!["a1".into()],
            uploaded_ids: vec!["a2".into()],
        };
        let json = serde_json::to_string(&server_msg).unwrap();
        assert!(json.contains("\"favoriteIds\":[\"a1\"]"), "favoriteIdsがcamelCaseでない: {json}");
        assert!(json.contains("\"uploadedIds\":[\"a2\"]"), "uploadedIdsがcamelCaseでない: {json}");
        assert!(json.contains("\"avatars.list-result\""), "typeタグが不正: {json}");
    }

    #[test]
    fn ui_state_roundtrip() {
        let get_json = r#"{"type":"ui-state.get"}"#;
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(get_json),
            Ok(ClientMessage::UiStateGet)
        ));

        let set_json = r#"{"type":"ui-state.set","sortMode":"name-asc","selectedFolderId":"__favorites__"}"#;
        match serde_json::from_str::<ClientMessage>(set_json) {
            Ok(ClientMessage::UiStateSet { sort_mode, selected_folder_id }) => {
                assert_eq!(sort_mode, "name-asc");
                assert_eq!(selected_folder_id.as_deref(), Some("__favorites__"));
            }
            other => panic!("デシリアライズ失敗: {other:?}"),
        }

        let json = serde_json::to_string(&ServerMessage::UiStateGetResult {
            sort_mode: "updated-desc".into(),
            selected_folder_id: None,
        })
        .unwrap();
        assert!(json.contains("\"ui-state.get-result\""), "typeタグが不正: {json}");
        assert!(json.contains("\"sortMode\":\"updated-desc\""), "sortModeがcamelCaseでない: {json}");
        assert!(json.contains("\"selectedFolderId\":null"), "selectedFolderIdの扱いが不正: {json}");
    }

    #[test]
    fn eye_height_reset_roundtrip() {
        let json = r#"{"type":"eyeheight.reset"}"#;
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(json),
            Ok(ClientMessage::EyeHeightReset)
        ));
    }

    #[test]
    fn eye_height_settings_roundtrip() {
        let get_json = r#"{"type":"eyeheight-settings.get"}"#;
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(get_json),
            Ok(ClientMessage::EyeHeightSettingsGet)
        ));

        let json = serde_json::to_string(&ServerMessage::EyeHeightSettingsUpdate { limit_enabled: false })
            .unwrap();
        assert!(json.contains("\"eyeheight-settings.update\""), "typeタグが不正: {json}");
        assert!(json.contains("\"limitEnabled\":false"), "limitEnabledがcamelCaseでない: {json}");
    }

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

        let json = serde_json::to_string(&ServerMessage::EyeHeightUpdate { value: 1.5, is_reset: true }).unwrap();
        assert!(json.contains("\"eyeheight-update\""), "typeタグが不正: {json}");
        assert!(json.contains("\"value\":1.5"), "valueフィールドが不正: {json}");
        assert!(json.contains("\"isReset\":true"), "isResetフィールドが不正: {json}");

        let query_json = r#"{"type":"eyeheight.query"}"#;
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(query_json),
            Ok(ClientMessage::EyeHeightQuery)
        ));
    }
}

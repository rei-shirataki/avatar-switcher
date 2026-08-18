pub mod commands;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarFolder {
    pub id: String,
    pub name: String,
    // Serialized as "avatarIds" so the Tauri response matches the TypeScript interface.
    // "avatar_ids" alias preserves compatibility with previously stored JSON.
    #[serde(rename = "avatarIds", alias = "avatar_ids", default)]
    pub avatar_ids: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub order: u32,
}

/// オーバーレイパネルの配置方式（#28）。CLI引数として overlay-sidecar に渡す文字列と
/// 1:1対応させている（`as_cli_arg`）。設定変更はサイドカー再起動後に反映される
/// （WS経由のホットリロードはv1スコープ外）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayPlacementMode {
    /// ダブルプレスした手のコントローラーに常時追従させる（既定）。
    Hand,
    /// 表示ONの瞬間に手元付近へ出現させ、以降は空間に静止させる（OyasumiVR方式）。
    Space,
}

impl Default for OverlayPlacementMode {
    fn default() -> Self {
        OverlayPlacementMode::Hand
    }
}

impl OverlayPlacementMode {
    pub fn as_cli_arg(self) -> &'static str {
        match self {
            OverlayPlacementMode::Hand => "hand",
            OverlayPlacementMode::Space => "space",
        }
    }
}

/// SteamVRオーバーレイ関連のユーザー設定（#28）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySettings {
    #[serde(default)]
    pub placement_mode: OverlayPlacementMode,
}

fn default_ui_state_sort_mode() -> String {
    "updated-desc".to_string()
}

/// overlay-uiのUI状態（ソートモード・選択中タブ）。overlay-sidecarはCEFプロセスごとに
/// 新しいキャッシュディレクトリを使う（`Program.cs::InitCef`、複数プロセス起動時の
/// Chromiumシングルトン衝突回避のため）ため、ブラウザのlocalStorageは再起動を
/// またいで永続化できない。Rust側の`app-settings.json`に保存する。
/// `selected_folder_id`は「すべて」タブ選択時はNone、フォルダタブ/お気に入り(`__favorites__`)/
/// アップロード済み(`__uploaded__`)選択時はそのID文字列（overlay-ui側の名前空間をそのまま保存）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayUiState {
    #[serde(default = "default_ui_state_sort_mode")]
    pub sort_mode: String,
    #[serde(default)]
    pub selected_folder_id: Option<String>,
}

impl Default for OverlayUiState {
    fn default() -> Self {
        Self {
            sort_mode: default_ui_state_sort_mode(),
            selected_folder_id: None,
        }
    }
}

/// ローカルで管理するアバター表示オーバーライド（名前・サムネイル）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AvatarOverride {
    #[serde(rename = "avatarId")]
    pub avatar_id: String,
    /// カスタム表示名。None の場合は VRChat から取得した名前を使用
    #[serde(rename = "customName", default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    /// カスタムサムネイル（base64 data URL）。None の場合は VRChat の URL を使用
    #[serde(rename = "customThumbnail", default, skip_serializing_if = "Option::is_none")]
    pub custom_thumbnail: Option<String>,
}

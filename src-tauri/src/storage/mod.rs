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

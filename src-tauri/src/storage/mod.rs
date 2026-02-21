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

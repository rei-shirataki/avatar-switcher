use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VRCUser {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub current_avatar_image_url: String,
    #[serde(default)]
    pub user_icon: String,
    #[serde(default)]
    pub profile_pic_override: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VRCAvatar {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author_name: String,
    #[serde(default)]
    pub author_id: String,
    #[serde(default)]
    pub thumbnail_image_url: String,
    #[serde(default)]
    pub image_url: String,
    #[serde(default)]
    pub release_status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub version: u32,
    #[serde(default, alias = "created_at")]
    pub created_at: String,
    #[serde(default, alias = "updated_at")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub success: bool,
    pub requires_2fa: bool,
    pub method: Option<String>,
    pub user: Option<VRCUser>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserResponse {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub requires_two_factor_auth: Option<Vec<String>>,
    pub current_avatar_image_url: Option<String>,
    pub user_icon: Option<String>,
    pub profile_pic_override: Option<String>,
    pub status: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TwoFactorVerifyResponse {
    pub verified: bool,
}


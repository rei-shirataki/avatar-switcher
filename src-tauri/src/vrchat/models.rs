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

/// VRChat `/auth/user` レスポンス。
/// `accepted_privacy_version` 等は将来の同意状況判定で参照する予定だが、現状未使用。
/// レスポンス全体の互換性を保つため struct ごと dead_code を許可する。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AuthUserResponse {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub requires_two_factor_auth: Option<Vec<String>>,
    pub current_avatar_image_url: Option<String>,
    pub user_icon: Option<String>,
    pub profile_pic_override: Option<String>,
    pub status: Option<String>,
    pub state: Option<String>,
    /// 利用規約の最新バージョンに同意していない場合、API が `accountDeletionDate` などと
    /// 合わせて返すフラグ。同意が無いと一部 API が 403 を返すので、ログイン時に検知する。
    #[serde(default)]
    pub accepted_tos_version: Option<u32>,
    #[serde(default)]
    pub accepted_privacy_version: Option<u32>,
    #[serde(default)]
    pub email_verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TwoFactorVerifyResponse {
    pub verified: bool,
}


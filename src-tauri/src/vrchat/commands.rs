use crate::vrchat::{auth, avatars};
use crate::vrchat::models::*;

#[tauri::command]
pub async fn vrchat_login(username: String, password: String) -> Result<LoginResult, String> {
    auth::login(&username, &password).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_verify_2fa(code: String, method: String) -> Result<bool, String> {
    auth::verify_2fa(&code, &method).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_get_current_user() -> Result<Option<VRCUser>, String> {
    auth::get_current_user().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_logout() -> Result<(), String> {
    auth::logout().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_get_my_avatars(offset: Option<u32>) -> Result<Vec<VRCAvatar>, String> {
    avatars::get_my_avatars(offset.unwrap_or(0)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_get_favorite_avatars() -> Result<Vec<VRCAvatar>, String> {
    avatars::get_favorite_avatars().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_select_avatar(avatar_id: String) -> Result<VRCAvatar, String> {
    avatars::select_avatar(&avatar_id).await.map_err(|e| e.to_string())
}

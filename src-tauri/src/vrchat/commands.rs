use crate::vrchat::{auth, avatars, cache, files};
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
    // 先に auth.logout を完了させてからキャッシュを破棄する。
    // 逆順だとログアウト失敗時にキャッシュだけ消え、再ログイン後の初回起動で
    // SWR の即時表示が効かなくなる（UX 退行）。
    auth::logout().await.map_err(|e| e.to_string())?;
    cache::clear();
    Ok(())
}

/// 自前アバターを全件取得する。バックエンドで投機的並列フェッチするため、
/// フロントは 1 回呼べばよい（旧 offset 引数は廃止）。
#[tauri::command]
pub async fn vrchat_get_my_avatars() -> Result<Vec<VRCAvatar>, String> {
    avatars::get_my_avatars_all().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_get_favorite_avatars() -> Result<Vec<VRCAvatar>, String> {
    avatars::get_favorite_avatars().await.map_err(|e| e.to_string())
}

/// 前回取得した自前アバターをディスクキャッシュから即時返す（SWR の stale 部分）。
/// キャッシュが無い／壊れている場合は空配列を返す。
#[tauri::command]
pub async fn vrchat_get_cached_avatars() -> Result<Vec<VRCAvatar>, String> {
    Ok(cache::load_avatars())
}

/// 前回取得したお気に入りアバターをディスクキャッシュから即時返す（SWR の stale 部分）。
#[tauri::command]
pub async fn vrchat_get_cached_favorites() -> Result<Vec<VRCAvatar>, String> {
    Ok(cache::load_favorites())
}

#[tauri::command]
pub async fn vrchat_select_avatar(avatar_id: String) -> Result<VRCAvatar, String> {
    avatars::select_avatar(&avatar_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_update_avatar(avatar_id: String, name: String) -> Result<VRCAvatar, String> {
    avatars::update_avatar(&avatar_id, &name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vrchat_update_avatar_image(
    avatar_id: String,
    data_url: String,
) -> Result<VRCAvatar, String> {
    let image_url = files::upload_image(&data_url)
        .await
        .map_err(|e| e.to_string())?;
    avatars::update_avatar_image(&avatar_id, &image_url)
        .await
        .map_err(|e| e.to_string())
}

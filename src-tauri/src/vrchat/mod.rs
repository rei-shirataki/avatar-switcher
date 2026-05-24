pub mod auth;
pub mod avatars;
pub mod cache;
pub mod commands;
pub mod files;
pub mod models;

pub async fn init(app: &tauri::AppHandle) {
    use tauri::Manager;
    // cache::init は lib.rs setup の同期セクションで先行実行済み
    // （フロントが早期に vrchat_get_cached_* を呼ぶ race を防ぐため）。
    let app_data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&app_data_dir).ok();
    auth::init(app_data_dir).await;
}

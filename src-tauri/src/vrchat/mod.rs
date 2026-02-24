pub mod auth;
pub mod avatars;
pub mod commands;
pub mod files;
pub mod models;

pub async fn init(app: &tauri::AppHandle) {
    use tauri::Manager;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&app_data_dir).ok();
    auth::init(app_data_dir).await;
}

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use crate::storage::AvatarFolder;

const STORE_PATH: &str = "app-settings.json";
const FOLDERS_KEY: &str = "avatar_folders";

fn load_folders(app: &AppHandle) -> Vec<AvatarFolder> {
    let store = app.store(STORE_PATH).unwrap();
    store
        .get(FOLDERS_KEY)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

fn save_folders(app: &AppHandle, folders: &[AvatarFolder]) {
    if let Ok(store) = app.store(STORE_PATH) {
        store.set(FOLDERS_KEY, serde_json::to_value(folders).unwrap());
        store.save().ok();
    }
}

#[tauri::command]
pub async fn folders_get_all(app: AppHandle) -> Result<Vec<AvatarFolder>, String> {
    Ok(load_folders(&app))
}

#[tauri::command]
pub async fn folders_create(app: AppHandle, name: String) -> Result<AvatarFolder, String> {
    let mut folders = load_folders(&app);
    let order = folders.len() as u32;
    let folder = AvatarFolder {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        avatar_ids: vec![],
        color: None,
        order,
    };
    folders.push(folder.clone());
    save_folders(&app, &folders);
    Ok(folder)
}

#[tauri::command]
pub async fn folders_update(app: AppHandle, folder: AvatarFolder) -> Result<(), String> {
    let mut folders = load_folders(&app);
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder.id) {
        *f = folder;
    }
    save_folders(&app, &folders);
    Ok(())
}

#[tauri::command]
pub async fn folders_delete(app: AppHandle, folder_id: String) -> Result<(), String> {
    let mut folders = load_folders(&app);
    folders.retain(|f| f.id != folder_id);
    save_folders(&app, &folders);
    Ok(())
}

#[tauri::command]
pub async fn folders_add_avatar(
    app: AppHandle,
    folder_id: String,
    avatar_id: String,
) -> Result<(), String> {
    let mut folders = load_folders(&app);
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        if !f.avatar_ids.contains(&avatar_id) {
            f.avatar_ids.push(avatar_id);
        }
    }
    save_folders(&app, &folders);
    Ok(())
}

#[tauri::command]
pub async fn folders_remove_avatar(
    app: AppHandle,
    folder_id: String,
    avatar_id: String,
) -> Result<(), String> {
    let mut folders = load_folders(&app);
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        f.avatar_ids.retain(|id| id != &avatar_id);
    }
    save_folders(&app, &folders);
    Ok(())
}

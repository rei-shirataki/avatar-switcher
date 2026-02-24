use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use crate::storage::{AvatarFolder, AvatarOverride};

const STORE_PATH: &str = "app-settings.json";
const FOLDERS_KEY: &str = "avatar_folders";
const OVERRIDES_KEY: &str = "avatar_overrides";

fn load_folders(app: &AppHandle) -> Vec<AvatarFolder> {
    app.store(STORE_PATH)
        .ok()
        .and_then(|store| {
            store
                .get(FOLDERS_KEY)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        })
        .unwrap_or_default()
}

fn save_folders(app: &AppHandle, folders: &[AvatarFolder]) {
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(value) = serde_json::to_value(folders) {
            store.set(FOLDERS_KEY, value);
            store.save().ok();
        }
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

// ── Avatar overrides ──────────────────────────────────────────────────────────

fn load_overrides(app: &AppHandle) -> HashMap<String, AvatarOverride> {
    app.store(STORE_PATH)
        .ok()
        .and_then(|store| {
            store
                .get(OVERRIDES_KEY)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        })
        .unwrap_or_default()
}

fn save_overrides(app: &AppHandle, map: &HashMap<String, AvatarOverride>) {
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(value) = serde_json::to_value(map) {
            store.set(OVERRIDES_KEY, value);
            store.save().ok();
        }
    }
}

#[tauri::command]
pub async fn avatar_overrides_get_all(app: AppHandle) -> Result<Vec<AvatarOverride>, String> {
    Ok(load_overrides(&app).into_values().collect())
}

#[tauri::command]
pub async fn avatar_overrides_set(
    app: AppHandle,
    avatar_id: String,
    custom_name: Option<String>,
    custom_thumbnail: Option<String>,
) -> Result<AvatarOverride, String> {
    let mut map = load_overrides(&app);
    let existing = map.get(&avatar_id).cloned().unwrap_or_default();
    let updated = AvatarOverride {
        avatar_id: avatar_id.clone(),
        // None 引数 = 変更なし → 既存値を引き継ぐ
        custom_name: custom_name.or(existing.custom_name),
        custom_thumbnail: custom_thumbnail.or(existing.custom_thumbnail),
    };
    map.insert(avatar_id, updated.clone());
    save_overrides(&app, &map);
    Ok(updated)
}

#[tauri::command]
pub async fn avatar_overrides_delete(app: AppHandle, avatar_id: String) -> Result<(), String> {
    let mut map = load_overrides(&app);
    map.remove(&avatar_id);
    save_overrides(&app, &map);
    Ok(())
}

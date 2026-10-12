use once_cell::sync::Lazy;
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;
use crate::storage::{AvatarFolder, AvatarOverride, EyeHeightSettings, OverlaySettings, OverlayUiState};

const STORE_PATH: &str = "app-settings.json";
const FOLDERS_KEY: &str = "avatar_folders";
const OVERRIDES_KEY: &str = "avatar_overrides";
const OVERLAY_SETTINGS_KEY: &str = "overlay_settings";
const OVERLAY_UI_STATE_KEY: &str = "overlay_ui_state";
const EYE_HEIGHT_SETTINGS_KEY: &str = "eye_height_settings";

// 並列 invoke の read-modify-write 競合を防ぐためのプロセス全体ロック。
// tauri-plugin-store は単一 set/get 単位でしか同期しないため、
// load → 変更 → save の一連を必ず排他化する。
static FOLDERS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static OVERRIDES_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static OVERLAY_SETTINGS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static EYE_HEIGHT_SETTINGS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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

fn validate_folder_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("フォルダ名を入力してください".into());
    }
    if trimmed.chars().count() > 64 {
        return Err("フォルダ名は64文字以内で入力してください".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn folders_create(app: AppHandle, name: String) -> Result<AvatarFolder, String> {
    validate_folder_name(&name)?;
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app);
    // len() だと削除後に既存 order と重複するため最大値+1 にする。
    let order = folders.iter().map(|f| f.order + 1).max().unwrap_or(0);
    let folder = AvatarFolder {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
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
    validate_folder_name(&folder.name)?;
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app);
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder.id) {
        *f = folder;
    }
    save_folders(&app, &folders);
    Ok(())
}

#[tauri::command]
pub async fn folders_delete(app: AppHandle, folder_id: String) -> Result<(), String> {
    let _guard = FOLDERS_LOCK.lock().await;
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
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app);
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        if !f.avatar_ids.contains(&avatar_id) {
            f.avatar_ids.push(avatar_id);
        }
    }
    save_folders(&app, &folders);
    Ok(())
}

/// 複数アバターを 1 トランザクションでフォルダに追加する。
/// 並列の folders_add_avatar 呼び出しは個別ロックを取り直すため、
/// バルク追加では本コマンドを使うことで読み書きを 1 回に集約する。
#[tauri::command]
pub async fn folders_add_avatars(
    app: AppHandle,
    folder_id: String,
    avatar_ids: Vec<String>,
) -> Result<(), String> {
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app);
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        for id in avatar_ids {
            if !f.avatar_ids.contains(&id) {
                f.avatar_ids.push(id);
            }
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
    let _guard = FOLDERS_LOCK.lock().await;
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
    let _guard = OVERRIDES_LOCK.lock().await;
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
    let _guard = OVERRIDES_LOCK.lock().await;
    let mut map = load_overrides(&app);
    map.remove(&avatar_id);
    save_overrides(&app, &map);
    Ok(())
}

// ── Overlay settings (#28) ────────────────────────────────────────────────────

fn load_overlay_settings(app: &AppHandle) -> OverlaySettings {
    app.store(STORE_PATH)
        .ok()
        .and_then(|store| {
            store
                .get(OVERLAY_SETTINGS_KEY)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        })
        .unwrap_or_default()
}

fn save_overlay_settings(app: &AppHandle, settings: &OverlaySettings) {
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(value) = serde_json::to_value(settings) {
            store.set(OVERLAY_SETTINGS_KEY, value);
            store.save().ok();
        }
    }
}

#[tauri::command]
pub async fn overlay_settings_get(app: AppHandle) -> Result<OverlaySettings, String> {
    Ok(load_overlay_settings(&app))
}

/// 設定変更はサイドカー起動時のCLI引数(`--placement-mode`)経由でのみ反映されるため、
/// ここでは保存するだけで良い（サイドカーへの即時反映は行わない。次回起動時に適用される）。
#[tauri::command]
pub async fn overlay_settings_set(app: AppHandle, settings: OverlaySettings) -> Result<(), String> {
    let _guard = OVERLAY_SETTINGS_LOCK.lock().await;
    save_overlay_settings(&app, &settings);
    Ok(())
}

// ── Eye height settings (身長変更の上限適用オン/オフ) ───────────────────────────

fn load_eye_height_settings(app: &AppHandle) -> EyeHeightSettings {
    app.store(STORE_PATH)
        .ok()
        .and_then(|store| {
            store
                .get(EYE_HEIGHT_SETTINGS_KEY)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        })
        .unwrap_or_default()
}

fn save_eye_height_settings(app: &AppHandle, settings: &EyeHeightSettings) {
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(value) = serde_json::to_value(settings) {
            store.set(EYE_HEIGHT_SETTINGS_KEY, value);
            store.save().ok();
        }
    }
}

#[tauri::command]
pub async fn eye_height_settings_get(app: AppHandle) -> Result<EyeHeightSettings, String> {
    Ok(load_eye_height_settings(&app))
}

/// デスクトップUIから呼ばれる。overlay-uiとはSteamVR WSブリッジ経由でしか通信
/// できない（別プロセスでlocalStorageを共有できないため）ため、保存に加えて
/// `steamvr::bridge` の接続中クライアント全員へ即座にpushし、VR内パネルの
/// クランプ範囲もリアルタイムに切り替える。
#[tauri::command]
pub async fn eye_height_settings_set(
    app: AppHandle,
    settings: EyeHeightSettings,
) -> Result<(), String> {
    let _guard = EYE_HEIGHT_SETTINGS_LOCK.lock().await;
    save_eye_height_settings(&app, &settings);
    crate::steamvr::bridge::broadcast_eye_height_settings(settings.limit_enabled);
    Ok(())
}

// ── Overlay UI state (ソート/選択タブの永続化) ───────────────────────────────────
// デスクトップUIからは呼ばれず overlay-sidecar (WSブリッジ、`steamvr::bridge`) からのみ
// 使うため #[tauri::command] は付けない。単純な上書き保存のためロック不要
// （read-modify-writeではなくoverlay-ui側が確定させた値をそのまま保存するだけ）。

pub(crate) fn load_overlay_ui_state(app: &AppHandle) -> OverlayUiState {
    app.store(STORE_PATH)
        .ok()
        .and_then(|store| {
            store
                .get(OVERLAY_UI_STATE_KEY)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        })
        .unwrap_or_default()
}

pub(crate) fn save_overlay_ui_state(app: &AppHandle, state: &OverlayUiState) {
    if let Ok(store) = app.store(STORE_PATH) {
        if let Ok(value) = serde_json::to_value(state) {
            store.set(OVERLAY_UI_STATE_KEY, value);
            store.save().ok();
        }
    }
}

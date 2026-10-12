use once_cell::sync::Lazy;
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::Mutex;
use crate::storage::{AvatarFolder, AvatarOverride, EyeHeightSettings, OverlaySettings, OverlayUiState};

const STORE_PATH: &str = "app-settings.json";
const FOLDERS_KEY: &str = "avatar_folders";
const OVERRIDES_KEY: &str = "avatar_overrides";
const OVERLAY_SETTINGS_KEY: &str = "overlay_settings";
const OVERLAY_UI_STATE_KEY: &str = "overlay_ui_state";
const EYE_HEIGHT_SETTINGS_KEY: &str = "eye_height_settings";

/// ストアからキーの値を読む。キー未登録は既定値、ストアを開けない/値が壊れている
/// 場合は Err。後者を既定値で握り潰すと、続く save が空データで既存を上書きして
/// 保存済みのフォルダ等が無言で消えるため、更新系は Err で中断させる。
fn load_value<T: DeserializeOwned + Default>(app: &AppHandle, key: &str) -> Result<T, String> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| format!("設定ストアを開けませんでした: {e}"))?;
    match store.get(key) {
        None => Ok(T::default()),
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
            format!("設定「{key}」の読み込みに失敗しました（データを保護するため更新を中止します）: {e}")
        }),
    }
}

fn save_value<T: Serialize + ?Sized>(app: &AppHandle, key: &str, value: &T) -> Result<(), String> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| format!("設定ストアを開けませんでした: {e}"))?;
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    store.set(key, json);
    store.save().map_err(|e| format!("設定の保存に失敗しました: {e}"))
}

/// 参照系コマンド用。読み込み失敗は既定値で返しログに残す（書き込みはしないので安全）。
fn or_default<T: Default>(r: Result<T, String>) -> T {
    r.unwrap_or_else(|e| {
        log::warn!("[storage] {e}");
        T::default()
    })
}

// 並列 invoke の read-modify-write 競合を防ぐためのプロセス全体ロック。
// tauri-plugin-store は単一 set/get 単位でしか同期しないため、
// load → 変更 → save の一連を必ず排他化する。
static FOLDERS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static OVERRIDES_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static OVERLAY_SETTINGS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static EYE_HEIGHT_SETTINGS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn load_folders(app: &AppHandle) -> Result<Vec<AvatarFolder>, String> {
    load_value(app, FOLDERS_KEY)
}

fn save_folders(app: &AppHandle, folders: &[AvatarFolder]) -> Result<(), String> {
    save_value(app, FOLDERS_KEY, folders)
}

#[tauri::command]
pub async fn folders_get_all(app: AppHandle) -> Result<Vec<AvatarFolder>, String> {
    Ok(or_default(load_folders(&app)))
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
    let mut folders = load_folders(&app)?;
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
    save_folders(&app, &folders)?;
    Ok(folder)
}

#[tauri::command]
pub async fn folders_update(app: AppHandle, folder: AvatarFolder) -> Result<(), String> {
    validate_folder_name(&folder.name)?;
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app)?;
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder.id) {
        *f = folder;
    }
    save_folders(&app, &folders)?;
    Ok(())
}

#[tauri::command]
pub async fn folders_delete(app: AppHandle, folder_id: String) -> Result<(), String> {
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app)?;
    folders.retain(|f| f.id != folder_id);
    save_folders(&app, &folders)?;
    Ok(())
}

#[tauri::command]
pub async fn folders_add_avatar(
    app: AppHandle,
    folder_id: String,
    avatar_id: String,
) -> Result<(), String> {
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app)?;
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        if !f.avatar_ids.contains(&avatar_id) {
            f.avatar_ids.push(avatar_id);
        }
    }
    save_folders(&app, &folders)?;
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
    let mut folders = load_folders(&app)?;
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        for id in avatar_ids {
            if !f.avatar_ids.contains(&id) {
                f.avatar_ids.push(id);
            }
        }
    }
    save_folders(&app, &folders)?;
    Ok(())
}

#[tauri::command]
pub async fn folders_remove_avatar(
    app: AppHandle,
    folder_id: String,
    avatar_id: String,
) -> Result<(), String> {
    let _guard = FOLDERS_LOCK.lock().await;
    let mut folders = load_folders(&app)?;
    if let Some(f) = folders.iter_mut().find(|f| f.id == folder_id) {
        f.avatar_ids.retain(|id| id != &avatar_id);
    }
    save_folders(&app, &folders)?;
    Ok(())
}

// ── Avatar overrides ──────────────────────────────────────────────────────────

fn load_overrides(app: &AppHandle) -> Result<HashMap<String, AvatarOverride>, String> {
    load_value(app, OVERRIDES_KEY)
}

fn save_overrides(app: &AppHandle, map: &HashMap<String, AvatarOverride>) -> Result<(), String> {
    save_value(app, OVERRIDES_KEY, map)
}

#[tauri::command]
pub async fn avatar_overrides_get_all(app: AppHandle) -> Result<Vec<AvatarOverride>, String> {
    Ok(or_default(load_overrides(&app)).into_values().collect())
}

#[tauri::command]
pub async fn avatar_overrides_set(
    app: AppHandle,
    avatar_id: String,
    custom_name: Option<String>,
    custom_thumbnail: Option<String>,
) -> Result<AvatarOverride, String> {
    let _guard = OVERRIDES_LOCK.lock().await;
    let mut map = load_overrides(&app)?;
    let existing = map.get(&avatar_id).cloned().unwrap_or_default();
    let updated = AvatarOverride {
        avatar_id: avatar_id.clone(),
        // None 引数 = 変更なし → 既存値を引き継ぐ
        custom_name: custom_name.or(existing.custom_name),
        custom_thumbnail: custom_thumbnail.or(existing.custom_thumbnail),
    };
    map.insert(avatar_id, updated.clone());
    save_overrides(&app, &map)?;
    Ok(updated)
}

#[tauri::command]
pub async fn avatar_overrides_delete(app: AppHandle, avatar_id: String) -> Result<(), String> {
    let _guard = OVERRIDES_LOCK.lock().await;
    let mut map = load_overrides(&app)?;
    map.remove(&avatar_id);
    save_overrides(&app, &map)?;
    Ok(())
}

/// 指定値でそのまま置換する。`avatar_overrides_set` は `None` を「変更なし」と解釈する
/// ため片方だけを消せず、delete→set の非アトミックな再保存が必要だった。ここでは
/// `None` を「クリア」と解釈し、両方 `None` ならエントリごと削除して `None` を返す。
#[tauri::command]
pub async fn avatar_overrides_replace(
    app: AppHandle,
    avatar_id: String,
    custom_name: Option<String>,
    custom_thumbnail: Option<String>,
) -> Result<Option<AvatarOverride>, String> {
    let _guard = OVERRIDES_LOCK.lock().await;
    let mut map = load_overrides(&app)?;
    let result = replace_override(&mut map, avatar_id, custom_name, custom_thumbnail);
    save_overrides(&app, &map)?;
    Ok(result)
}

fn replace_override(
    map: &mut HashMap<String, AvatarOverride>,
    avatar_id: String,
    custom_name: Option<String>,
    custom_thumbnail: Option<String>,
) -> Option<AvatarOverride> {
    if custom_name.is_none() && custom_thumbnail.is_none() {
        map.remove(&avatar_id);
        return None;
    }
    let value = AvatarOverride {
        avatar_id: avatar_id.clone(),
        custom_name,
        custom_thumbnail,
    };
    map.insert(avatar_id, value.clone());
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_clears_only_the_field_passed_as_none() {
        let mut map = HashMap::new();
        replace_override(&mut map, "a".into(), Some("n".into()), Some("t".into()));
        let r = replace_override(&mut map, "a".into(), Some("n".into()), None).unwrap();
        assert_eq!(r.custom_name.as_deref(), Some("n"));
        assert!(r.custom_thumbnail.is_none());
    }

    #[test]
    fn replace_with_both_none_removes_entry() {
        let mut map = HashMap::new();
        replace_override(&mut map, "a".into(), Some("n".into()), None);
        assert!(replace_override(&mut map, "a".into(), None, None).is_none());
        assert!(map.is_empty());
    }
}

// ── Overlay settings (#28) ────────────────────────────────────────────────────

fn load_overlay_settings(app: &AppHandle) -> Result<OverlaySettings, String> {
    load_value(app, OVERLAY_SETTINGS_KEY)
}

fn save_overlay_settings(app: &AppHandle, settings: &OverlaySettings) -> Result<(), String> {
    save_value(app, OVERLAY_SETTINGS_KEY, settings)
}

#[tauri::command]
pub async fn overlay_settings_get(app: AppHandle) -> Result<OverlaySettings, String> {
    Ok(or_default(load_overlay_settings(&app)))
}

/// 設定変更はサイドカー起動時のCLI引数(`--placement-mode`)経由でのみ反映されるため、
/// ここでは保存するだけで良い（サイドカーへの即時反映は行わない。次回起動時に適用される）。
#[tauri::command]
pub async fn overlay_settings_set(app: AppHandle, settings: OverlaySettings) -> Result<(), String> {
    let _guard = OVERLAY_SETTINGS_LOCK.lock().await;
    save_overlay_settings(&app, &settings)?;
    Ok(())
}

// ── Eye height settings (身長変更の上限適用オン/オフ) ───────────────────────────

fn load_eye_height_settings(app: &AppHandle) -> Result<EyeHeightSettings, String> {
    load_value(app, EYE_HEIGHT_SETTINGS_KEY)
}

fn save_eye_height_settings(app: &AppHandle, settings: &EyeHeightSettings) -> Result<(), String> {
    save_value(app, EYE_HEIGHT_SETTINGS_KEY, settings)
}

#[tauri::command]
pub async fn eye_height_settings_get(app: AppHandle) -> Result<EyeHeightSettings, String> {
    Ok(or_default(load_eye_height_settings(&app)))
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
    // 永続化に失敗しても、実行中の OSC クランプ範囲と VR パネルには反映する。
    // 反映しないと UI で許可した値が Rust 側で無言で丸められる（#57 と同種の不整合）。
    let saved = save_eye_height_settings(&app, &settings);
    crate::steamvr::bridge::broadcast_eye_height_settings(settings.limit_enabled);
    saved
}

// ── Overlay UI state (ソート/選択タブの永続化) ───────────────────────────────────
// デスクトップUIからは呼ばれず overlay-sidecar (WSブリッジ、`steamvr::bridge`) からのみ
// 使うため #[tauri::command] は付けない。単純な上書き保存のためロック不要
// （read-modify-writeではなくoverlay-ui側が確定させた値をそのまま保存するだけ）。

pub(crate) fn load_overlay_ui_state(app: &AppHandle) -> OverlayUiState {
    or_default(load_value(app, OVERLAY_UI_STATE_KEY))
}

pub(crate) fn save_overlay_ui_state(app: &AppHandle, state: &OverlayUiState) {
    if let Err(e) = save_value(app, OVERLAY_UI_STATE_KEY, state) {
        log::warn!("[storage] {e}");
    }
}

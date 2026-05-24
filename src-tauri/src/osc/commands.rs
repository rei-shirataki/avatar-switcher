use crate::osc;
use serde::Serialize;

#[tauri::command]
pub async fn osc_change_avatar(avatar_id: String) -> Result<(), String> {
    osc::send_avatar_change(&avatar_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn osc_set_avatar_eye_height(value: f32) -> Result<(), String> {
    osc::send_avatar_eye_height(value).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct OscStatus {
    pub port: u16,
    pub vrchat_detected: bool,
}

/// 他のコマンドと戻り型を揃えるため Result<_, String> でラップする。
/// 現状失敗パスは無いが、将来 oscquery 側が fallible になっても呼び出し側を変えずに済む。
#[tauri::command]
pub async fn osc_get_status() -> Result<OscStatus, String> {
    Ok(OscStatus {
        port: osc::oscquery::get_vrchat_osc_port(),
        vrchat_detected: osc::oscquery::is_vrchat_detected(),
    })
}

use crate::osc;

#[tauri::command]
pub async fn osc_change_avatar(avatar_id: String) -> Result<(), String> {
    osc::send_avatar_change(&avatar_id).map_err(|e| e.to_string())
}

use crate::osc;

#[tauri::command]
pub async fn osc_change_avatar(avatar_id: String) -> Result<(), String> {
    osc::send_avatar_change(&avatar_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn osc_set_avatar_parameter_float(
    name: String,
    value: f32,
) -> Result<(), String> {
    osc::send_avatar_parameter_float(&name, value).map_err(|e| e.to_string())
}

use serde::{Deserialize, Serialize};
use rosc::OscType;
use crate::osc;

#[derive(Debug, Serialize, Deserialize)]
pub struct OscStatus {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum OscValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    String(String),
}

#[tauri::command]
pub async fn osc_change_avatar(avatar_id: String) -> Result<(), String> {
    osc::send_avatar_change(&avatar_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn osc_get_status() -> Result<OscStatus, String> {
    Ok(OscStatus { enabled: true })
}

#[tauri::command]
pub async fn osc_send_parameter(address: String, value: OscValue) -> Result<(), String> {
    // Only allow VRChat avatar parameter addresses to prevent misuse.
    if !address.starts_with("/avatar/parameters/") {
        return Err("OSC address must start with /avatar/parameters/".into());
    }
    let osc_val = match value {
        OscValue::Float(v) => OscType::Float(v),
        OscValue::Int(v) => OscType::Int(v),
        OscValue::Bool(v) => OscType::Bool(v),
        OscValue::String(v) => OscType::String(v),
    };
    osc::send_parameter(&address, osc_val).map_err(|e| e.to_string())
}

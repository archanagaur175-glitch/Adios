use crate::db::{queries, DbState};
use std::collections::HashMap;
use tauri::State;

/// Error mapping helper for command results.
fn lock_err<E: std::fmt::Display>(err: E) -> String {
    format!("db lock poisoned: {err}")
}

/// Return every stored setting (key → raw JSON string).
#[tauri::command]
pub fn get_settings(db: State<DbState>) -> Result<HashMap<String, String>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    queries::settings_get_all(&conn).map_err(|e| e.to_string())
}

/// Persist a single setting. The value must serialize to valid JSON.
#[tauri::command]
pub fn set_setting(
    db: State<DbState>,
    key: String,
    value_json: String,
) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(&value_json)
        .map_err(|e| format!("invalid JSON for '{key}': {e}"))?;
    let conn = db.0.lock().map_err(lock_err)?;
    queries::settings_set(&conn, &key, &value_json).map_err(|e| e.to_string())
}

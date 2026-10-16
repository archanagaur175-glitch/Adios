use crate::db::{queries, DbState};
use std::collections::HashMap;
use tauri::{Manager, State};

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

/// Apply the `cacheSizeMb` cap now (Settings → Cache + after cap changes).
/// Returns the pass stats; logs one line per run (empty passes included)
/// so the dev log doubles as the gate's eviction probe.
#[tauri::command]
pub fn run_cache_eviction(
    app: tauri::AppHandle,
    db: State<DbState>,
) -> Result<crate::server::evict::EvictStats, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("cache");
    let conn = db.0.lock().map_err(lock_err)?;
    let cap = crate::server::evict::cache_cap_bytes(&conn);
    let stats =
        crate::server::evict::evict_to_cap(&conn, &dir, cap).map_err(|e| e.to_string())?;
    println!(
        "[cache] evict removed={} parts={} orphans={} freed={}B left={}B cap={}B",
        stats.removed,
        stats.removed_parts,
        stats.removed_orphans,
        stats.freed_bytes,
        stats.remaining_bytes,
        cap
    );
    Ok(stats)
}

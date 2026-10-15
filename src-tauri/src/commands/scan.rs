use crate::db::DbState;
use crate::scan::{self, ScanSummary};
use tauri::{AppHandle, Manager, State};

/// Scan a folder for audio files, import tags + embedded art into the library.
/// Returns a summary; per-file errors are collected (never aborts the scan).
#[tauri::command]
pub fn scan_folder(
    folder: String,
    app: AppHandle,
    db: State<DbState>,
) -> Result<ScanSummary, String> {
    let covers_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let conn = db
        .0
        .lock()
        .map_err(|e| format!("db lock poisoned: {e}"))?;
    let summary = scan::import_folder(&conn, std::path::Path::new(&folder), &covers_dir);
    println!(
        "[scan] folder=\"{}\" added={} skipped={} failed={} in {}ms",
        summary.folder, summary.added, summary.skipped, summary.failed, summary.elapsed_ms
    );
    for error in &summary.errors {
        println!("[scan]   error: {error}");
    }
    Ok(summary)
}

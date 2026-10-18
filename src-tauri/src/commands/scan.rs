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

/// P18.1c: sub-directory listing for the in-app folder picker (Android's
/// plugin-dialog rejects `open({directory:true})` — no native folder UI).
/// Returns sorted dir names (hidden `.` entries excluded) + parent path.
#[derive(serde::Serialize)]
pub struct DirListing {
    pub parent: Option<String>,
    pub dirs: Vec<String>,
}

#[tauri::command]
pub fn list_directory(path: String) -> Result<DirListing, String> {
    let read = std::fs::read_dir(&path).map_err(|e| format!("{path}: {e}"))?;
    let mut dirs: Vec<String> = Vec::new();
    for entry in read.flatten() {
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_dir && !name.starts_with('.') {
            dirs.push(name);
        }
    }
    dirs.sort_unstable();
    let parent = std::path::Path::new(&path)
        .parent()
        .and_then(|p| p.to_str())
        .map(str::to_owned);
    Ok(DirListing { parent, dirs })
}

#[cfg(test)]
mod tests {
    use super::list_directory;

    #[test]
    fn list_directory_returns_sorted_dirs_parent_and_errors_on_missing() {
        let dir = std::env::temp_dir().join("adios-listdir-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("zeta")).unwrap();
        std::fs::create_dir_all(dir.join("alpha")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        std::fs::write(dir.join("song.mp3"), b"x").unwrap();

        let listing = list_directory(dir.to_string_lossy().into_owned()).unwrap();
        assert_eq!(listing.dirs, ["alpha", "zeta"]);
        assert!(listing.parent.is_some());
        assert!(list_directory("/no/such/dir/here".into()).is_err());
    }
}

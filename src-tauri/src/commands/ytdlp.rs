//! "Update yt-dlp" action (architecture §3.3, P4.4): download the official
//! Windows build into `<app-data>/bin/yt-dlp.exe`, verify it runs
//! (`--version`), then hot-swap the resolver's `BinHandle` and persist the
//! path so the next launch reuses it. Progress is emitted to the webview
//! (`ytdlp-progress` events) for the Settings UI.

use futures::StreamExt;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{queries, DbState};
use crate::resolver::ytdlp::BinHandle;

const RELEASE_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const SETTING_KEY: &str = "ytdlpPath";

#[tauri::command]
pub async fn update_ytdlp(
    app: AppHandle,
    db: State<'_, DbState>,
    bin: State<'_, BinHandle>,
) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let bin_dir = data_dir.join("bin");
    tokio::fs::create_dir_all(&bin_dir)
        .await
        .map_err(|e| format!("create bin dir: {e}"))?;
    let tmp = bin_dir.join("yt-dlp.exe.part");
    let dest = bin_dir.join("yt-dlp.exe");

    app.emit(
        "ytdlp-progress",
        serde_json::json!({ "stage": "downloading", "written": 0u64, "total": null }),
    )
    .map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let res = client
        .get(RELEASE_URL)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("download failed: HTTP {}", res.status()));
    }
    let total = res.content_length().unwrap_or(0);
    let mut stream = res.bytes_stream();
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("create temp file: {e}"))?;
    let mut written: u64 = 0;
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("download stream: {e}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("write temp file: {e}"))?;
        written += chunk.len() as u64;
        let _ = app.emit(
            "ytdlp-progress",
            serde_json::json!({ "stage": "downloading", "written": written, "total": total }),
        );
    }
    drop(file);
    if written == 0 {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err("download was empty".to_owned());
    }

    // Verify before swapping: the download must actually run.
    // sidecar_command hides the console window on Windows (P9.6 fix #1).
    let verify = crate::resolver::ytdlp::sidecar_command(&tmp)
        .arg("--version")
        .output()
        .await;
    let out = match verify {
        Ok(out) if out.status.success() => out,
        Ok(_) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err("--version check failed on the downloaded build".to_owned());
        }
        Err(err) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(format!("downloaded file failed to run: {err}"));
        }
    };
    let version = String::from_utf8_lossy(&out.stdout).trim().to_owned();

    if let Err(err) = tokio::fs::rename(&tmp, &dest).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(format!(
            "could not replace {}: {err} (another resolve may be running — retry)",
            dest.display()
        ));
    }

    // Hot-swap for this session + persist for the next launch.
    *bin.write().await = dest.clone();
    {
        let path_json =
            serde_json::to_string(&dest.to_string_lossy()).map_err(|e| e.to_string())?;
        let conn = db.0.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        queries::settings_set(&conn, SETTING_KEY, &path_json).map_err(|e| e.to_string())?;
    }

    let _ = app.emit(
        "ytdlp-progress",
        serde_json::json!({ "stage": "done", "version": version, "path": dest.to_string_lossy() }),
    );
    Ok(version)
}

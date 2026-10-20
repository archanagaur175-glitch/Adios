//! P19.1 (D33): Android yt-dlp bridge skeleton. `resolve_android`
//! proves the Rust → Kotlin plugin round-trip (registration, payload
//! serde, response shape) before 19.2 ports resolve/batch to Kotlin.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::resolver::mobile::YtDlpMobile;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingOut {
    pub ok: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn resolve_android(mobile: State<'_, YtDlpMobile>) -> Result<PingOut, String> {
    #[cfg(target_os = "android")]
    {
        mobile
            .0
            .run_mobile_plugin_async("ping", serde_json::json!({}))
            .await
            .map_err(|e| format!("ytdlp plugin: {e}"))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = mobile;
        Err("resolve_android is Android-only (desktop uses the Rust chain)".into())
    }
}

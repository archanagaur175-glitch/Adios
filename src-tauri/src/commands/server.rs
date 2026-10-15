use serde::Serialize;
use tauri::State;

/// Loopback server connection info handed to the webview (one invoke at startup).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub port: u16,
    pub token: String,
    pub base_url: String,
}

#[tauri::command]
pub fn server_info(info: State<ServerInfo>) -> ServerInfo {
    info.inner().clone()
}

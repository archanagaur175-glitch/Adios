mod commands;
mod db;
mod scan;
mod server;
mod win;

use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window should exist");
            let effect = win::vibrancy::apply(&window);
            println!("[vibrancy] applied: {effect}");

            let data_dir = app.path().app_data_dir().expect("app data dir");
            let conn = db::open(&data_dir.join("adios.db")).expect("open database");
            app.manage(db::DbState::new(conn));
            println!("[db] opened at {}", data_dir.join("adios.db").display());

            let loopback = server::start().expect("loopback server");
            println!("[server] listening on {}", loopback.base_url());
            app.manage(commands::server::ServerInfo {
                port: loopback.port,
                token: loopback.token.clone(),
                base_url: loopback.base_url(),
            });
            app.manage(server::ManagedServer(loopback));

            // Global media keys (play/pause/next/prev/stop) → "media-key" events.
            win::media_keys::spawn(app.handle().clone());

            // Debug harness channel: webview smoke logs (src/audio/smoke.ts).
            let _ = app.listen("smoke", |event| {
                println!("[smoke] {}", event.payload());
            });

            // Debug-only smoke harness: run one scan at startup so the
            // real command + lofty path can be verified headlessly.
            #[cfg(debug_assertions)]
            if let Ok(dir) = std::env::var("ADIOS_AUTO_SCAN") {
                let result = commands::scan::scan_folder(
                    dir,
                    app.handle().clone(),
                    app.state::<db::DbState>(),
                );
                match result {
                    Ok(s) => println!(
                        "[smoke] auto-scan done: added={} skipped={} failed={}",
                        s.added, s.skipped, s.failed
                    ),
                    Err(err) => println!("[smoke] auto-scan failed: {err}"),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::library::list_tracks,
            commands::library::get_track,
            commands::library::insert_track,
            commands::library::delete_track,
            commands::server::server_info,
            commands::scan::scan_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod commands;
mod db;
mod meta;
mod resolver;
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
            let db_state = db::DbState::new(conn);
            let shared_db = db_state.shared();
            app.manage(db_state);
            println!("[db] opened at {}", data_dir.join("adios.db").display());

            // Resolver chain (Invariant 2): LocalFile → Cache → yt-dlp →
            // Piped/Invidious. yt-dlp binary: env override > setting > PATH.
            let ytdlp_path = std::env::var("ADIOS_YTDLP")
                .ok()
                .or_else(|| {
                    shared_db.lock().ok().and_then(|conn| {
                        conn.query_row(
                            "SELECT value_json FROM settings WHERE key = 'ytdlpPath'",
                            [],
                            |row| row.get::<_, String>(0),
                        )
                        .ok()
                    }).and_then(|raw| serde_json::from_str::<String>(&raw).ok())
                })
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("yt-dlp"));
            let bin = resolver::ytdlp::bin_handle(ytdlp_path);
            app.manage(bin.clone());

            let cache_dir = data_dir.join("cache");
            let chain = std::sync::Arc::new(resolver::ResolverChain::new(vec![
                std::sync::Arc::new(resolver::local::LocalFileResolver::new(shared_db.clone())),
                std::sync::Arc::new(resolver::cache::CacheResolver::new(
                    shared_db.clone(),
                    cache_dir.clone(),
                )),
                std::sync::Arc::new(resolver::ytdlp::YtDlpResolver::new(bin)),
                std::sync::Arc::new(resolver::piped::PipedResolver::new(shared_db.clone())),
            ]));
            app.manage(chain.clone());
            // Resolution table powers the single re-resolve on URL expiry.
            let table = std::sync::Arc::new(resolver::ResolutionTable::new());
            app.manage(table.clone());

            // Metadata search (P5.1): cache-first MB client behind the
            // global 1-req/s queue. UA placeholder per D2 — contact email
            // field + warning land with the search UI (5.3).
            let mb_client =
                meta::mb_client::MbClient::with_default_transport(shared_db.clone(), meta::mb_client::user_agent(None));
            app.manage(mb_client);

            let loopback =
                server::start(shared_db, cache_dir, chain, table).expect("loopback server");
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
            commands::scan::scan_folder,
            commands::resolver::resolve_query,
            commands::resolver::resolver_health,
            commands::resolver::test_instance,
            commands::ytdlp::update_ytdlp,
            commands::search::search_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

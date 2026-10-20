mod commands;
mod db;
mod meta;
mod net;
mod resolver;
mod scan;
mod server;
mod win;

use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // D10/P9.3: single-instance must register FIRST; a second launch focuses
    // the existing main window instead of opening a duplicate.
    #[cfg_attr(mobile, allow(unused_mut))]
    let mut builder = tauri::Builder::default();
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_focus();
            }
        }));
    }
    builder
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
            // Piped/Invidious. yt-dlp binary slot (P22/D35): env override >
            // settings Update copy > installer-bundled copy > PATH.
            let ytdlp_path = resolver::ytdlp_resolve::startup_path(
                std::env::var("ADIOS_YTDLP").ok(),
                shared_db.lock().ok().and_then(|conn| {
                    conn.query_row(
                        "SELECT value_json FROM settings WHERE key = 'ytdlpPath'",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .ok()
                }),
                app.path()
                    .resolve("bin/yt-dlp.exe", tauri::path::BaseDirectory::Resource)
                    .ok(),
            );
            println!("[ytdlp] binary slot -> {}", ytdlp_path.display());
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
            // global 1-req/s queue. The D2 UA is built per network call from
            // the `contactEmail` setting (5.3 — placeholder until saved).
            let mb_client = meta::mb_client::MbClient::with_default_transport(shared_db.clone());
            app.manage(mb_client);

            // iTunes track picker (P9.7): keyless, relevance-ordered,
            // cache-first, own 400 ms spacing queue (never MB's 1 rps).
            app.manage(meta::itunes::ItunesClient::with_default_transport(
                shared_db.clone(),
            ));

            // RSS top-songs chart (P16.5): 6 h age-aware meta_cache row,
            // stale-on-error so Home's Discover shelf survives offline.
            app.manage(meta::itunes_rss::RssClient::with_default_transport(
                shared_db.clone(),
            ));

            // Cover Art Archive (P5.2): no CAA rate limits per docs, so the
            // client does not share the MB 1-req/s queue.
            app.manage(meta::cover_art::CoverArtClient::default());

            // LRCLIB lyrics (P7.1): cache-first, own 1-req/s queue (never MB's).
            app.manage(meta::lrclib::LyricsClient::with_default_transport(
                shared_db.clone(),
            ));

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

            // P6.1: bands bridge start/stop proof lines in the dev log
            // (webview console output is not forwarded to stdout).
            let _ = app.listen("bands", |event| {
                println!("[bands] {}", event.payload());
            });

            // P6.3: visualizer fps/tier proof lines.
            let _ = app.listen("viz", |event| {
                println!("[viz] {}", event.payload());
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
            commands::settings::run_cache_eviction,
            commands::library::list_tracks,
            commands::library::get_track,
            commands::library::insert_track,
            commands::library::delete_track,
            commands::server::server_info,
            commands::scan::scan_folder,
            commands::scan::list_directory,
            commands::resolver::resolve_query,
            commands::resolver::resolve_batch,
            commands::resolver::resolver_health,
            commands::resolver::test_instance,
            commands::ytdlp::update_ytdlp,
            commands::search::search_metadata,
            commands::search::search_itunes,
            commands::search::rss_top_songs,
            commands::search::search_local,
            commands::library::add_stream_track,
            commands::cover::fetch_cover,
            commands::lyrics::get_lyrics,
            commands::playlists::playlist_create,
            commands::playlists::playlist_rename,
            commands::playlists::playlist_delete,
            commands::playlists::playlist_list,
            commands::playlists::playlist_detail,
            commands::playlists::playlist_add_track,
            commands::playlists::playlist_remove_track,
            commands::playlists::playlist_reorder,
            commands::playlists::playlist_export_file,
            commands::playlists::playlist_import_file,
            commands::session::session_save_queue,
            commands::session::session_load_queue,
            commands::session::list_cached_tracks,
            commands::history::record_play,
            commands::history::list_play_history,
            commands::download::download_track,
            commands::session::cached_track_ids,
            commands::spotify::spotify_fetch_playlist
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

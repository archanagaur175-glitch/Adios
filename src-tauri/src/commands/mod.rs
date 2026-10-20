pub mod cover;
pub mod download;
pub mod history;
pub mod library;
pub mod lyrics;
pub mod mobile;
pub mod playlists;
pub mod resolver;
pub mod scan;
pub mod search;
pub mod server;
pub mod session;
pub mod settings;
pub mod spotify;
pub mod ytdlp;

/// Single invoke_handler list — lives here so lib.rs stays under the
/// 200-line cap (expanded in lib.rs via `adios_invoke_handler!()`).
#[macro_export]
macro_rules! adios_invoke_handler {
    () => {
        tauri::generate_handler![
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
            commands::mobile::resolve_android,
            commands::mobile::resolve_android_ping,
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
        ]
    };
}

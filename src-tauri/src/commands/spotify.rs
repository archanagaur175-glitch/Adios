//! P17.1: Spotify embed-import command (D18) — fetches a PUBLIC
//! playlist's full track list anonymously for the Playlists page's
//! import box; no Premium, no dev app, no login.

#[tauri::command]
pub async fn spotify_fetch_playlist(
    url: String,
) -> Result<crate::meta::spotify_embed::SpotifyPlaylistOut, String> {
    let client = reqwest::Client::new();
    crate::meta::spotify_embed::fetch_full_playlist(
        &client,
        &crate::meta::spotify_embed::EmbedBases::prod(),
        &url,
    )
    .await
}

//! P17.1 (D18): serde shapes for Spotify's public embed SSR payloads +
//! the anonymous playlist API page — verified live 2026-10-17 (see
//! BUILD_PLAN D18). Unofficial endpoints: every field is Option so a
//! shape change degrades into an honest error, never a panic.

use serde::Deserialize;

/// `__NEXT_DATA__` wrapper: props.pageProps.state.data.entity (+ the
/// anonymous session token under state.settings.session).
#[derive(Debug, Deserialize)]
pub struct EmbedNext<T> {
    pub props: EmbedProps<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedProps<T> {
    pub page_props: EmbedPageProps<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedPageProps<T> {
    /// Option: error pages (unknown/private playlist ids) ship a
    /// NEXT_DATA whose pageProps has no `state` at all.
    pub state: Option<EmbedState<T>>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedState<T> {
    pub data: Option<EmbedData<T>>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedData<T> {
    pub entity: Option<T>,
}

/// Playlist embed entity (open.spotify.com/embed/playlist/{id}).
#[derive(Debug, Deserialize)]
pub struct PlaylistEntity {
    pub name: Option<String>,
    /// camelCase `trackList` — the first ≤100 tracks WITH metadata.
    #[serde(rename = "trackList")]
    pub track_list: Option<Vec<EmbedTrackItem>>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedTrackItem {
    pub uri: Option<String>,
    pub title: Option<String>,
    /// Comma-joined artist names.
    pub subtitle: Option<String>,
    /// Milliseconds.
    pub duration: Option<i64>,
}

/// Track embed entity (open.spotify.com/embed/track/{id}) — metadata
/// fallback for tracks beyond the playlist page's 100-item cap.
#[derive(Debug, Deserialize)]
pub struct TrackEntity {
    pub uri: Option<String>,
    pub name: Option<String>,
    /// Milliseconds.
    pub duration: Option<i64>,
    pub artists: Option<Vec<EmbedArtist>>,
    /// P21 art source: cover variants live under here (300/64/640 px).
    #[serde(rename = "visualIdentity")]
    pub visual_identity: Option<VisualIdentity>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedArtist {
    pub name: Option<String>,
}

/// P21: track visual identity — colors we ignore + `image[]` covers.
#[derive(Debug, Deserialize)]
pub struct VisualIdentity {
    pub image: Option<Vec<EmbedImage>>,
}

/// One cover variant; `maxHeight`/`maxWidth` pick the largest usable one.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct EmbedImage {
    pub url: Option<String>,
    #[serde(rename = "maxHeight")]
    pub max_height: Option<i64>,
    #[serde(rename = "maxWidth")]
    pub max_width: Option<i64>,
}

/// P21: public oEmbed response (only the field we take).
#[derive(Debug, Deserialize)]
pub struct OEmbedResponse {
    #[serde(rename = "thumbnail_url")]
    pub thumbnail_url: Option<String>,
}

/// spclient playlist-v2 page (spclient.wg.spotify.com/playlist/v2/…)
/// — uri-only items; `length` is the playlist's TRUE total.
#[derive(Debug, Deserialize)]
pub struct SpPlaylist {
    pub length: Option<i64>,
    pub contents: Option<SpContents>,
}

#[derive(Debug, Deserialize)]
pub struct SpContents {
    pub items: Option<Vec<SpItem>>,
}

#[derive(Debug, Deserialize)]
pub struct SpItem {
    pub uri: Option<String>,
}

/// One track for the webview (camelCase).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyTrackOut {
    pub uri: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: Option<i64>,
    /// P21: album art — track pages carry it directly, playlist-page tracks
    /// get it from the oEmbed fill pass; None = import continues coverless.
    pub artwork_url: Option<String>,
}

/// Full playlist fetch result (camelCase).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyPlaylistOut {
    pub name: String,
    pub total: i64,
    pub tracks: Vec<SpotifyTrackOut>,
}

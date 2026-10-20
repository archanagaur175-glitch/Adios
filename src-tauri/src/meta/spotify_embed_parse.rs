//! P17.1 pure parsing for the Spotify embed SSR payloads (D18) — no
//! network, no tokio: url/id handling, `__NEXT_DATA__` + token
//! extraction, entity → output mapping. Shapes: spotify_embed_types.rs.

use super::spotify_embed_types::*;

/// Accepts a web URL (`…/playlist/{id}`, query/fragment OK), a
/// `spotify:playlist:{id}` URI, or a bare 22-char base62 id.
pub fn parse_playlist_id(input: &str) -> Result<String, String> {
    let s = input.trim();
    let candidate = if let Some(rest) = s.strip_prefix("spotify:playlist:") {
        rest.split(['?', '#']).next().unwrap_or(rest)
    } else if let Some(idx) = s.find("/playlist/") {
        let rest = &s[idx + "/playlist/".len()..];
        rest.split(['/', '?', '#']).next().unwrap_or("")
    } else if s.len() == 22 && !s.contains(['/', ':']) {
        s
    } else {
        return Err("Not a Spotify playlist link — paste a link like open.spotify.com/playlist/<id>".to_owned());
    };
    if candidate.len() != 22 || !candidate.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("Not a Spotify playlist link — no playlist id found in \"{input}\""));
    }
    Ok(candidate.to_owned())
}

/// The `<script id="__NEXT_DATA__" type="application/json">…</script>` body.
pub fn extract_next_data(html: &str) -> Option<&str> {
    const MARK: &str = "<script id=\"__NEXT_DATA__\" type=\"application/json\">";
    let start = html.find(MARK)? + MARK.len();
    let rest = &html[start..];
    let end = rest.find("</script>")?;
    Some(&rest[..end])
}

/// Anonymous embed token from `"settings":{"session":{"accessToken":"…"}}`.
pub fn extract_access_token(html: &str) -> Option<String> {
    const MARK: &str = "\"accessToken\":\"";
    let start = html.find(MARK)? + MARK.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    let tok = &rest[..end];
    if tok.len() < 20 {
        return None;
    }
    Some(tok.to_owned())
}

pub fn track_id_from_uri(uri: &str) -> Option<&str> {
    uri.strip_prefix("spotify:track:")
}

/// P21: largest cover from `visualIdentity.image[]` (height first, then
/// width — 640px wins over 300/64); url-less entries are ignored.
pub fn pick_artwork(images: &[EmbedImage]) -> Option<String> {
    images
        .iter()
        .filter(|i| i.url.is_some())
        .max_by_key(|i| (i.max_height.unwrap_or(0), i.max_width.unwrap_or(0)))
        .and_then(|i| i.url.clone())
}

/// P21: public oEmbed endpoint for one track (unencoded nested url is
/// legal in a query and matches the live probe).
pub fn oembed_url(base_open: &str, tid: &str) -> String {
    format!("{base_open}/oembed?url=https://open.spotify.com/track/{tid}")
}

/// P21: oEmbed JSON → thumbnail (None = missing/blank, never a panic).
pub fn oembed_thumbnail(body: &str) -> Option<String> {
    let parsed: OEmbedResponse = serde_json::from_str(body).ok()?;
    parsed
        .thumbnail_url
        .map(|u| u.trim().to_owned())
        .filter(|u| !u.is_empty())
}

/// Playlist embed page → (name, first-page tracks with metadata).
pub fn playlist_from_html(html: &str) -> Result<(String, Vec<SpotifyTrackOut>), String> {
    let data = extract_next_data(html).ok_or("Spotify embed page had no data — the import endpoint may have changed")?;
    let next: EmbedNext<PlaylistEntity> =
        serde_json::from_str(data).map_err(|e| format!("Spotify embed page parse failed: {e}"))?;
    let entity = next
        .props
        .page_props
        .state
        .and_then(|s| s.data)
        .and_then(|d| d.entity)
        .ok_or("Playlist not found or private — Adios can only import public playlists")?;
    let name = entity
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Spotify playlist".to_owned());
    let tracks = entity
        .track_list
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| {
            let uri = t.uri?;
            if track_id_from_uri(&uri).is_none() {
                return None;
            }
            Some(SpotifyTrackOut {
                uri,
                title: t.title.unwrap_or_default(),
                artist: t.subtitle.unwrap_or_default(),
                duration_ms: t.duration,
                // Playlist pages carry no art — the oEmbed fill pass runs later.
                artwork_url: None,
            })
        })
        .collect();
    Ok((name, tracks))
}

/// Track embed page → one metadata row (title/artist/duration).
pub fn track_from_html(html: &str) -> Result<SpotifyTrackOut, String> {
    let data = extract_next_data(html).ok_or("Spotify track page had no data — the import endpoint may have changed")?;
    let next: EmbedNext<TrackEntity> =
        serde_json::from_str(data).map_err(|e| format!("Spotify track page parse failed: {e}"))?;
    let entity = next
        .props
        .page_props
        .state
        .and_then(|s| s.data)
        .and_then(|d| d.entity)
        .ok_or("Spotify track page had no track")?;
    let uri = entity
        .uri
        .filter(|u| track_id_from_uri(u).is_some())
        .ok_or("Spotify track page missing its track uri")?;
    let artist = entity
        .artists
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| a.name)
        .find(|n| !n.trim().is_empty())
        .unwrap_or_default();
    let artwork_url = entity
        .visual_identity
        .and_then(|v| v.image)
        .map(|images| pick_artwork(&images))
        .unwrap_or(None);
    Ok(SpotifyTrackOut {
        uri,
        title: entity.name.unwrap_or_default(),
        artist,
        duration_ms: entity.duration,
        artwork_url,
    })
}

/// spclient playlist-v2 body → (true total, track uris on this page).
pub fn sp_uris_from_json(body: &str) -> Result<(i64, Vec<String>), String> {
    let page: SpPlaylist = serde_json::from_str(body).map_err(|e| format!("Spotify API page parse failed: {e}"))?;
    let total = page.length.unwrap_or(0);
    let uris = page
        .contents
        .and_then(|c| c.items)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| i.uri)
        .filter(|u| track_id_from_uri(u).is_some())
        .collect();
    Ok((total, uris))
}

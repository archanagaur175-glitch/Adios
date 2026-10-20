//! P17.1 (D18): anonymous Spotify playlist import via the PUBLIC embed
//! SSR pages + the page-embedded API token. No Premium, no dev app, no
//! login, no third-party tools (user-approved workaround 2026-10-17).
//!
//! Flow: `open.spotify.com/embed/playlist/{id}` → `__NEXT_DATA__` entity
//! (name + first ≤100 tracks WITH metadata) + `settings.session.accessToken`
//! → `spclient.wg.spotify.com/playlist/v2/playlist/{id}?from=N&length=M`
//! (uri-only pages; `length` field = true total) → tracks 101+ get
//! metadata from `/embed/track/{id}` (1 GET each, 8 concurrent).
//! Unofficial endpoints: any shape change surfaces as an honest Err.
//! Pure parsing lives in spotify_embed_parse.rs.

pub use super::spotify_embed_parse::*;
pub use super::spotify_embed_types::*;

pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
const CHUNK: usize = 8;

/// Base URLs — swappable in tests for a local axum fixture server.
#[derive(Clone)]
pub struct EmbedBases {
    pub open: String,
    pub spclient: String,
}

impl EmbedBases {
    pub fn prod() -> Self {
        Self {
            open: "https://open.spotify.com".to_owned(),
            spclient: "https://spclient.wg.spotify.com".to_owned(),
        }
    }
}

async fn get_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| format!("Spotify request failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Spotify returned HTTP {status} for {url}"));
    }
    resp.text().await.map_err(|e| format!("Spotify response read failed: {e}"))
}

async fn spclient_page(
    client: &reqwest::Client,
    bases: &EmbedBases,
    token: &str,
    id: &str,
    from: i64,
    length: i64,
) -> Result<String, String> {
    let url = format!("{}/playlist/v2/playlist/{id}?from={from}&length={length}", bases.spclient);
    let resp = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, UA)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| format!("Spotify request failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Spotify returned HTTP {status} for the playlist API"));
    }
    resp.text().await.map_err(|e| format!("Spotify response read failed: {e}"))
}

/// Metadata fallback for one track beyond the first page.
pub async fn fetch_track_meta(
    client: &reqwest::Client,
    bases: &EmbedBases,
    uri: &str,
) -> Result<SpotifyTrackOut, String> {
    let tid = track_id_from_uri(uri).ok_or_else(|| format!("Not a track uri: {uri}"))?;
    let html = get_text(client, &format!("{}/embed/track/{tid}", bases.open)).await?;
    track_from_html(&html)
}

/// P21: album art for one track via the PUBLIC oEmbed endpoint (tiny JSON —
/// playlist pages and spclient carry no art, verified 2026-10-19). Always
/// best-effort: any failure is None and the import continues coverless.
pub async fn fetch_track_art(
    client: &reqwest::Client,
    bases: &EmbedBases,
    uri: &str,
) -> Option<String> {
    let tid = track_id_from_uri(uri)?;
    match get_text(client, &oembed_url(&bases.open, tid)).await {
        Ok(body) => oembed_thumbnail(&body),
        Err(e) => {
            eprintln!("[spotify] artwork fetch failed: {e}");
            None
        }
    }
}

/// Full import fetch: first 100 detailed from the playlist page, remaining
/// uris via the token-paginated API, their metadata via track pages.
pub async fn fetch_full_playlist(
    client: &reqwest::Client,
    bases: &EmbedBases,
    input: &str,
) -> Result<SpotifyPlaylistOut, String> {
    let id = parse_playlist_id(input)?;
    let html = get_text(client, &format!("{}/embed/playlist/{id}", bases.open)).await?;
    // Entity first: a missing entity is the honest "not found or private"
    // (error pages also lack the session token — don't blame the token).
    let (name, mut tracks) = playlist_from_html(&html)?;
    let token = extract_access_token(&html).ok_or("Spotify embed token missing — the import endpoint may have changed")?;
    let total = match spclient_page(client, bases, &token, &id, 0, 1).await {
        Ok(body) => sp_uris_from_json(&body)?.0.max(tracks.len() as i64),
        Err(e) => {
            eprintln!("[spotify] total probe failed ({e}); using the first page only");
            tracks.len() as i64
        }
    };
    let mut extra: Vec<String> = Vec::new();
    if total > tracks.len() as i64 {
        let mut from = tracks.len() as i64;
        while from < total {
            let len = (total - from).min(100);
            let body = spclient_page(client, bases, &token, &id, from, len)
                .await
                .map_err(|e| format!("Spotify playlist page at {from} failed: {e}"))?;
            let (_, uris) = sp_uris_from_json(&body)?;
            if uris.is_empty() {
                break;
            }
            extra.extend(uris);
            from += len;
        }
    }
    println!("[spotify] {name} — {total} tracks ({} detailed, {} to fetch)", tracks.len(), extra.len());
    let mut meta_ok = 0usize;
    let mut meta_fail = 0usize;
    for chunk in extra.chunks(CHUNK) {
        let futs = chunk.iter().map(|uri| fetch_track_meta(client, bases, uri));
        for res in futures::future::join_all(futs).await {
            match res {
                Ok(t) => {
                    tracks.push(t);
                    meta_ok += 1;
                }
                Err(e) => {
                    eprintln!("[spotify] track metadata failed: {e}");
                    meta_fail += 1;
                }
            }
        }
    }
    if meta_fail > 0 {
        println!("[spotify] metadata: {meta_ok} ok, {meta_fail} skipped");
    }

    // P21: fill art for the tracks that arrived without it (playlist-page
    // rows always; track-page rows when visualIdentity was absent) — 8
    // concurrent oEmbed GETs, per-track degrade, never fails the import.
    let missing: Vec<usize> = tracks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.artwork_url.is_none())
        .map(|(i, _)| i)
        .collect();
    if !missing.is_empty() {
        let mut art_ok = 0usize;
        for chunk in missing.chunks(CHUNK) {
            let futs = chunk.iter().map(|&i| fetch_track_art(client, bases, &tracks[i].uri));
            for (slot, art) in chunk.iter().zip(futures::future::join_all(futs).await) {
                if let Some(url) = art {
                    tracks[*slot].artwork_url = Some(url);
                    art_ok += 1;
                }
            }
        }
        println!("[spotify] artwork: {art_ok}/{} fetched", missing.len());
    }
    Ok(SpotifyPlaylistOut { name, total, tracks })
}

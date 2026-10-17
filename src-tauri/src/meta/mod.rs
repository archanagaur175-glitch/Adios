//! MusicBrainz + Cover Art Archive metadata (P5.1–P5.2) and LRCLIB lyrics
//! (P7.1): MB 1-req/s rate-limit queue + cache-first JSON search client,
//! CAA front-cover fetcher with 512² JPEG shaping + 3-color palette
//! extraction, the iTunes search client (P9.7) + the RSS top-songs chart
//! client behind it (P16.5), and the LRCLIB cache-first
//! lyrics client (hit + negative rows). API shapes verified live 2026-10-15/16 —
//! see BUILD_PLAN VERIFICATION LOG (musicbrainz.org/doc/{MusicBrainz_API,
//! MusicBrainz_API/Search,Cover_Art_Archive/API}; lrclib.net/api/get|search
//! + github.com/tranxuanthang/lrclib server source). Import enrichment
//! (P5.4) was removed in P12 at the user's request — `cover_store` and
//! `fetch_cover` remain as the cover entry points.
pub mod cover_art;
pub mod cover_store;
pub mod itunes;
pub mod itunes_rss;
pub mod itunes_rss_types;
pub mod lrclib;
pub mod lrclib_types;
pub mod mb_client;
pub mod mb_parse;
pub mod mb_types;
pub mod palette;
pub mod rate_limit;
pub mod spotify_embed;
pub mod spotify_embed_parse;
pub mod spotify_embed_types;
pub mod stream_art;

#[cfg(test)]
mod cover_art_tests;
#[cfg(test)]
mod itunes_rss_tests;
#[cfg(test)]
mod itunes_tests;
#[cfg(test)]
mod lrclib_tests;
#[cfg(test)]
mod lrclib_url_tests;
#[cfg(test)]
mod mb_client_tests;
#[cfg(test)]
mod mb_live_tests;
#[cfg(test)]
mod mb_types_tests;
#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod rate_limit_tests;
#[cfg(test)]
mod spotify_embed_tests;
#[cfg(test)]
mod stream_art_tests;

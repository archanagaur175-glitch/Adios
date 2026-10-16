//! MusicBrainz + Cover Art Archive metadata (P5.1–P5.4) and LRCLIB lyrics
//! (P7.1): MB 1-req/s rate-limit queue + cache-first JSON search client,
//! CAA front-cover fetcher with 512² JPEG shaping + 3-color palette
//! extraction (shared with import enrichment), the enrichment matcher
//! (title sim + ±3 s duration gate), and the LRCLIB cache-first lyrics
//! client (hit + negative rows). API shapes verified live 2026-10-15/16 —
//! see BUILD_PLAN VERIFICATION LOG (musicbrainz.org/doc/{MusicBrainz_API,
//! MusicBrainz_API/Search,Cover_Art_Archive/API}; lrclib.net/api/get|search
//! + github.com/tranxuanthang/lrclib server source).
pub mod cover_art;
pub mod cover_store;
pub mod enrich;
pub mod itunes;
pub mod lrclib;
pub mod lrclib_types;
pub mod mb_client;
pub mod mb_parse;
pub mod mb_types;
pub mod palette;
pub mod rate_limit;

#[cfg(test)]
mod cover_art_tests;
#[cfg(test)]
mod enrich_match_tests;
#[cfg(test)]
mod itunes_tests;
#[cfg(test)]
mod enrich_tests;
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

//! MusicBrainz + Cover Art Archive metadata (P5.1/P5.2): MB 1-req/s
//! rate-limit queue + cache-first JSON search client, and the CAA front-cover
//! fetcher with 512² JPEG shaping + 3-color palette extraction. API shapes
//! verified live 2026-10-15 — see BUILD_PLAN VERIFICATION LOG
//! (musicbrainz.org/doc/{MusicBrainz_API,MusicBrainz_API/Search,Cover_Art_Archive/API}).
pub mod cover_art;
pub mod mb_client;
pub mod mb_parse;
pub mod mb_types;
pub mod palette;
pub mod rate_limit;

#[cfg(test)]
mod cover_art_tests;
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

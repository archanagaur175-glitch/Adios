//! MusicBrainz metadata (P5.1): 1-req/s rate-limit queue + JSON search client
//! with `meta_cache` write-back. API shapes verified live 2026-10-15 — see
//! BUILD_PLAN VERIFICATION LOG (musicbrainz.org/doc/MusicBrainz_API{/Search}).
pub mod mb_client;
pub mod mb_types;
pub mod rate_limit;

#[cfg(test)]
mod mb_client_tests;
#[cfg(test)]
mod mb_types_tests;
#[cfg(test)]
mod rate_limit_tests;

//! Single re-resolve for expired/forbidden upstream URLs (P4.4,
//! architecture §3.2/§3.3): lookup the original query in the resolution
//! table, claim the ONE allowed attempt, invalidate the stale meta_cache
//! payload, and run the chain. New remote URLs retarget the table entry
//! (carrying the claim forward); local candidates are served directly.

use std::path::PathBuf;

use crate::resolver::cache::{invalidate_resolution, is_remote_url, put_resolution};
use crate::server::handlers::ServerState;

pub enum ReResolved {
    Url(String),
    Path(PathBuf),
}

pub async fn re_resolve_once(state: &ServerState, failed_url: &str) -> Option<ReResolved> {
    let query = state.table.query_for(failed_url)?;
    if !state.table.claim(failed_url) {
        println!("[proxy] re-resolve already used for {failed_url}");
        return None;
    }
    // Drop the stale payload so the chain cannot serve the failed URL again.
    let _ = invalidate_resolution(&state.db, &query);
    match state.chain.resolve(&query).await {
        Ok(candidate) if is_remote_url(&candidate.url_or_path) => {
            state.table.retarget(failed_url, &candidate.url_or_path, &query);
            let _ = put_resolution(&state.db, &query, &candidate);
            Some(ReResolved::Url(candidate.url_or_path))
        }
        Ok(candidate) => Some(ReResolved::Path(PathBuf::from(candidate.url_or_path))),
        Err(err) => {
            println!("[proxy] re-resolve miss: {err}");
            None
        }
    }
}

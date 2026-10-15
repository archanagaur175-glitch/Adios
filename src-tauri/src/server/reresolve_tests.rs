//! P4.4 tests: proxy single re-resolve on upstream 4xx (mock upstream,
//! mock chain — no network).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::resolver::{
    Candidate, Health, HealthStatus, Query, ResolveError, Resolver, ResolverChain,
    ResolutionTable,
};
use crate::server::test_util::test_start_with;

fn query() -> Query {
    Query {
        artist: "Arijit Singh".to_owned(),
        title: "Apna Bana Le".to_owned(),
        album: None,
        duration_ms: Some(261_000),
    }
}

/// Mock upstream: /expired and /expired2 always 403, /fresh returns bytes.
async fn upstream_403() -> u16 {
    let router = Router::new()
        .route(
            "/expired",
            get(|| async { axum::http::StatusCode::FORBIDDEN }),
        )
        .route(
            "/expired2",
            get(|| async { axum::http::StatusCode::FORBIDDEN }),
        )
        .route("/fresh", get(|| async { "fresh-bytes" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });
    port
}

struct MockResolver {
    calls: Arc<AtomicUsize>,
    next_url: String,
}

impl Resolver for MockResolver {
    fn name(&self) -> &'static str {
        "mock"
    }
    fn health<'a>(&'a self) -> crate::resolver::BoxFuture<'a, Health> {
        Box::pin(std::future::ready(Health {
            status: HealthStatus::Ok,
            latency_ms: 0,
        }))
    }
    fn resolve<'a>(
        &'a self,
        _query: &'a Query,
    ) -> crate::resolver::BoxFuture<'a, Result<Candidate, ResolveError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let url = self.next_url.clone();
        Box::pin(std::future::ready(Ok(Candidate {
            url_or_path: url,
            mime: Some("text/plain".to_owned()),
            expires_at: None,
            source: "mock".to_owned(),
            confidence: 0.9,
        })))
    }
}

fn media_url(server: &super::LoopbackServer) -> String {
    format!("{}/media/{}", server.base_url(), server.token)
}

#[tokio::test]
async fn expired_url_reresolves_once_then_serves_fresh() {
    let port = upstream_403().await;
    let expired = format!("http://127.0.0.1:{port}/expired");
    let fresh = format!("http://127.0.0.1:{port}/fresh");

    let calls = Arc::new(AtomicUsize::new(0));
    let table = ResolutionTable::new();
    table.register(&expired, &query());
    let chain = ResolverChain::new(vec![Arc::new(MockResolver {
        calls: calls.clone(),
        next_url: fresh.clone(),
    })]);

    let (server, _db, _dir) = test_start_with(chain, table);
    let res = reqwest::Client::new()
        .get(media_url(&server))
        .query(&[("url", expired.as_str())])
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200, "re-resolved request must succeed");
    assert_eq!(res.text().await.unwrap(), "fresh-bytes");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "exactly one resolve");
}

#[tokio::test]
async fn second_failure_is_not_reresolved_again() {
    let port = upstream_403().await;
    let expired = format!("http://127.0.0.1:{port}/expired");
    let expired2 = format!("http://127.0.0.1:{port}/expired2");

    let calls = Arc::new(AtomicUsize::new(0));
    let table = ResolutionTable::new();
    table.register(&expired, &query());
    let chain = ResolverChain::new(vec![Arc::new(MockResolver {
        calls: calls.clone(),
        next_url: expired2.clone(),
    })]);

    let (server, _db, _dir) = test_start_with(chain, table);
    let res = reqwest::Client::new()
        .get(media_url(&server))
        .query(&[("url", expired.as_str())])
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403, "second failure passes through");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "claim must allow only ONE resolve across the URL chain"
    );
}

#[tokio::test]
async fn unknown_url_never_resolves() {
    let port = upstream_403().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let chain = ResolverChain::new(vec![Arc::new(MockResolver {
        calls: calls.clone(),
        next_url: "unused".to_owned(),
    })]);

    let (server, _db, _dir) = test_start_with(chain, ResolutionTable::new());
    let url = format!("http://127.0.0.1:{port}/expired");
    let res = reqwest::Client::new()
        .get(media_url(&server))
        .query(&[("url", url.as_str())])
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403, "no table entry → passthrough");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no resolve for unknown urls");
}

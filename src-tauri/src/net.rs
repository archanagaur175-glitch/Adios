//! Shared HTTP client factory.
//!
//! All production reqwest clients go through here so TLS never touches
//! rustls-platform-verifier: reqwest's `rustls` feature wires that crate in,
//! but on Android it must be initialized against the app's JVM/Context
//! (and shipped with its `org.rustls.platformverifier` Kotlin artifact) —
//! Tauri does neither, so the first HTTPS verification panics, and the
//! release profile (`panic = "abort"`) turns that into the startup SIGABRT
//! seen at gate183 ("Expect rustls-platform-verifier to be initialized").
//! Preconfiguring rustls with webpki-roots (Mozilla roots) keeps TLS fully
//! in-process for the Piped/metadata endpoints this app talks to (D4/D19).

use std::sync::Arc;

/// reqwest client with webpki-roots verification (never the platform verifier).
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .tls_backend_preconfigured(tls_config())
        .build()
        .expect("http client with preconfigured tls")
}

fn tls_config() -> rustls::ClientConfig {
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("tls protocol versions")
    .with_root_certificates(roots)
    .with_no_client_auth()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builds_with_webpki_roots() {
        let client = http_client();
        let _request = client.get("https://example.invalid/"); // no network: builder only
        let _ = tls_config();
    }
}

pub mod handlers;
pub mod range;
pub mod stream;

#[cfg(test)]
mod tests;

use handlers::{router, ServerState};
use std::sync::{mpsc, Arc};
use tokio::sync::oneshot;

/// Keeps the loopback server alive for the whole app session
/// (dropping the handle shuts it down via the oneshot sender).
pub struct ManagedServer(
    /// Held purely for its lifetime; not read.
    #[allow(dead_code)] pub LoopbackServer,
);

/// Loopback audio server (Invariant 1): 127.0.0.1, ephemeral port,
/// per-session random token, Host validation, Range + CORS.
pub struct LoopbackServer {
    pub port: u16,
    pub token: String,
    shutdown: Option<oneshot::Sender<()>>,
}

impl LoopbackServer {
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Build a media URL for a local file path.
    /// Used by tests and by future Rust-side callers; the webview builds
    /// equivalent URLs from `base_url` + `token` with `encodeURIComponent`.
    #[allow(dead_code)]
    pub fn media_url(&self, path: &std::path::Path) -> String {
        let encoded = range::encode_path(&path.to_string_lossy());
        format!("{}/media/{}?path={}", self.base_url(), self.token, encoded)
    }

    /// Explicit teardown (tests); in production the process exit stops the server.
    #[allow(dead_code)]
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

/// Bind 127.0.0.1:0 and serve until `shutdown()` or process exit.
/// Blocking (uses an internal thread + runtime) so it can be called
/// from Tauri's synchronous `setup`.
pub fn start() -> std::io::Result<LoopbackServer> {
    let token = uuid::Uuid::new_v4().to_string();
    let (ready_tx, ready_rx) = mpsc::channel::<u16>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let token_for_server = token.clone();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("loopback runtime");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind loopback");
            let port = listener.local_addr().expect("local addr").port();
            let state = Arc::new(ServerState {
                token: token_for_server,
                port,
            });
            ready_tx.send(port).ok();
            let serve = axum::serve(listener, router(state))
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
            if let Err(err) = serve {
                eprintln!("[server] loopback server error: {err}");
            }
        });
    });

    let port = ready_rx
        .recv()
        .map_err(|_| std::io::Error::other("server thread died"))?;
    Ok(LoopbackServer {
        port,
        token,
        shutdown: Some(shutdown_tx),
    })
}

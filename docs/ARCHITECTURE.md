# Adios — Architecture

Local personal-use Windows desktop music player. Tauri v2 (Rust) + React 19 + Vite + TypeScript (strict) + Tailwind v4 + Zustand + SQLite (rusqlite, bundled, FTS5). Rust crates: axum, reqwest, tokio, lofty, window-vibrancy, tracing.
Tagline: "Say goodbye to ads and subscription paywalls." Single machine, never published, no accounts, no servers. No three.js / WebGL in this version (an optional 3D layer is deferred; see §9).

## 1. High-level topology

```
┌───────────────────────── WebView (React 19) ─────────────────────────┐
│  Tailwind v4 UI · custom titlebar · react-router · Zustand stores    │
│  PlayerEngine (two <audio> A/B, crossOrigin="anonymous")             │
│  Web Audio: MediaElementSource → AnalyserNode → GainNode → output    │
│  audioBands bridge → CSS vars (--bass/--mid/--high/--rms) via rAF    │
│  ONE 2D canvas visualizer + Aurora backdrop (layered 2D immersive)   │
└───────────▲──────────────────────────────▲───────────────────────────┘
     invoke (JSON only)             http://127.0.0.1:<ephemeral>
     commands/queries               loopback audio server (axum)
┌───────────┴──────────────────────────────┴───────────────────────────┐
│ Rust core                                                            │
│  • Loopback server: token + Host validation, Range, CORS,            │
│    streams local files AND remote audio; writes disk cache           │
│  • Resolver chain behind ONE trait `Resolver`                        │
│    LocalFile → Cache → YtDlpSidecar → Piped/Invidious fallback       │
│  • yt-dlp sidecar + in-app "Update yt-dlp"                           │
│  • Metadata: MusicBrainz (1 rps queue) · Cover Art Archive · LRCLIB  │
│  • SQLite (rusqlite bundled): tracks, playlists, queue, cache_index, │
│    lyrics_cache, meta_cache, settings + FTS5 over tracks             │
│  • window-vibrancy (Mica/Acrylic), media keys, single-instance,      │
│    tracing logs, NSIS installer                                      │
└──────────────────────────────────────────────────────────────────────┘
Storage: %APPDATA%\Adios (adios.db, cache\, covers\, logs\)
```

**Invariant 1 — all audio via the loopback server.** Audio bytes NEVER pass through Tauri `invoke`, and the Tauri asset protocol is NOT used for audio. Every playable item (local file or remote URL) is served by ONE Rust axum server bound to `127.0.0.1` on an ephemeral port:

- URL shape: `http://127.0.0.1:<port>/media/<random-per-session-token>?…`
- The session token is generated at startup in Rust, handed to the webview via one invoke, embedded in media URLs, and validated on every request; wrong token → 403.
- Host-header validation (allowlist of `127.0.0.1:<port>` / `localhost:<port>`) → mismatch returns 403 (DNS-rebinding hardening). Test required.
- HTTP Range pass-through (206 responses) for seeking; `Accept-Ranges: bytes`.
- CORS: only the app origin is allowed (`Access-Control-Allow-Origin` echoes the verified app origin, `Access-Control-Allow-Credentials: false`). `<audio>` elements always use `crossOrigin="anonymous"` so the AnalyserNode receives real data. One code path for local + remote = no asset-protocol cross-origin question.

**Invariant 3 — local playback works with no network and no resolver.** LocalFile resolver maps a path to the loopback URL directly; the server streams from disk. No DNS, no yt-dlp.

## 2. Webview layer

### 2.1 PlayerEngine
- Two `<audio>` elements (A/B), both `crossOrigin="anonymous"`, both feeding `MediaElementAudioSourceNode`s.
- Gapless path (default): when the current element's `timeupdate` is within `crossfadeMs` of `duration`, set B.src = next track's loopback URL, `play()`, then swap; with `crossfadeMs = 0` this is a preload-and-switch (near-gapless).
- Optional equal-power crossfade when `crossfadeMs > 0`: equal-power curves on the two GainNodes over the window; swap after fade-out completes.
- Graph: `MediaElementSource(A|B) → Gain(A|B) → AnalyserNode → masterGain → destination`.
- Analyser: `fftSize 2048`, `smoothingTimeConstant 0.8`. Pure util derives bands: bass 20–150 Hz, mid 150 Hz–2 kHz, high 2–12 kHz, plus RMS.
- Engine is a plain TS class (no React), driven by `usePlayerStore` subscriptions. Store owns queue/position/volume/state; engine owns the elements and Web Audio graph.

### 2.2 audioBands bridge (Invariant 6)
- rAF loop reads the AnalyserNode, computes smoothed bands (asymmetric easing: attack 0.35 / release 0.08), writes `--bass`, `--mid`, `--high`, `--rms` (0–1 numbers) onto `document.documentElement.style`.
- rAF loop is STARTED only when: playback is active AND document.visibilityState === 'visible'. Stopped otherwise; when paused it writes a static resting frame once.
- Components consume these via CSS (`calc`, `color-mix`) or via ref-reads in their own rAF — NEVER via React state. No rAF-driven React re-renders.

### 2.3 Media Session / global keys
- `navigator.mediaSession` metadata + handlers (play, pause, previoustrack, nexttrack, seekto).
- Rust: RegisterHotKey-style global media keys when the app is unfocused; forwarded to the webview via a Tauri event.
- In-app shortcuts (Space, ←/→, ↑/↓, Ctrl+K, L, Q, M) handled in a single React keydown hook.

### 2.4 Router
- react-router (v7) with a single `AppShell` layout route and child routes: `/` (Home), `/library`, `/playlists`, `/playlists/:id`, `/search` (also an overlay), `/cached`, `/settings`. Chosen in DECISIONS.

## 3. Rust core

### 3.1 SQLite (rusqlite bundled, FTS5)
Tables (migration 001):
- `tracks(id, path UNIQUE NULL, source, url NULL, artist, title, album, duration_ms, year, track_no, disc_no, mb_recording_id NULL, mb_release_id NULL, cover_path NULL, accent NULL, accent2 NULL, dominant_colors_json NULL, file_size, mtime, added_at, updated_at, play_count, last_played_at)`
- `tracks_fts` — FTS5 virtual table (artist, title, album) + insert/delete/update triggers.
- `playlists(id, name, created_at, updated_at)` · `playlist_tracks(playlist_id, track_id, position, PRIMARY KEY(playlist_id, track_id))`
- `queue(track_id, position)` — persisted session queue.
- `cache_index(url, local_path, mime, bytes, expires_at NULL, last_access, hit_count)`
- `lyrics_cache(track_key PRIMARY KEY, synced_json NULL, plain_text NULL, fetched_at)` — negative rows = misses cached.
- `meta_cache(cache_key PRIMARY KEY, kind, payload_json, fetched_at)`
- `settings(key PRIMARY KEY, value_json)`
Migrations run at startup in a transaction; each has a unit test.

### 3.2 Loopback server (axum)
- `tokio` runtime shared with Tauri; `axum::serve` on `127.0.0.1:0`; bound port + token discovered via invoke `server_info`.
- `GET /media/:token` handler: validate token + Host, look up source (local path or remote URL from an in-memory resolution table populated by resolver), stream via `reqwest` (remote, Range forwarded) or `tokio::fs` (local, Range parsed), stream to disk cache on remote (temp → rename), correct MIME, CORS allowlist, 206 on Range, 403 on bad token/Host.
- Disk cache writes update `cache_index`.

### 3.3 Resolver chain (Invariant 2)
```rust
trait Resolver {
    fn name(&self) -> &'static str;
    async fn health(&self) -> Health;   // Ok | Degraded | Down + latency
    async fn resolve(&self, q: &Query) -> Result<Candidate, ResolveError>;
}
// Query:  { artist, title, album, duration_ms }
// Candidate: { url_or_path, mime, expires_at, source, confidence }
```
- Chain order (settings-configurable): LocalFile → Cache → YtDlpSidecar → Piped/Invidious.
- Candidate scoring: duration delta ±3 s (hard reject beyond), token-set title similarity, "official audio"/"topic" hints, reject live/cover/remix unless track metadata says so. Unit-tested.
- yt-dlp sidecar: resolve via `-J` (metadata JSON) and stream URL extraction (`-g`-equivalent via format selection `-f bestaudio[ext=m4a]/bestaudio`); prefer m4a (AAC) or webm (Opus) audio-only — both play natively in WebView2. NO ffmpeg in v1. Spike must record whether YouTube challenges require an external JS runtime (yt-dlp "EJS") or a PO-token provider (bgutil); Node 22 is available. Default: no PO provider unless the spike fails.
- "Update yt-dlp" action: runs the sidecar's self-update or fetches the latest release binary into the app data dir; progress surfaced in UI.
- Piped/Invidious: EMPTY instance list shipped; settings has instance list editor + "Test instance" button; never hard-coded.
- URL expiry: on 403/expiry during proxy fetch → re-resolve once, retry; if it fails again, surface error chip.

### 3.4 Metadata clients
- MusicBrainz: UA `Adios/0.1 (personal use; contact: <email from settings>)`; email settings field; until filled, send UA with placeholder and show a one-time warning. ONE global queue, ≥1000 ms between requests (unit-tested). Responses → `meta_cache` (TTL 30 d).
- Cover Art Archive: front cover by MBID → `covers/<mbid>.jpg` + downscale to 512² for hero art; palette = 3 dominant colors from a 32×32 downscale, cached on the track row.
- LRCLIB: `GET /api/get?artist_name=&track_name=&album_name=&duration=` → fallback `GET /api/search?q=`; prefer `syncedLyrics`; store hits AND misses in `lyrics_cache`.
- Local tags on import: `lofty` reads tags + embedded cover art (written to `covers/` when present) during folder scan.

### 3.5 Local-first search (Invariant 4)
1. Typing → instant FTS5 results from `tracks_fts` (no debounce needed beyond ~50 ms keystroke coalescing).
2. Debounced 250–300 ms → MusicBrainz search (through the 1-rps queue) → results appended to the same grouped UI (Tracks / Albums / Artists), marked as online results.
3. Everything MB/CAA returns is cached in `meta_cache`; repeat queries never hit the network (logged).

### 3.6 Windows integration
- `window-vibrancy`: Mica on Win 11; Acrylic fallback; custom titlebar with drag region + min/max/close buttons.
- Single-instance plugin; optional autostart setting; NSIS installer via tauri bundler.
- `tracing` + rolling file logs in `%APPDATA%\Adios\logs\`.

## 4. State management (Zustand)
- `usePlayerStore` — current track, queue, position, volume, crossfadeMs, isPlaying, sourceChip state.
- `useLibraryStore` — tracks, playlists, filters, FTS results, scan progress.
- `useUiStore` — sidebar, expanded Now Playing, immersive lyrics, modals, toasts, quality tier, eco flag.
- `useSearchStore` — query, local results (sync), online results (async, debounced), keyboard nav index.
- `useSettingsStore` — persisted settings mirror (resolver order, instances, cache size, crossfade, email, tier).
- Sync: invoke for reads/writes; Tauri events for server port/token, scan progress, resolver health, yt-dlp update progress.

## 5. Module layout
```
src/
  audio/        engine.ts, graph.ts, bands.ts, mediaSession.ts
  visuals/      audioBandsBridge.ts, aurora.ts, palette.ts, VisualizerHost.tsx, visualizer2d.ts, tiers.ts, idle.ts
  store/        playerStore.ts, libraryStore.ts, uiStore.ts, searchStore.ts, settingsStore.ts
  lib/          api.ts, debounce.ts, lrc.ts, format.ts, shortcuts.ts
  components/   titlebar/, sidebar/, player/, search/, library/, playlists/, lyrics/, queue/, common/
  routes/       Home.tsx, Library.tsx, Playlists.tsx, PlaylistDetail.tsx, Search.tsx, Cached.tsx, Settings.tsx
src-tauri/src/
  main.rs, lib.rs
  db/           mod.rs, migrations/, queries.rs
  server/       mod.rs, handlers.rs, range.rs, cache.rs
  resolver/     mod.rs (trait), local.rs, cache.rs, ytdlp.rs, piped.rs, score.rs
  meta/         musicbrainz.rs, coverart.rs, lrclib.rs, rate_limit.rs
  win/          vibrancy.rs, media_keys.rs
  commands/     library.rs, settings.rs, player.rs, resolver.rs, search.rs, lyrics.rs
```
Every file ≤ 200 lines, one responsibility. TypeScript strict; `any` only with an explanatory comment.

## 6. Error handling & observability
- `tracing` levels: error (user-visible failures), warn (fallbacks, re-resolve), info (lifecycle), debug (resolver decisions).
- Proxy failures → one re-resolve → UI toast + source chip "Unresolved". Local playback never blocked by metadata/network failures (Invariant 3).
- Scan errors collected per-file; scan completes with a summary.

## 7. Security / privacy
- Loopback-only server; per-session random token; Host allowlist; CORS app-origin-only; no telemetry; no accounts; no remote config; no asset-protocol audio.

## 8. Performance budget (2D)
- At most 4 `backdrop-filter` elements visible at once, blur ≤ 24 px; animated layers use PRE-blurred static layers, never live blur on moving elements.
- Animate only `transform` and `opacity`.
- One 2D canvas visualizer, DPR ≤ 1.5, drawn only while playing + visible.
- rAF loops (audioBands, visualizer, aurora drift) all gated on visibility + playing; paused = one static frame; hidden = fully stopped.
- Quality tiers: Low (48 bars / 30 fps) · Med (96 / 60) · High (128 / 60 + glow pass) · Eco (no backdrop-filter, static aurora, visualizer off, 30 fps cap, auto-suggest on battery).
- `prefers-reduced-motion`: drift/tilt/spin off, static aurora, instant transitions.

## 9. Deferred: 3D layer
The audio-reactive pipeline is exposed behind a `VisualizerHost` interface + the generic `audioBands` bridge. An optional 3D (three.js/R3F) layer can be added later as P10 without touching the PlayerEngine, resolver, or stores. Not planned in detail for this version.

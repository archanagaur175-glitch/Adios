# Adios — Architecture

Merged architecture document for the code at HEAD. It combines the repository's
original design notes with facts verified against the source. Items that could not
be verified statically are listed under [Unknowns](#unknowns).

Local personal-use Windows desktop music player: Tauri v2 (Rust core) + React 19 +
Vite 8 + TypeScript (strict) + Tailwind 4 + Zustand + SQLite (`rusqlite`, bundled,
FTS5).

## 1. Topology

```
WebView (React 19)
  Tailwind UI, custom titlebar, React Router, Zustand stores
  PlayerEngine: two <audio> elements + Web Audio analyser
  one 2D canvas visualiser + aurora backdrop
        │ invoke (JSON)                 │ HTTP 127.0.0.1:<ephemeral>
        ▼                               ▼
Rust core
  axum loopback audio server: per-session token, Host validation,
    Range, CORS, local + remote streaming, disk cache
  resolver chain: LocalFile → Cache → yt-dlp → Piped/Invidious
  metadata: iTunes, MusicBrainz, LRCLIB, Spotify embed
  SQLite: tracks, playlists, queue, cache_index, lyrics_cache,
    meta_cache, settings + FTS5 over tracks
  window-vibrancy, media keys, single-instance, tracing, NSIS
```

Storage lives under the Tauri app data directory and contains `adios.db` (plus WAL),
`cache/`, `covers/` and `logs/`.

### Invariant: all audio through the loopback server

Audio bytes never pass through Tauri `invoke`, and the asset protocol is not used
for audio. Every playable item is served by one axum server bound to `127.0.0.1` on
an ephemeral port:

- URL shape: `http://127.0.0.1:<port>/media/<random-per-session-token>`.
- The session token is generated at startup, handed to the webview through one
  invoke, embedded in media URLs and validated per request.
- The Host header is validated, and CORS allows only the app origin.
- HTTP Range requests are supported for seeking.
- Local playback needs no network and no resolver: the LocalFile resolver maps a
  path directly to the loopback URL.

## 2. Webview layer

- `PlayerEngine` uses two `<audio>` elements (A/B) with
  `crossOrigin="anonymous"`, feeding Web Audio nodes. With `crossfadeMs = 0` it
  preloads and switches near-gaplessly; with a positive value it applies an
  equal-power crossfade.
- An analyser (`fftSize 2048`, smoothing `0.8`) derives bass (20–150 Hz), mid
  (150 Hz–2 kHz), high (2–12 kHz) and RMS values, written to CSS custom properties
  by a `requestAnimationFrame` loop that runs only while playing and visible.
- `navigator.mediaSession` provides metadata and transport handlers. Global media
  keys are forwarded from Rust as Tauri events. In-app shortcuts are handled by one
  keydown hook.
- React Router 7 with an `AppShell` layout and 9 routes: Home, Library, Playlists,
  PlaylistDetail, Search, Cached, Settings and MoodView, plus the shell.

Frontend modules live under `src/` (132 files) across audio, components, lib,
routes, store, styles and visuals. The API layer mirrors the Rust commands in
`src/lib/api.ts`, `playlistApi.ts`, `downloadApi.ts`, `historyApi.ts`,
`sessionApi.ts`, `resolverApi.ts` and `onlineApi.ts`.

## 3. Rust core

### 3.1 Commands

43 Tauri commands are registered through `adios_invoke_handler!()` across 15
modules:

`cover`, `download`, `history`, `library`, `lyrics`, `mobile`, `playlists`,
`resolver`, `scan`, `search`, `server`, `session`, `settings`, `spotify`, `ytdlp`.

The playlist module alone exposes 10 commands: create, rename, delete, list, detail,
add_track, remove_track, reorder, export_file, import_file. Resolver exposes
`resolve_query`, `resolve_batch`, `resolver_health` and `test_instance`, plus three
Android variants.

### 3.2 Loopback server

The server module is split into 8 units: `cache`, `evict`, `handlers`, `proxy`,
`range`, `reresolve`, `stream` and `tee`. It serves:

- local files through `tokio::fs` with parsed Range requests,
- remote audio through `reqwest` with Range forwarded,
- writes to the disk cache with a temp-then-rename flow, updating `cache_index`,
- `206` responses for Range requests and `403` for a bad token or Host.

### 3.3 Resolver chain

Order: LocalFile → Cache → yt-dlp → Piped/Invidious.

The yt-dlp integration runs as a sidecar, streaming audio-only formats. Piped and
Invidious instances are user-configurable, and the settings surface includes a
"test instance" action. On URL expiry during a proxy fetch, the server re-resolves
once and retries; a second failure surfaces an error in the UI.

### 3.4 Metadata clients

| Source | Role |
| --- | --- |
| iTunes | Search and RSS metadata |
| MusicBrainz | Recording and release metadata |
| LRCLIB | Lyrics, synced preferred |
| Spotify embed | Additional metadata |
| Local tags (`lofty`) | Tags and embedded cover art during scans |

Results are cached in `meta_cache`; cover art is downscaled and stored under
`$APPDATA/covers`.

### 3.5 Search

Typing queries FTS5 (`tracks_fts`) for instant local results. A debounced online
pass adds metadata results to the same grouped UI. Metadata responses are cached so
repeat queries do not hit the network.

### 3.6 Windows integration

Mica/Acrylic vibrancy, a custom titlebar with drag region and window controls,
single-instance handling, media keys, `tracing` logs in the app data directory, and
an NSIS installer target.

## 4. Data model

SQLite `adios.db`, opened at startup; migrations run through a `schema_migrations`
table.

| Migration | Change |
| --- | --- |
| `m001_init` | Base tables and FTS5 |
| `m002_lyrics_cache` | Lyrics cache |
| `m003_download_pins` | Download pins |
| `m004_genre` | Nullable `genre` column, backfilled on next scan |

`m001` creates `tracks`, `playlists`, `playlist_tracks`, `queue`, `cache_index`,
`lyrics_cache`, `meta_cache` and `settings`, plus the FTS5 virtual table
`tracks_fts` with insert/delete/update triggers. `tracks` is indexed on `artist`
and `album`.

DB access is split into focused modules: `fts`, `history`, `lyrics_cache`,
`playlist`, `playlist_io`, `queries`, `session` and `stream`. Queue state and cached
track ids persist through `session_save_queue`, `session_load_queue` and
`cached_track_ids`.

## 5. Configuration

| Name | Purpose |
| --- | --- |
| `ADIOS_YTDLP` | Override the yt-dlp binary path |
| `ADIOS_AUTO_SCAN` | Startup scan directory |
| `TAURI_DEV_HOST` | Dev host |
| `VITE_ADIOS_SMOKE` | Frontend smoke-test flag |
| `VITE_ADIOS_XFADE_TEST` | Frontend crossfade-test flag |

User settings persist in `settings` as JSON values with keys such as `ytdlpPath`.
No `.env` file and no committed credentials exist.

## 6. Tests and CI

| Suite | Inventory |
| --- | --- |
| Vitest | 14 files: audio retry, batch resolve, colour, LRC parsing, online play, play gating, playlist save, recommendations, Android resolve, Spotify API, taste, download store, player store, search ranking |
| Rust | 178 `#[test]` / `#[tokio::test]` functions in dedicated `*_tests.rs` modules for db, meta, resolver, scan and server |

**No CI workflows are tracked.** Build, test and lint run locally only. Test counts
come from file inspection and were not re-executed.

## 7. Security and privacy

- Loopback-only HTTP server with a per-session random token, Host allowlist and
  app-origin CORS.
- No telemetry, no accounts, no remote config.
- No asset-protocol audio.
- CSP is disabled (`"csp": null`) in the Tauri window config while the app loads
  remote audio and metadata. This is a weaker default than the rest of the boundary
  suggests.

## 8. Packaging

- Window: 1280×800, minimum 900×600, decorations off, transparent.
- Bundle target: NSIS. `tauri.windows.conf.json` adds `bin/yt-dlp.exe` as a Windows
  resource.
- No mobile build configuration is committed, although the codebase contains an
  Android resolver plugin and mobile-only commands.

## Unknowns

- Whether `npm run build` and `cargo test` pass at this commit.
- Whether the configured Piped/Invidious instances are reachable.
- Whether the Android build produces an installable APK.
- Whether the phase gates recorded in `BUILD_PLAN.md` were actually run as recorded.

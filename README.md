# Adios

A local, personal-use desktop music player for Windows, built on Tauri v2 (Rust
core) and React 19.

Adios plays a local library and resolves tracks on demand for streaming. All audio
— local files and remote streams alike — is served to the webview by a single
loopback HTTP server rather than through Tauri's asset protocol or IPC.

## Features

- **Local library** — folder scan, tag reading via `lofty`, SQLite storage and FTS5
  search over artist, title and album.
- **On-demand playback** — a resolver chain of LocalFile → Cache → yt-dlp →
  Piped/Invidious.
- **One audio path** — a loopback server on `127.0.0.1` with an ephemeral port, a
  per-session token, Host validation, HTTP Range support and CORS for the app
  origin.
- **Metadata** — iTunes, MusicBrainz, LRCLIB and Spotify embed clients, cached in
  SQLite; cover art cached on disk.
- **Playlists** — create, rename, delete, list, add/remove/reorder tracks, plus file
  import and export.
- **Queue and session** — queue state and cached track ids persist in SQLite.
- **Visuals** — an audio-reactive 2D visualizer and aurora backdrop with quality
  tiers and reduced-motion handling.
- **Android target** — a Kotlin yt-dlp plugin and mobile-only resolve commands.

## Stack

- Tauri 2, React 19.1, Vite 8, TypeScript 6 (strict), Tailwind 4, Zustand 5,
  React Router 7.
- Rust edition 2024: `rusqlite` 0.40.2 (bundled), `axum` 0.8.9, `tokio` 1.53.3,
  `reqwest` 0.13.5, `lofty` 0.25.4, `image` 0.25, `uuid` 1.28.
- Release profile: `codegen-units=1`, LTO, `opt-level=3`, `panic=abort`, `strip`.
- Bundle target: NSIS, with `bin/yt-dlp.exe` as a Windows resource.

## Requirements

- Windows 10/11 x64 for the desktop build.
- Node.js and npm (a `package-lock.json` is committed).
- Rust stable with the Tauri 2 prerequisites (a `Cargo.lock` is committed).
- A `yt-dlp` binary: either bundled at `bin/yt-dlp.exe` or pointed to by
  `ADIOS_YTDLP`.

## Build and run

```powershell
npm ci
npm run dev        # Vite dev server
npm run build      # tsc && vite build
npm run tauri      # Tauri dev/build
```

Scripts: `dev`, `build`, `preview`, `tauri`, `typecheck`, `lint` (eslint) and
`test` (vitest).

## Test

```powershell
npm test                 # Vitest
cd src-tauri
cargo test               # Rust tests
```

The repository contains 14 Vitest files (audio retry, batch resolve, colour, LRC
parsing, online play, play gating, playlist save, recommendations, Android resolve,
Spotify API, taste, download store, player store, search ranking) and 178 Rust
`#[test]` / `#[tokio::test]` functions.

**No CI workflows are tracked.** Build, test and lint run locally only; the test
counts above were not re-executed for this documentation.

## Architecture summary

```
WebView (React 19)
  PlayerEngine (two <audio> elements, Web Audio analyser)
        │ invoke (JSON)                 │ HTTP 127.0.0.1:<ephemeral>
        ▼                               ▼
Rust core ── axum loopback audio server (token + Host + Range + CORS)
          ── resolver chain: LocalFile → Cache → yt-dlp → Piped/Invidious
          ── metadata: iTunes, MusicBrainz, LRCLIB, Spotify embed
          ── SQLite (rusqlite, FTS5) + disk cache + cover art cache
```

The loopback server validates its per-session token and the Host header, supports
Range requests, and allows only the app origin via CORS. Local playback works with
no network and no resolver.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the merged architecture
document, and [`docs/DESIGN.md`](docs/DESIGN.md) for the UI design system.

## Data model

SQLite database `adios.db` in the app data directory, with four versioned
migrations (`m001_init`, `m002_lyrics_cache`, `m003_download_pins`, `m004_genre`):

| Table | Role |
| --- | --- |
| `tracks` | Library rows, indexed on artist and album |
| `tracks_fts` | FTS5 virtual table with insert/delete/update triggers |
| `playlists`, `playlist_tracks` | Playlists and ordered membership |
| `queue` | Persisted session queue |
| `cache_index` | Remote stream cache entries |
| `lyrics_cache` | Lyrics hits and misses |
| `meta_cache` | Metadata cache |
| `settings` | JSON settings values |

`m004` adds a nullable `genre` column for taste and mood shelves. Migration state
is tracked in `schema_migrations`. Cover art is cached under `$APPDATA/covers`.

## Configuration

| Name | Purpose |
| --- | --- |
| `ADIOS_YTDLP` | Override the yt-dlp binary path |
| `ADIOS_AUTO_SCAN` | Scan a directory at startup |
| `TAURI_DEV_HOST` | Dev host for the Vite/Tauri dev server |
| `VITE_ADIOS_SMOKE` | Frontend smoke-test flag |
| `VITE_ADIOS_XFADE_TEST` | Frontend crossfade-test flag |

User settings persist in the `settings` table as JSON values (for example
`ytdlpPath`). No `.env` file or credentials are committed.

## Notes and limitations

- CSP is disabled (`csp: null`) while the app loads remote audio and metadata and
  runs a local HTTP server.
- The bundle target is Windows NSIS only, although the codebase also maintains
  mobile commands and a mobile plugin; no mobile build config is committed.
- The Android path depends on an external Kotlin yt-dlp plugin and a bundled
  `bin/yt-dlp.exe`, neither of which can be verified statically.
- `BUILD_PLAN.md` records phase gates and device runs that cannot be re-verified
  from the repository.

## Documentation

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- [`docs/DESIGN.md`](docs/DESIGN.md)
- `BUILD_PLAN.md` — decisions, phases and gate notes.

## License

MIT. See [`LICENSE`](LICENSE).

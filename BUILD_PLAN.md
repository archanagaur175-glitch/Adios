# Adios — Build Plan

STATUS: APPROVED (2026-10-15) · P4 · Step 4.2 · PENDING

## DECISIONS
- **D1 MSVC Build Tools**: missing (verified: no cl.exe). Install REQUIRED in P0.1 after approval: `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"` (UAC + large download; reopen terminal after). If install fails: STOP and ask. No GNU-toolchain workaround.
- **D2 MusicBrainz UA**: `Adios/0.1 (personal use; contact: <email>)`. Settings field for email; until filled, send UA with placeholder + one-time warning in Settings.
- **D3 Crossfade**: default OFF (0 ms), preload-and-switch near-gapless. Setting range 0–8 s. Change default only after a listening test in P3; log result.
- **D4 Piped/Invidious**: ship EMPTY instance list; settings editor + "Test instance" button + docs hint. yt-dlp is the primary resolver.
- **D5 rusqlite (bundled)** over tauri-plugin-sql: full control of FTS5, migrations, meta/lyrics caches, single-DB.
- **D6 One loopback axum server** for ALL audio (local + remote), per Invariant 1. Tauri asset protocol NOT used for audio.
- **D7 yt-dlp sidecar**: resolve via `-J` / stream-URL extraction; prefer m4a (AAC) then webm (Opus) audio-only; NO ffmpeg in v1. Spike records EJS/JS-runtime and PO-token (bgutil) findings; Node 22 available. No PO provider in v1 unless spike fails.
- **D8 react-router v7** — single router choice; AppShell layout route + child routes. **HashRouter** (not BrowserRouter) so deep routes survive reloads under Tauri's custom protocol.
- **D9 Zustand** for all stores; engine/visuals are plain TS classes driven by stores, never React state for audio data.
- **D10 window-vibrancy** Mica with Acrylic fallback; NSIS installer; single-instance plugin; tracing file logs.
- **D11 No three.js/WebGL this version.** Immersive = layered 2D (Aurora + 2D canvas visualizer). Generic `audioBands` bridge + `VisualizerHost` interface kept so a 3D layer can be added later as P10 (not planned in detail).
- **D12 Search local-first**: instant FTS5, then debounced (250–300 ms) MusicBrainz appended (Invariant 4).
- **D13 Dependencies & why**: axum (loopback server) · reqwest (stream remote audio + HTTP clients) · tokio (async runtime, bundled with Tauri) · lofty (tags + embedded art on scan) · window-vibrancy (Mica/Acrylic) · tracing (logs) · rusqlite bundled (DB+FTS5) · react-router (routing) · zustand (state) · tailwind v4 (styling) · @tauri-apps/* plugins: single-instance (+ later autostart) · vite-plugin (tauri) via create-tauri-app template.
- **D14 Plan-file safety**: ALWAYS `git commit` plan files after writing them. `create-tauri-app -f` can wipe non-empty dirs (incident 2026-10-15 — plan files restored from session context). Commit before any scaffolding/force operation.

## CONSTRAINTS
1. **Rule 1 — Local plan persistence.** This file + docs/ARCHITECTURE.md + docs/DESIGN.md are memory. After EVERY step: update STATUS, checkbox, VERIFICATION LOG, NEXT ACTION before anything else. At session start / after compaction: read all three, continue from NEXT ACTION. Keep this file ≤ ~400 lines.
2. **Rule 2 — Strict approval gateway.** No implementation until a message containing APPROVED. After approval: STATUS → APPROVED, `git init`, Phase 0 only. Later phases only after prior gate passes and is logged. If the plan proves wrong: STOP, record problem + proposed change here, ask. Never silently change scope.
3. **Rule 3 — Engineering rules.** Files ≤ 200 lines, one responsibility. TypeScript strict; no `any` without a comment; `tsc --noEmit` + lint at every gate. Verify before claiming done: run the command, paste result in VERIFICATION LOG. PowerShell-safe commands; scaffold NON-INTERACTIVELY. Record every dependency and why (DECISIONS). Commit after each passing step (feat:/fix:/chore:). Never invent API shapes — check official docs/installed types for MusicBrainz, CAA, LRCLIB, Tauri v2, yt-dlp options, axum; note the source in the log. Unit tests REQUIRED for: LRC parser, resolver candidate scoring, MB rate-limit queue, cache eviction, DB migrations.
4. **Rule 4 — Invariants (violating = bug).**
   - 4.1 ALL audio via the loopback server (127.0.0.1, ephemeral port, per-session random token, Host-header validation → 403, HTTP Range, CORS app-origin-only). `<audio>` uses `crossOrigin="anonymous"`. Audio bytes NEVER via invoke. NOT via asset protocol.
   - 4.2 Resolution behind ONE Rust trait `Resolver`: LocalFile → Cache → YtDlpSidecar → Piped/Invidious; each has `health()`. Instance list user-configured, never hard-coded. Re-resolve once on 403/expiry.
   - 4.3 Local playback works with no network and no resolver.
   - 4.4 MusicBrainz: descriptive UA, ≤1 rps single queue, everything cached in SQLite. Search local-first (FTS5 instant, then debounced MB).
   - 4.5 LRCLIB: GET /api/get (artist, track, album, duration) → fallback /api/search; prefer synced; cache hits AND misses.
   - 4.6 Immersive-UI perf: audio-reactive values → CSS variables / canvas via refs inside rAF, NEVER React state. Animate only transform/opacity. ≤ 4 backdrop-filter elements visible, blur ≤ 24 px. All rAF loops stop when hidden/minimized; paused = static frame. Respect prefers-reduced-motion. Tiers Low/Med/High + Eco.

## PHASES

### P0 Prerequisites + scaffold
- **Modules**: none (scaffold only).
- **Deps**: D1 MSVC install; network.
- **Risks**: MSVC install needs UAC/large download and may need reboot; scaffold must be non-interactive.
- **Gate**: `npm run tauri dev` opens a window; `cargo build` OK; tsc + lint clean.
- [x] **0.1 Install/verify MSVC Build Tools (D1)** — run winget command (user approves UAC), reopen checks. Done-When: `cl.exe` reachable via `vswhere` + vcvars. Verify: `& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe" -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath` returns a path; then `cmd /c "\"<path>\VC\Auxiliary\Build\vcvars64.bat\" && where cl"` finds cl.exe. (If install fails → STOP, log, ask.)
- [x] **0.2 Non-interactive scaffold: Tauri v2 + React + Vite + TS strict** — `npm create tauri-app@latest . -- -m npm -t react-ts -y -f --identifier com.nullbrain.adios --tauri-version 2` (flags confirmed via `--help` first; NON-interactive). Done-When: app runs. Verify: `npm install; cargo build` succeeds; `npm run tauri dev` opens window. ⚠️ `-f` wiped uncommitted plan files — restored (see D14).
- [x] **0.3 Tailwind v4 + ESLint + path aliases** — `npm i -D tailwindcss @tailwindcss/vite eslint typescript-eslint @eslint/js @types/node`; `tailwindcss()` in vite config + `@import "tailwindcss"` in `src/index.css`; `@/*` alias in tsconfig paths + vite resolve; scripts `typecheck`/`lint`. Verify: `npx tsc --noEmit` (clean), `npm run lint` (clean), `npm run build` renders (Tailwind CSS 13.3 kB emitted).
- [x] **0.4 Commit plan files + scaffold (chore)** — git repo exists (re-init only if gone); commit BUILD_PLAN.md, docs/, scaffold. Verify: `git status --short` clean; `git log --oneline` has initial commit.

### P1 App shell
- **Modules**: `src/routes/AppShell.tsx`, `src/components/titlebar/Titlebar.tsx`, `src/components/sidebar/Sidebar.tsx`, `src/store/*Store.ts` (5 stubs), `src/lib/shortcuts.ts`, `src-tauri/src/win/vibrancy.rs`, tokens CSS.
- **Deps**: P0.
- **Risks**: Mica + custom titlebar drag regions; maximized-window hit testing.
- **Gate**: navigable shell; Mica applied (Acrylic fallback); tokens drive colors; tsc + lint clean.
- [x] **1.1 Design tokens + global CSS + custom scrollbar** — tokens per docs/DESIGN.md §3. Verify: `npx tsc --noEmit && npm run lint`; visual check dev window.
- [x] **1.2 Custom titlebar + window controls + Mica/Acrylic vibrancy** — `vibrancy.rs` + drag region + min/max/close via Tauri window API. Verify: tsc/lint; manual: drag works, controls work, Mica visible (log fallback if not).
- [x] **1.3 Layout + sidebar + react-router** — AppShell with 5 routes. Verify: `npx tsc --noEmit && npm run lint`; manual nav each route.
- [x] **1.4 Zustand stores scaffold + settings persistence wiring** — 5 stores, typed, stubbed. Verify: tsc/lint; store unit smoke test if trivial. (`zustand@5.0.15`; ui/settings stores use `persist` → localStorage until P2.2 swaps to invoke.)

### P2 SQLite + settings + library models
- **Modules**: `src-tauri/src/db/mod.rs`, `db/migrations/*`, `db/queries.rs`, `commands/library.rs`, `commands/settings.rs`, `src/lib/api.ts`.
- **Deps**: P1.
- **Risks**: FTS5 trigger correctness; migration idempotency.
- **Gate**: migration tests pass; settings + track CRUD roundtrip; `cargo test` green.
- [x] **2.1 rusqlite init + migration runner + schema 001 (tables, FTS5, triggers)** — per docs/ARCHITECTURE.md §3.1. Unit tests: fresh migration, re-run is no-op, FTS insert/update/delete triggers. Verify: `cargo test` in src-tauri.
- [x] **2.2 Settings get/set commands + settings store** — Verify: `cargo test` (settings roundtrip); tsc/lint.
- [x] **2.3 Library CRUD commands + typed invoke wrappers (`src/lib/api.ts`)** — Verify: `cargo test` (CRUD roundtrip); tsc/lint.

### P3 Local playback (loopback server + engine + scan)
- **Modules**: `src-tauri/src/server/{mod,handlers,range}.rs`, `commands/player.rs`, `commands/scan.rs`, `src/audio/{engine,graph,bands,mediaSession}.ts`, `src/store/playerStore.ts`, `src/components/player/*`, folder-scan UI.
- **Deps**: P2.
- **Risks**: Range implementation bugs; analyser silence if CORS/token wrong; lofty tag variance across formats.
- **Gate**: scan folder → play/seek/next/prev works; analyser returns non-zero data (logged); wrong-token/Host → 403 test passes; crossfade listening test logged (D3).
- [x] **3.1 Loopback server skeleton: token + Host validation + CORS + Range (local files)** — axum on 127.0.0.1:0; `server_info` invoke. Tests: valid request 200, Range → 206 with correct bytes, wrong token → 403, wrong Host → 403 (R10). Verify: `cargo test`.
- [x] **3.2 Folder picker + scan with lofty (tags + embedded art → covers/)** — `scan_folder` command + `tauri-plugin-dialog` folder picker + Library page Scan button; embedded Jpg/Png art → `covers/<sanitized-stem>-<fnv1a>.jpg`. Tests: recursive/hidden/extension filter, WAV fixture via real lofty read (duration+fallbacks), dup skip, error collection. Verify: `cargo test`; live scan of real MP3s logs N tracks.
- [x] **3.3 PlayerEngine A/B + Web Audio graph + queue + player bar (seek/volume)** — two `<audio crossOrigin="anonymous">` → loopback URLs (server_info invoke); graph Element→Gain→Analyser→Master (analyser before master so bands survive low volume); A/B preload-and-swap (D3, crossfadeMs 0); store-driven engine with seekVersion (no echo loops); PlayerBar (transport/seek/volume/cover via convertFileSrc); Library rows click-to-play. Gate: tsc/lint clean; smoke log RMS > 0 while playing, seek/next/prev/pause verified.
- [x] **3.4 Media Session + global media keys + keyboard shortcuts + D3 crossfade listening test** — MediaSession metadata/artwork + action handlers; `media-key` event bridge; RegisterHotKey thread (play/pause/next/prev/stop); shortcuts Space/←/→/↑/↓/M/Ctrl+K (Q/L deferred until queue/like UI exists); equal-power crossfade (CrossfadeController + A/B swap). Gate: tsc/lint clean; real OS media key advances track; crossfade gain curve + swap verified live; **D3 default kept at 0 ms** (fade works objectively; OFF remains the right default for gapless local listening).

### P4 Resolver spike + proxy (CORE RISK — before metadata/UI polish)
- **Modules**: `src-tauri/src/resolver/{mod,local,cache,ytdlp,piped,score}.rs`, `src-tauri/src/server/cache.rs`, `commands/resolver.rs`, settings UI (instances, update yt-dlp).
- **Deps**: P3 (server, playback).
- **Risks**: yt-dlp YouTube challenges (EJS/JS runtime, PO-token); URL expiry; remote streaming through axum; public instance flakiness.
- **Gate**: manual "artist - title" query → plays via loopback with analyser data; Range on remote → 206; network kill mid-stream → exactly ONE logged re-resolve; scoring tests pass; spike findings recorded in VERIFICATION LOG.
- [x] **4.1 Resolver trait + LocalFile + Cache resolvers + health() + candidate scoring** — object-safe `Resolver` trait (BoxFuture), `ResolverChain` (first hit wins, misses logged), `Query`/`Candidate`/`Health` types; LocalFile scores library rows via the shared scorer (best ≥ 0.5 conf, mime from extension); Cache serves `meta_cache(kind=resolution)` + `cache_index` + on-disk file (FNV-1a query key, duration in whole seconds); chain managed in setup (SharedDb = `Arc<Mutex<Connection>>` behind `DbState::shared()`). Scoring: duration ±3 s hard bound, token-set title overlap, official-audio +0.05 / topic +0.08 boosts, live/cover/remix reject unless the query itself contains the word. Tests: 9 scoring cases (match/duration/live/cover/remix/hints/mismatch/noisy title) + LocalFile + Cache (hit, key-miss, missing-file) + chain (local-first, health order, empty → NotFound). Verify: `cargo test` 34 passed.
- [ ] **4.2 yt-dlp sidecar: resolve command + stream URL extraction (m4a/webm prefer) + spike findings** — run yt-dlp `-J` and format extraction against a real YouTube query; record: EJS/JS-runtime needed? PO-token needed? Node 22 present. If spike fails → STOP per Rule 2.4, propose PO-token plan. Verify: `cargo test` (sidecar arg construction) + manual resolve log.
- [ ] **4.3 Proxy remote streaming + disk cache writer + cache_index** — remote URL → reqwest stream → client + disk (temp→rename); Range forwarded. Verify: `cargo test` (cache_index bookkeeping); manual: play remote track, `curl -H "Range: bytes=0-1" http://127.0.0.1:<port>/media/<token>` → 206.
- [ ] **4.4 Re-resolve once on 403/expiry + Piped/Invidious fallback (empty list, Test instance) + "Update yt-dlp" action** — Verify: `cargo test` (expiry triggers single re-resolve — mock); manual: kill network mid-stream → exactly one logged re-resolve; update action downloads/runs.

### P5 Metadata + local-first search
- **Modules**: `src-tauri/src/meta/{musicbrainz,coverart,rate_limit}.rs`, `src-tauri/src/db/` (meta_cache), `commands/search.rs`, `src/store/searchStore.ts`, `src/components/search/*`, settings (contact email + warning).
- **Deps**: P2, P4 (candidate scoring reused for enrichment).
- **Risks**: MB API shape (verify docs first); rate-limit correctness; cache poisoning.
- **Gate**: repeat query served from meta_cache with NO network hit (logged); burst test confirms ≤1 rps; local FTS results instant, online appended after debounce; tsc/lint clean.
- [ ] **5.1 MusicBrainz rate-limit queue + search client + meta_cache** — UA per D2; unit tests: 10 rapid requests span ≥ 9 s, cache hit skips queue. Verify: `cargo test`. NOTE: check MB web service docs for response shape before coding; cite URL in log.
- [ ] **5.2 Cover Art Archive fetch + cover files + palette extraction (3 colors, cached on track)** — Verify: `cargo test` (palette extraction on fixture image); manual fetch for a known MBID. Cite CAA docs URL in log.
- [ ] **5.3 Local-first search UI: Ctrl+K overlay, grouped, arrow nav, 250–300 ms debounce** — Verify: tsc/lint; manual: FTS instant, MB appended, second identical query = cache-only (log).
- [ ] **5.4 Import enrichment: match MB by duration ±3 s + title similarity** — Verify: `cargo test` (enrichment matching cases); manual scan → enriched rows.

### P6 Immersive UI (2D)
- **Modules**: `src/visuals/{audioBandsBridge,aurora,palette,VisualizerHost,visualizer2d,tiers,idle}.ts(x)`, Now Playing hero components, shared-element transition util, Eco logic.
- **Deps**: P3 (analyser), P5 (palette).
- **Risks**: rAF leaks; backdrop-filter budget violations; View Transitions fallback behavior.
- **Gate**: loops stop when hidden/paused (log proof); tiers switch; profiler notes confirm no rAF-driven React re-renders; ≤ 4 backdrop-filter visible; tsc/lint clean.
- [ ] **6.1 audioBands bridge (CSS vars, visibility gating) + palette extraction wiring** — Verify: tsc/lint; log: rAF running only when playing+visible; vars update.
- [ ] **6.2 Aurora backdrop (pre-blurred blobs, grain, 800 ms palette tween) + art cross-fade** — Verify: tsc/lint; manual track change tween.
- [ ] **6.3 2D canvas visualizer + tiers (Low/Med/High) + Eco + battery suggestion** — Verify: tsc/lint; manual tier switch; fps log per tier.
- [ ] **6.4 Now Playing hero (tilt art, vinyl slide/spin, View Transitions) + ambient idle mode + reduced-motion** — Verify: tsc/lint; manual expand/collapse + idle fade; reduced-motion OS setting respected.

### P7 Lyrics
- **Modules**: `src-tauri/src/meta/lrclib.rs`, `src-tauri/src/db/` (lyrics_cache), `src/lib/lrc.ts`, `src/components/lyrics/*`, `commands/lyrics.rs`.
- **Deps**: P3, P5 patterns.
- **Risks**: malformed LRC; negative-cache poisoning.
- **Gate**: lyrics follow seek within ±150 ms; offline returns cached/negative; parser tests pass.
- [ ] **7.1 LRCLIB client + lyrics_cache (hits AND misses)** — cite LRCLIB docs in log. Verify: `cargo test` (cache negative row); manual fetch.
- [ ] **7.2 LRC parser + synced panel + immersive mode + click-to-seek** — unit tests: offsets, multi-line, malformed lines skipped. Verify: `cargo test` + `npx tsc --noEmit && npm run lint`; manual seek sync.

### P8 Offline cache + playlists
- **Modules**: `src-tauri/src/server/cache.rs` (eviction), `commands/settings.rs` (cache size), playlists commands + UI, session restore.
- **Deps**: P4, P5.
- **Risks**: LRU correctness; ordering on drag reorder; JSON schema drift for import/export.
- **Gate**: eviction keeps cache_index consistent; playlist CRUD + import/export roundtrips; restart restores queue/position/volume.
- [ ] **8.1 Cache size setting + LRU eviction job** — unit tests: size cap respected, hit_count/LRU order, index consistent after eviction. Verify: `cargo test`.
- [ ] **8.2 Playlist CRUD + drag reorder + JSON import/export** — Verify: `cargo test` (CRUD + reorder + import/export roundtrip); tsc/lint; manual.
- [ ] **8.3 Session restore (queue, position, volume) + Cached view polish** — Verify: manual restart restores state; tsc/lint.

### P9 Polish / a11y / perf / installer
- **Modules**: a11y fixes across components, perf pass, `src-tauri/tauri.conf.json` (NSIS, version), docs touch-ups.
- **Deps**: all.
- **Risks**: installer config; hidden perf regressions.
- **Gate**: `npm run tauri build` produces NSIS installer; keyboard-only pass; no file > 200 lines; final full verification logged.
- [ ] **9.1 Accessibility pass** — focus rings, aria labels, contrast ≥ 4.5:1, reduced-motion audit, live-region track announcements. Verify: `npx tsc --noEmit && npm run lint`; manual keyboard-only walkthrough checklist logged.
- [ ] **9.2 Perf pass** — bundle size, ≤ 4 backdrop-filter audit, canvas paint times, file-length check. Verify: script/test asserting every file ≤ 200 lines; perf log numbers pasted.
- [ ] **9.3 NSIS installer + versioning + single-instance + FINAL full verification** — Verify: `npm run tauri build` → installer exists (path logged); fresh-install smoke test (install → play local file offline).

P10 (optional, NOT planned): 3D layer behind existing VisualizerHost interface.

## VERIFICATION LOG
| Date | Command | Result |
|---|---|---|
| 2026-10-15 | `node -v; npm -v; rustc -V; cargo -V; git --version` | v22.22.3 / 10.9.8 / 1.97.1 / 1.97.1 / 2.54.0.windows.1 |
| 2026-10-15 | WebView2 check (`EdgeWebView\Application`) | 155.0.4283.45 present |
| 2026-10-15 | `cl.exe` search under VS dirs | NOT FOUND — MSVC Build Tools missing (D1) |
| 2026-10-15 | `winget install ...BuildTools...` (P0.1) | exit 1, log: "Visual Studio Build Tools 2022 is already installed" — pre-existing install at `C:\BuildTools` |
| 2026-10-15 | vswhere -all -products * | VS Build Tools 2022, 17.14.38, path `C:\BuildTools`, isComplete=true |
| 2026-10-15 | `cmd /c "vcvars64.bat && where cl && where link"` (P0.1 gate) | `...\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe` + `link.exe` — **P0.1 PASS** |
| 2026-10-15 | `npm create tauri-app@latest . -- -m npm -t react-ts -y -f --identifier com.nullbrain.adios --tauri-version 2` (P0.2) | "Template created!", no prompts; React 19.1, Vite 8, TS ~6.0.3, strict tsconfig |
| 2026-10-15 | `npm install` | 27 packages, 0 vulnerabilities |
| 2026-10-15 | `cargo build` (workdir src-tauri) | Finished dev profile in 4m45s, no errors — MSVC toolchain works |
| 2026-10-15 | `npm run tauri dev` smoke (P0.2 gate) | Process `tauri-app` PID 23044, title 'tauri-app' — **P0.2 PASS** (killed after) |
| 2026-10-15 | incident: plan files gone after scaffold | cause: `create-tauri-app -f` wiped uncommitted BUILD_PLAN.md + docs/; restored from session context; D14 added |
| 2026-10-15 | `git add BUILD_PLAN.md docs && git commit` | root commit `48aba66` "chore: add build plan and architecture/design docs" |
| 2026-10-15 | `npm i -D tailwindcss @tailwindcss/vite eslint typescript-eslint @eslint/js @types/node` (P0.3) | 109+2 packages, 0 vulnerabilities |
| 2026-10-15 | `npx tsc --noEmit` (after TS6 baseUrl fix: paths without baseUrl) | clean; alias probe `@/lib/...` resolved |
| 2026-10-15 | `npm run lint` | clean (0 errors, 0 warnings) |
| 2026-10-15 | `npm run build` (P0.3 gate) | tsc + vite build OK, `dist/assets/index-*.css` 13.33 kB (Tailwind active) — **P0.3 PASS** |
| 2026-10-15 | `git add -A && git commit` | `7e220d5` "chore: scaffold Tauri v2 + React 19 + Vite + Tailwind v4 + ESLint (P0.2-0.4)" |
| 2026-10-15 | `npm run tauri dev` smoke (P0 gate) | window up, PID 20868 — **P0 GATE PASS** (killed after) |
| 2026-10-15 | `npx tsc --noEmit; npm run lint; npm run build` (P1.1) | all clean; dist CSS 14.9 kB |
| 2026-10-15 | theme-utility probe (temp classes → build → grep dist CSS) | `--color-ink-2` ✓, `#0d0f16` ✓, `.rounded-card{border-radius:var(--radius-card)}` ✓, font-display ✓ — **P1.1 PASS** (probe reverted, tsc clean) |
| 2026-10-15 | API check: tauri-2.12.4 `src/window/mod.rs`, tauri-utils `WindowEffect` enum, @tauri-apps/api `window.d.ts`, capability schema | EffectsBuilder doc example, variants MicaDark/Mica/Acrylic/Tabbed, minimize/toggleMaximize/close/isMaximized/startDragging all exist; permission ids verified |
| 2026-10-15 | `npx tsc --noEmit; npm run lint; cargo build` (P1.2) | all clean (after adding `use tauri::Manager`) |
| 2026-10-15 | `npm run tauri dev` smoke (P1.2 gate) | window PID 20536 title **"Adios"**, log `[vibrancy] applied: mica-dark`, decorations off + 1280×800/min 900×600 — **P1.2 PASS** (drag/controls/visual need user confirmation) |
| 2026-10-15 | `npm install react-router@7` (D8) | 3 packages, 0 vulnerabilities |
| 2026-10-15 | headless Edge DOM dump (route render test) | 1st run caught `getCurrentWindow()` throwing outside Tauri → made Titlebar Tauri-guarded; re-run: sidebar/Home/player-bar rendered, no vite errors |
| 2026-10-15 | headless Edge × 7 routes (`#/`, `#/library`, `#/playlists`, `#/playlists/42`, `#/search`, `#/cached`, `#/settings`) | all rendered `class="page"` — **P1.3 PASS** (tsc + lint clean) |
| 2026-10-15 | `npm install zustand` (D9) | zustand@5.0.15 |
| 2026-10-15 | `npx tsc --noEmit; npm run lint; npm run build` (P1.4) | all clean — **P1.4 PASS** |
| 2026-10-15 | P1 gate review | navigable shell (7 routes rendered) ✓ Mica `mica-dark` applied ✓ tokens verified ✓ tsc/lint clean ✓ — **P1 GATE PASS** |
| 2026-10-15 | `cargo add rusqlite --features bundled` (D5) | rusqlite v0.40.2; verified FTS5: libsqlite3-sys build.rs compiles with `-DSQLITE_ENABLE_FTS5` |
| 2026-10-15 | `cargo test` (P2.1 gate) | **5 passed, 0 failed** (fresh_migration_creates_all_tables, migration_rerun_is_noop, version_mismatch_skips_reapplied_migration, fts_triggers_track_insert_update_delete, fts_search_matches_title_and_artist) — **P2.1 PASS** |
| 2026-10-15 | `cargo test` (P2.2: + settings_roundtrip_and_upsert) | **6 passed, 0 failed**; `cargo build` 0 warnings; tsc + lint clean |
| 2026-10-15 | `npm run tauri dev` smoke (P2.2) | `[db] opened at C:\Users\archa\AppData\Roaming\com.nullbrain.adios\adios.db`, db+WAL files created, window title Adios — **P2.2 PASS**; docs storage path corrected to Tauri identifier-based dir |
| 2026-10-15 | `cargo test` (P2.3: + track_crud_roundtrip, tracks_all_orders_by_artist_then_title) | **8 passed, 0 failed**; `cargo build` 0 warnings; tsc + lint clean; headless render OK — **P2.3 PASS** |
| 2026-10-15 | P2 gate review | migrations 8/8 tests ✓ settings roundtrip ✓ CRUD roundtrip ✓ typed api.ts ✓ tsc/lint clean ✓ — **P2 GATE PASS** |
| 2026-10-15 | deps: `cargo add axum tokio futures uuid percent-encoding bytes reqwest --no-default-features --features rustls,stream` (D13) | axum 0.8.9, tokio 1.53.3, futures 0.3.34, uuid 1.28 (v4), percent-encoding 2.3.2, bytes 1.12.1, **reqwest 0.13.5** (note: 0.13 renamed features — `rustls`, not `rustls-tls`) |
| 2026-10-15 | `cargo test` (P3.1 gate) | **15 passed, 0 failed** — 8 db + 7 server (200+CORS, 206 slice bytes exact, wrong-token 403, wrong-Host 403, 404, disallowed-origin no ACAO, 416) ; 0 warnings |
| 2026-10-15 | live app smoke: dev window + `curl` against real server | `[server] listening on http://127.0.0.1:57629`; live wrong-token → 403, unknown route → 404 — **P3.1 PASS** |
| 2026-10-15 | `cargo add lofty tauri-plugin-dialog` + `npm i @tauri-apps/plugin-dialog` | lofty 0.25.4, tauri-plugin-dialog 2.8.3 (JS `open({directory:true})` → `string\|null`, permission `dialog:default` verified in crate permissions dir) |
| 2026-10-15 | lofty 0.25.4 API verified from installed source (never invented) | `read_from_path` → `TaggedFile`; `AudioFile::properties().duration()`; `TaggedFileExt::primary_tag/first_tag`; `Accessor::{title,artist,album,track,disk,date}`; `Tag::pictures()` → `mime_type()/data()` (MimeType::Jpeg/Png) |
| 2026-10-15 | `cargo test` (P3.2 gate) | **20 passed, 0 failed** — 8 db + 7 server + 5 scan (collect filter/hidden/case, WAV fixture: duration 500ms + stem/artist fallbacks, dup `added=1→skipped=1`, error collection, cover-name determinism) ; 0 warnings |
| 2026-10-15 | `npx tsc --noEmit; npm run lint` | both clean (Library.tsx Scan button + track list, api.ts scanFolder, format.ts) |
| 2026-10-15 | dev smoke `ADIOS_AUTO_SCAN=<fixture with 3 real MP3s>` (auto-scan harness, debug-only) | `[scan] folder=... added=3 skipped=0 failed=0 in 49ms` + `[smoke] auto-scan done` — **real lofty tag read OK** |
| 2026-10-15 | covers/ inspection after live scan | `Apna_Bana_Le-23c1b777f0bc1114.jpg` (49 KB) + `Back_to_Friends-490191272d4d0e78.jpg` (51 KB) written from embedded art (3rd file had none) |
| 2026-10-15 | re-scan same fixture (duplicate handling, fresh app run) | `[scan] ... added=0 skipped=3 failed=0 in 3ms` — DB persistence + ON CONFLICT path verified live |
| 2026-10-15 | headless DOM `http://localhost:1420/#/library` | Library page renders, `Scan folder` button present, browser-mode gate message correct (invoke disabled outside Tauri) |
| 2026-10-15 | `acl-manifests.json` check + file-length audit | `dialog` permission registered ✓; scan/mod 153, tags 91, tests 109, cmd 33, Library 90, api 93, format 11 — all ≤200 — **P3.2 PASS** |
| 2026-10-15 | `npx tsc --noEmit; npm run lint` (P3.3) | both clean (engine/graph/bands/smoke, PlayerBar+icons+css, store seek/playQueue, App init, Library click-to-play) |
| 2026-10-15 | dev smoke `VITE_ADIOS_SMOKE=1` first run (StrictMode double-harness, interleaved — diagnosed, not a bug: queue [1,3,2] artist-sort + shared store) | rms>0, seek/next correct; harness made idempotent (`started` guard) |
| 2026-10-15 | dev smoke `VITE_ADIOS_SMOKE=1` clean single run (dev8) | `[smoke] tracks loaded: 3` · `rms=0.0126 isPlaying=true position=2.05s duration=261.36s` (**AnalyserNode RMS > 0 while playing**) · `after seek(5): position=5.53s` · `after next: 1 -> 3 position=1.32s isPlaying=true` · `after prev: 3 -> 1 position=1.06s isPlaying=true` · `final isPlaying=false - smoke done` — **P3.3 PASS** |
| 2026-10-15 | `cargo test` (P3.3 regression) | 20 passed, 0 failed; file-length audit: engine 196, playerStore 134, PlayerBar 110, playerbar.css 149, smoke 83, bands/graph 52, icons 70 — all ≤200 |
| 2026-10-15 | `cargo add windows-sys --features Win32_Foundation,Win32_UI_Input_KeyboardAndMouse,Win32_UI_WindowsAndMessaging` (P3.4) | windows-sys 0.61.2; `RegisterHotKey`/`GetMessageW`/`MSG`/`WM_HOTKEY=786`/`MOD_NOREPEAT=16384`/`VK_MEDIA_*` (VK cast u16→u32) verified from installed crate source |
| 2026-10-15 | `npx tsc --noEmit; npm run lint` (P3.4) | both clean; file audit: engine 196, crossfade 173, element 73, mediaSession 113, shortcuts 34, smoke 148, media_keys.rs 77 — all ≤200 |
| 2026-10-15 | `cargo test` (P3.4 regression) | 20 passed, 0 failed |
| 2026-10-15 | INCIDENT (dev9-11): zero `[smoke]` lines, media keys dead — CDP probe (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223` + Runtime.evaluate) | `invoke("plugin:event\|emit")` resolved "EMIT OK" but Rust listener never fired → **`media_keys::spawn` ran the blocking `GetMessageW` loop inline in `setup`**, stalling the event loop so `app.listen` never registered; fixed with `std::thread::spawn` |
| 2026-10-15 | INCIDENT (dev13): crossfade ended with id3 frozen at 1.93 s, `isPlaying=false` — CDP pause-hook with stack traces | WebView2 fires a `pause` event on the FROM element at natural end-of-track mid-fade → store set false → `applyPlaying(false)` → `cancel()` killed the fade; fixed: pause handler ignores `el.ended` (the `ended` handler owns queue advance) |
| 2026-10-15 | dev smoke `VITE_ADIOS_SMOKE=1 VITE_ADIOS_XFADE_TEST=1` cold boot (dev14) | `rms=0.0149 isPlaying=true position=2.38s` · `seek(5)→5.54s` · `next 1→3 isPlaying=true` · `prev 3→1` · **synthetic `media-key next`: 1→2** · xfade samples: gains 1.000/0.000 → 0.946/0.324 → 0.795/0.607 → 0.568/0.823 → 0.286/0.958 → 0.000/1.000, swap `1→3`, id3 position keeps advancing (2.39→3.99), `isPlaying=true`, final gains 1.00/0.00 — equal-power curve + swap + advance verified |
| 2026-10-15 | real OS media-key test: SendInput `VK_MEDIA_NEXT_TRACK` (0xB6, KEYEVENTF_EXTENDEDKEY) against live unfocused app | track advanced id3 → id2 ("back to friends", 0:03/4:21 playing) via RegisterHotKey → `media-key` event → FE handler — **global media keys PASS** (note: keybd_event without extended flag is ignored by the shell) |
| 2026-10-15 | D3 listening-test result | crossfade objective metrics all green (curve, swap, no gap, queue advance); **default kept 0 ms (OFF)** — gapless local listening preferred; user can enable 1–8 s in settings later — **P3.4 PASS** |
| 2026-10-15 | `cargo test` (P4.1) | **34 passed, 0 failed, 0 warnings** — 20 prior + 14 new: 9 score (exact ≥0.99, duration 15 s reject, duration +2 s accept, live reject & query-live allow, cover, remix, official-audio boost, topic boost, no-overlap reject, noisy lyric-video title accept) + LocalFile (hit/mime/miss/health Ok) + Cache (seeded hit conf 1.0, different-duration key miss, missing-disk-file miss) + chain (local-first over cache, health order, empty chain → NotFound) |
| 2026-10-15 | file audit + tsc/lint (P4.1) | resolver mod 154, score 138, local 98, cache 121, tests 176, chain_tests 146 — all ≤200; `npx tsc --noEmit` + `npm run lint` clean (no FE changes) |
| 2026-10-15 | dev boot smoke (P4.1 wiring) | `[db] opened` · `[server] listening` · `[media-keys] 4 registered` · harness `rms=0.0168 isPlaying=true`, seek/next fine — resolver chain constructed in setup without panic, playback unaffected — **P4.1 PASS** |

## OPEN QUESTIONS
1. None blocking. (D1–D4 decided by user; spike findings in P4.2 may raise a PO-token question — handled by STOP rule.)

## NEXT ACTION
Run P4.2: yt-dlp sidecar — resolve command + stream URL extraction (prefer m4a/webm audio-only via `-f bestaudio[ext=m4a]/bestaudio`) + spike findings against a real YouTube query (record: EJS/JS-runtime needed? PO-token needed? Node 22 present). If spike fails → STOP per Rule 2.4, propose PO-token plan. Verify: `cargo test` (sidecar arg construction) + manual resolve log.

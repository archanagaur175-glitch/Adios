# Adios — Build Plan

STATUS: APPROVED (2026-10-15) · P1 · Step 1.4 · IN PROGRESS

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
- [ ] **1.4 Zustand stores scaffold + settings persistence wiring** — 5 stores, typed, stubbed. Verify: tsc/lint; store unit smoke test if trivial.

### P2 SQLite + settings + library models
- **Modules**: `src-tauri/src/db/mod.rs`, `db/migrations/*`, `db/queries.rs`, `commands/library.rs`, `commands/settings.rs`, `src/lib/api.ts`.
- **Deps**: P1.
- **Risks**: FTS5 trigger correctness; migration idempotency.
- **Gate**: migration tests pass; settings + track CRUD roundtrip; `cargo test` green.
- [ ] **2.1 rusqlite init + migration runner + schema 001 (tables, FTS5, triggers)** — per docs/ARCHITECTURE.md §3.1. Unit tests: fresh migration, re-run is no-op, FTS insert/update/delete triggers. Verify: `cargo test` in src-tauri.
- [ ] **2.2 Settings get/set commands + settings store** — Verify: `cargo test` (settings roundtrip); tsc/lint.
- [ ] **2.3 Library CRUD commands + typed invoke wrappers (`src/lib/api.ts`)** — Verify: `cargo test` (CRUD roundtrip); tsc/lint.

### P3 Local playback (loopback server + engine + scan)
- **Modules**: `src-tauri/src/server/{mod,handlers,range}.rs`, `commands/player.rs`, `commands/scan.rs`, `src/audio/{engine,graph,bands,mediaSession}.ts`, `src/store/playerStore.ts`, `src/components/player/*`, folder-scan UI.
- **Deps**: P2.
- **Risks**: Range implementation bugs; analyser silence if CORS/token wrong; lofty tag variance across formats.
- **Gate**: scan folder → play/seek/next/prev works; analyser returns non-zero data (logged); wrong-token/Host → 403 test passes; crossfade listening test logged (D3).
- [ ] **3.1 Loopback server skeleton: token + Host validation + CORS + Range (local files)** — axum on 127.0.0.1:0; `server_info` invoke. Tests: valid request 200, Range → 206 with correct bytes, wrong token → 403, wrong Host → 403 (R10). Verify: `cargo test`.
- [ ] **3.2 Folder picker + scan with lofty (tags + embedded art → covers/)** — Verify: `cargo test` (scan fixture with sample file if available, else mock lofty read); manual scan of a real folder logs N tracks.
- [ ] **3.3 PlayerEngine A/B + Web Audio graph + queue + player bar (seek/volume)** — `<audio crossOrigin="anonymous">` → loopback URLs. Verify: tsc/lint; manual play/seek/next/prev; log AnalyserNode RMS > 0 while playing.
- [ ] **3.4 Media Session + global media keys + keyboard shortcuts + D3 crossfade listening test** — Verify: tsc/lint; manual media-key test; log listening-test result and final crossfade default.

### P4 Resolver spike + proxy (CORE RISK — before metadata/UI polish)
- **Modules**: `src-tauri/src/resolver/{mod,local,cache,ytdlp,piped,score}.rs`, `src-tauri/src/server/cache.rs`, `commands/resolver.rs`, settings UI (instances, update yt-dlp).
- **Deps**: P3 (server, playback).
- **Risks**: yt-dlp YouTube challenges (EJS/JS runtime, PO-token); URL expiry; remote streaming through axum; public instance flakiness.
- **Gate**: manual "artist - title" query → plays via loopback with analyser data; Range on remote → 206; network kill mid-stream → exactly ONE logged re-resolve; scoring tests pass; spike findings recorded in VERIFICATION LOG.
- [ ] **4.1 Resolver trait + LocalFile + Cache resolvers + health() + candidate scoring** — scoring: duration ±3 s hard bound, token-set title similarity, official-audio/topic hints, reject live/cover/remix. Unit tests: scoring cases (match, duration reject, live reject, hint boost). Verify: `cargo test`.
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

## OPEN QUESTIONS
1. None blocking. (D1–D4 decided by user; spike findings in P4.2 may raise a PO-token question — handled by STOP rule.)

## NEXT ACTION
Run P1.4: scaffold the five typed Zustand stores + settings persistence wiring; verify tsc/lint.

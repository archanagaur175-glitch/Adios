# Adios — Design System & UI Spec

Windows 11 native, dark, layered 2D immersive UI. No WebGL in this version.

## 1. Principles
1. **Depth over decoration** — three visual layers: World (Aurora backdrop + visualizer) → Glass (panels) → Content (type/controls). Content never sits directly on moving blobs; glass always mediates.
2. **Music is the light source** — accent colors, glow, and motion intensity derive from the album art and the audio signal. The UI is calm when silent.
3. **Calm by default, alive on beat** — resting state is slow and quiet; the spectrum only nudges scale/brightness, never shakes layout.
4. **Windows 11 native** — Mica/Acrylic, Segoe UI Variable, Fluent focus rings, 8 px grid feel.

## 2. Layer stack (back → front)
| Layer | Content | Notes |
|---|---|---|
| L0 Aurora | 3–4 large radial-gradient blobs from the album palette; slow drift via `transform`; scale/brightness nudged by `--bass`; static film-grain overlay ~4–6 % opacity | Blobs sit on PRE-blurred layers — no live `filter: blur()` on animated elements. Palette tween 800 ms on track change. |
| L1 Visualizer | ONE 2D canvas: radial ring + soft waveform/bars, additive `globalCompositeOperation: 'lighter'` glow, DPR ≤ 1.5 | Drawn only while playing and visible. Tiers: Low 48 bars/30 fps · Med 96/60 · High 128/60 + extra glow pass. Off in Eco. |
| L2 Glass | Panels, cards, player bar, sidebar, dialogs | See tokens. Max 4 visible `backdrop-filter` elements, blur ≤ 24 px. |
| L3 Hero | Now Playing: large album art, CSS 3D tilt, vinyl sliding from sleeve, accent glow pulsing with `--bass` | Shared-element expand via View Transitions API (WebView2 = Chromium); fallback plain fade. |

## 3. Tokens

### Color
| Token | Value |
|---|---|
| `--bg-0` | `#07080C` |
| `--bg-1` | `#0D0F16` |
| `--glass` | `rgba(22,25,36,.55)` + `backdrop-filter: blur(≤24px) saturate(140%)` |
| `--glass-strong` | `rgba(22,25,36,.78)` |
| `--stroke` | `rgba(255,255,255,.08)`; top edge highlight `rgba(255,255,255,.16)` |
| `--text-1` | `#F4F6FB` |
| `--text-2` | `#A5ACBD` |
| `--text-3` | `#6B7287` |
| `--danger` | `#FF5C7A` |
| `--accent` | dynamic from album art — OKLCH L 55–70 %, C ≥ .12; fallback `#7C5CFF` |
| `--accent-2` | dynamic secondary; fallback `#2DE2E6` |

Contrast rule: text on glass must be ≥ 4.5:1; if the dynamic accent fails contrast, fall back to `--text-1`.

### Type
- UI: **Segoe UI Variable** (fallback Inter).
- Display (titles, hero): **Inter Tight** or **Space Grotesk**, 600–700, tracking `-0.02em`.
- Scale: 12 / 14 / 16 / 20 / 28 / 44 / 72 px.
- Time/numerics: tabular numerals (`font-variant-numeric: tabular-nums`).

### Shape & spacing
- Radius: 8 (controls) / 12 (cards) / 20 (panels) / 999 (pills).
- Spacing: 4 pt grid (4/8/12/16/24/32/48).
- Elevation: glass + blur + `box-shadow: 0 20px 60px -20px rgba(0,0,0,.6)` + 1 px stroke with brighter top edge.
- Scrollbar: 8 px, transparent track, thumb `rgba(255,255,255,.18)`, accent-tinted while dragging.

### Motion
- Durations: fast 120 ms · base 220 ms · slow 500 ms (palette tween 800 ms).
- Easing: `cubic-bezier(.2,.8,.2,1)`.
- Springs: stiffness 260 / damping 26 (tilts, shared-element).
- Audio-reactive: asymmetric smoothing attack 0.35 / release 0.08.
- All of the above reduced/flattened under `prefers-reduced-motion`.

## 4. Layout
Target 1280×800, minimum 900×600.

- **Titlebar** (32 px): custom, Mica region, drag area, app icon, window controls (min/max/restore/close).
- **Sidebar** (collapsible, default 220 px): Home · Search · Library · Playlists · Cached. Active item gets a subtle accent pill.
- **Main area**: glass cards floating over the Aurora; content scrolls under a sticky player bar.
- **Player bar** (glass-strong, ~72 px): art thumb · title/artist · prev/play/next · seek bar · volume · lyrics toggle · queue toggle · expand.
- **Expanded Now Playing** (shared-element from bar): hero art left/center (L3), synced lyrics column, queue panel right. Full-bleed Aurora + visualizer behind.
- **Search**: Ctrl+K opens a command-palette overlay; grouped results (Tracks / Albums / Artists); arrow-key navigation; cards with pointer tilt.
- **Settings**: sections for Playback (crossfade), Resolver (order, instances, Test instance, Update yt-dlp), Cache (size), Metadata (contact email), Appearance (tier, reduced motion).

## 5. Components
- **Card**: glass, radius 12, pointer tilt ±6° with moving glare gradient, spring easing; focus ring Fluent accent 2 px.
- **Play button**: SVG play↔pause morph, brief accent bloom pulse on toggle.
- **Seek bar**: 4 px → 8 px on hover, buffered segment (text-3 @ 40 %), time bubble with tabular numerals.
- **Volume**: wheel over the control → exponential curve + transient HUD.
- **Lyrics**: active line scale 1.06; other lines blur up to 3 px by distance from active; click line to seek; 400 ms scroll easing. Immersive mode = full-screen large type, chrome hidden.
- **Source chip**: Resolving (shimmer skeleton) / Cached / Local / Unresolved (danger).
- **Toast**: bottom-right, glass-strong, auto-dismiss.
- **Skeleton**: shimmer sweep, used while resolving streams or fetching metadata.

## 6. Ambient idle mode
After 15 s without pointer/keyboard input while Now Playing is expanded: chrome (buttons, queue, lyrics text) fades to 0 opacity, Aurora + visualizer scale up ~8 % (screensaver feel). Any input restores instantly (220 ms).

## 7. Quality tiers & Eco
| Tier | Bars | fps | Backdrop-filter | Aurora | Visualizer glow |
|---|---|---|---|---|---|
| Low | 48 | 30 | ≤ 2 | drifting | off |
| Med | 96 | 60 | ≤ 4 | drifting | soft |
| High | 128 | 60 | ≤ 4 | drifting | full |
| Eco | off | 30 cap | none (solid translucent panels) | static | off |

Auto-suggest Eco on battery saver; user can pin any tier. Persisted in settings. `prefers-reduced-motion` forces static aurora + no tilt/spin regardless of tier.

## 8. Micro-interaction inventory
- Card tilt + glare (pointer-tracked, spring).
- Play↔pause morph + accent pulse (visualizer glow briefly boosted).
- Seek bar hover growth + buffered + bubble.
- Wheel volume + HUD.
- Track change: art cross-fade 500 ms, palette tween 800 ms, source chip updates.
- Skeleton shimmer while resolving.
- Fluent 2 px accent focus rings everywhere; full keyboard operability.

## 9. Shortcuts
| Key | Action |
|---|---|
| Space | play / pause |
| ← / → | seek ±5 s |
| ↑ / ↓ | volume ±5 % |
| Ctrl+K | search overlay |
| L | toggle lyrics (immersive with double-tap / modifier as needed) |
| Q | toggle queue |
| M | mute |

## 10. Accessibility
- WCAG AA contrast (4.5:1) on all text over glass; dynamic accents never used as sole indicator.
- Every interactive element keyboard-reachable with visible Fluent focus ring.
- `prefers-reduced-motion` honored (see §7).
- ARIA labels on icon-only controls (play, seek, volume, queue, lyrics).
- Screen-reader announcements for track changes (polite live region).

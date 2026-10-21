import { useEffect } from "react";
import { HashRouter, Route, Routes } from "react-router";
import { initMediaIntegration } from "@/audio/mediaSession";
import { initPlayerEngine } from "@/audio/engine";
import { runSmokeIfEnabled } from "@/audio/smoke";
import { ToastHost } from "@/components/common/ToastHost";
import { DownloadPanel } from "@/components/common/DownloadPanel";
import { Announcer } from "@/components/common/Announcer";
import { ImmersiveLyrics } from "@/components/lyrics/ImmersiveLyrics";
import { inTauri, listTracks } from "@/lib/api";
import { attachSessionPersist, restoreSession } from "@/lib/session";
import { initPlayHistory } from "@/lib/playHistory";
import { initYtdlpPathSync } from "@/lib/ytdlpSync";
import { useLibraryStore } from "@/store/libraryStore";
import { initBandsBridge } from "@/visuals/audioBandsBridge";
import { initBatterySuggest } from "@/visuals/batterySuggest";
import { initPaletteWiring } from "@/visuals/palette";
import { AppShell } from "@/routes/AppShell";
import HomePage from "@/routes/Home";
import LibraryPage from "@/routes/Library";
import PlaylistsPage from "@/routes/Playlists";
import PlaylistDetailPage from "@/routes/PlaylistDetail";
import MoodPage from "@/routes/MoodView";
import SearchPage from "@/routes/Search";
import CachedPage from "@/routes/Cached";
import SettingsPage from "@/routes/Settings";

function App() {
  useEffect(() => {
    initPlayerEngine();
    initMediaIntegration();
    // P6.1: analyser → --band-* vars (visibility-gated rAF) + track palette
    // → --palette-* vars. StrictMode double-mount is handled by cleanup.
    const stopBands = initBandsBridge();
    const stopPalette = initPaletteWiring();
    // P6.3: one-time Eco suggestion when the battery drops below 20%.
    const stopBattery = initBatterySuggest();
    // P8.3: persist queue/position/volume; restore paused after the library.
    const detachPersist = attachSessionPersist();
    // P9.4 (OQ2): keep ytdlpPath fresh even if Settings unmounts mid-update.
    const detachYtdlpSync = initYtdlpPathSync();
    // P10: one play record per track start (pure rule in lib/playGate.ts).
    const detachPlayHistory = initPlayHistory();
    if (inTauri) {
      void listTracks()
        .then(useLibraryStore.getState().setTracks)
        .then(() => restoreSession())
        .finally(() => useLibraryStore.getState().setHydrated(true));
    } else {
      useLibraryStore.getState().setHydrated(true);
    }
    void runSmokeIfEnabled();
    return () => {
      detachPersist();
      detachYtdlpSync();
      detachPlayHistory();
      stopBands();
      stopPalette();
      stopBattery();
    };
  }, []);

  return (
    <HashRouter>
      <ToastHost />
      <DownloadPanel />
      <Announcer />
      <ImmersiveLyrics />
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<HomePage />} />
          <Route path="library" element={<LibraryPage />} />
          <Route path="playlists" element={<PlaylistsPage />} />
          <Route path="playlists/:id" element={<PlaylistDetailPage />} />
          <Route path="moods/:id" element={<MoodPage />} />
          <Route path="search" element={<SearchPage />} />
          <Route path="cached" element={<CachedPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}

export default App;

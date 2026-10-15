import { useEffect } from "react";
import { HashRouter, Route, Routes } from "react-router";
import { initMediaIntegration } from "@/audio/mediaSession";
import { initPlayerEngine } from "@/audio/engine";
import { runSmokeIfEnabled } from "@/audio/smoke";
import { ToastHost } from "@/components/common/ToastHost";
import { inTauri, listTracks } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { AppShell } from "@/routes/AppShell";
import HomePage from "@/routes/Home";
import LibraryPage from "@/routes/Library";
import PlaylistsPage from "@/routes/Playlists";
import PlaylistDetailPage from "@/routes/PlaylistDetail";
import SearchPage from "@/routes/Search";
import CachedPage from "@/routes/Cached";
import SettingsPage from "@/routes/Settings";

function App() {
  useEffect(() => {
    initPlayerEngine();
    initMediaIntegration();
    if (inTauri) {
      void listTracks()
        .then(useLibraryStore.getState().setTracks)
        .finally(() => useLibraryStore.getState().setHydrated(true));
    } else {
      useLibraryStore.getState().setHydrated(true);
    }
    void runSmokeIfEnabled();
  }, []);

  return (
    <HashRouter>
      <ToastHost />
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<HomePage />} />
          <Route path="library" element={<LibraryPage />} />
          <Route path="playlists" element={<PlaylistsPage />} />
          <Route path="playlists/:id" element={<PlaylistDetailPage />} />
          <Route path="search" element={<SearchPage />} />
          <Route path="cached" element={<CachedPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}

export default App;

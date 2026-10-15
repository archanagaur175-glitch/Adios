import { HashRouter, Route, Routes } from "react-router";
import { AppShell } from "@/routes/AppShell";
import HomePage from "@/routes/Home";
import LibraryPage from "@/routes/Library";
import PlaylistsPage from "@/routes/Playlists";
import PlaylistDetailPage from "@/routes/PlaylistDetail";
import SearchPage from "@/routes/Search";
import CachedPage from "@/routes/Cached";
import SettingsPage from "@/routes/Settings";

function App() {
  return (
    <HashRouter>
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

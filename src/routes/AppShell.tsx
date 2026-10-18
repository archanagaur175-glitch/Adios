import { useEffect, useState } from "react";
import { Outlet } from "react-router";
import { installShortcuts } from "@/lib/shortcuts";
import { Titlebar } from "@/components/titlebar/Titlebar";
import { Sidebar } from "@/components/sidebar/Sidebar";
import { PlayerBar } from "@/components/player/PlayerBar";
import { SearchOverlay } from "@/components/search/SearchOverlay";
import { NowPlaying } from "@/components/nowplaying/NowPlaying";
import { Aurora } from "@/visuals/aurora";
import { VisualizerHost } from "@/visuals/VisualizerHost";
import "./appshell.css";

export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);
  const [drawer, setDrawer] = useState(false);
  useEffect(() => installShortcuts(), []);

  return (
    <div className="app-surface">
      <Aurora />
      <VisualizerHost />
      <Titlebar onMenu={() => setDrawer((d) => !d)} menuOpen={drawer} />
      <div className="shell">
        <Sidebar
          collapsed={collapsed}
          onToggle={() => setCollapsed((c) => !c)}
          open={drawer}
          onNavigate={() => setDrawer(false)}
        />
        <main className="shell__main" id="main-scroll">
          <Outlet />
        </main>
      </div>
      {drawer && (
        <div
          className="shell__scrim"
          onClick={() => setDrawer(false)}
          role="presentation"
        />
      )}
      <PlayerBar />
      <NowPlaying />
      <SearchOverlay />
    </div>
  );
}

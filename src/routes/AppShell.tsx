import { useEffect, useState } from "react";
import { Outlet } from "react-router";
import { installShortcuts } from "@/lib/shortcuts";
import { Titlebar } from "@/components/titlebar/Titlebar";
import { Sidebar } from "@/components/sidebar/Sidebar";
import { PlayerBar } from "@/components/player/PlayerBar";
import { SearchOverlay } from "@/components/search/SearchOverlay";
import { Aurora } from "@/visuals/aurora";
import "./appshell.css";

export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);
  useEffect(() => installShortcuts(), []);

  return (
    <div className="app-surface">
      <Aurora />
      <Titlebar />
      <div className="shell">
        <Sidebar
          collapsed={collapsed}
          onToggle={() => setCollapsed((c) => !c)}
        />
        <main className="shell__main" id="main-scroll">
          <Outlet />
        </main>
      </div>
      <PlayerBar />
      <SearchOverlay />
    </div>
  );
}

import { useState } from "react";
import { Outlet } from "react-router";
import { Titlebar } from "@/components/titlebar/Titlebar";
import { Sidebar } from "@/components/sidebar/Sidebar";
import "./appshell.css";

export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="app-surface">
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
      <footer className="player-bar-placeholder" aria-hidden="true">
        Player bar arrives in P3
      </footer>
    </div>
  );
}

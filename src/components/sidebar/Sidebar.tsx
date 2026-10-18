import { NavLink } from "react-router";
import "./sidebar.css";

type IconName = "home" | "search" | "library" | "playlist";

const paths: Record<IconName, string> = {
  home: "M2 6l6-4 6 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z",
  search: "M7 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10zm4 0l4 4",
  library: "M2 2v12M6 2v12M10 3l4-1 2 11-4 1z",
  playlist: "M2 4h8M2 8h8M2 12h5M12 6v6m0 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm0-6a2 2 0 1 0 0 4 2 2 0 0 0 0-4z",
};

function Icon({ name }: { name: IconName }) {
  return (
    <svg className="sidebar__icon" viewBox="0 0 16 16" aria-hidden="true">
      <path d={paths[name]} />
    </svg>
  );
}

const links: { to: string; label: string; icon: IconName; end?: boolean }[] = [
  { to: "/", label: "Home", icon: "home", end: true },
  { to: "/search", label: "Search", icon: "search" },
  { to: "/library", label: "Library", icon: "library" },
  { to: "/playlists", label: "Playlists", icon: "playlist" },
];

export function Sidebar({
  collapsed,
  onToggle,
  open,
  onNavigate,
}: {
  collapsed: boolean;
  onToggle: () => void;
  open?: boolean;
  onNavigate?: () => void;
}) {
  return (
    <nav
      className={`sidebar${collapsed ? " sidebar--collapsed" : ""}${
        open ? " sidebar--open" : ""
      }`}
      aria-label="Main navigation"
    >
      <button
        type="button"
        className="sidebar__toggle"
        aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        aria-expanded={!collapsed}
        onClick={onToggle}
      >
        ☰
      </button>
      <ul className="sidebar__list">
        {links.map(({ to, label, icon, end }) => (
          <li key={to}>
            <NavLink
              to={to}
              end={end}
              onClick={onNavigate}
              className={({ isActive }) =>
                `sidebar__link${isActive ? " sidebar__link--active" : ""}`
              }
              title={label}
            >
              <Icon name={icon} />
              <span className="sidebar__label">{label}</span>
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}

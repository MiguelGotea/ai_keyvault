/**
 * AppLayout — Layout principal de la app con sidebar de navegación.
 * Se usa en Dashboard y todas las páginas internas.
 */
import { useEffect, useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import ServerStatusBadge from "./ServerStatusBadge";
import PollerStatusBadge from "./PollerStatusBadge";
import CommandPalette from "./CommandPalette";
import { useServerHealth } from "../hooks/useServerHealth";
import es from "../i18n/es.json";

const NAV_ITEMS = [
  { to: "/dashboard", icon: "🏠", label: es.nav.dashboard },
  { to: "/alerts", icon: "🔔", label: es.nav.alerts },
  { to: "/ping", icon: "📡", label: es.nav.ping },
  { to: "/chats", icon: "💬", label: es.nav.chats },
  { to: "/accounts", icon: "🗝️", label: es.nav.accounts },
  { to: "/stats", icon: "📊", label: es.nav.stats },
  { to: "/settings", icon: "⚙️", label: es.nav.settings },
];

export default function AppLayout() {
  const navigate = useNavigate();
  const health = useServerHealth();
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen(o => !o);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  async function handleLock() {
    await invoke("lock_vault");
    navigate("/unlock");
  }

  return (
    <div style={styles.shell}>
      {/* Sidebar */}
      <aside style={styles.sidebar}>
        {/* Logo */}
        <div style={styles.sidebarHeader}>
          <span style={styles.sidebarIcon}>🔐</span>
          <span style={styles.sidebarTitle}>AI KeyVault</span>
        </div>

        {/* Navegación */}
        <nav style={styles.nav}>
          {NAV_ITEMS.map(item => (
            <NavLink
              key={item.to}
              to={item.to}
              style={({ isActive }) => ({
                ...styles.navItem,
                ...(isActive ? styles.navItemActive : {}),
              })}
            >
              <span style={styles.navIcon}>{item.icon}</span>
              <span>{item.label}</span>
            </NavLink>
          ))}
        </nav>

        {/* Footer del sidebar */}
        <div style={styles.sidebarFooter}>
          <div style={{ marginBottom: 8 }}>
            <ServerStatusBadge health={health} compact />
          </div>
          <PollerStatusBadge compact />
          <button
            style={{ ...styles.lockBtn, marginTop: 8, background: "transparent", color: "var(--color-text-secondary)", borderColor: "var(--color-border)" }}
            onClick={() => setPaletteOpen(true)}
            title="Abrir paleta de comandos (Ctrl+K)"
          >
            ⌘ Comandos (Ctrl+K)
          </button>
          <button style={styles.lockBtn} onClick={handleLock}>
            🔒 {es.vault.bloquear}
          </button>
        </div>
      </aside>

      {/* Contenido principal */}
      <main style={styles.main}>
        <Outlet />
      </main>

      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} />
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  shell: {
    display: "flex",
    minHeight: "100vh",
    background: "var(--color-bg-secondary)",
  },
  sidebar: {
    width: 220,
    background: "var(--color-bg)",
    borderRight: "1px solid var(--color-border)",
    display: "flex",
    flexDirection: "column",
    flexShrink: 0,
  },
  sidebarHeader: {
    display: "flex",
    alignItems: "center",
    gap: 10,
    padding: "20px 18px 16px",
    borderBottom: "1px solid var(--color-border-subtle)",
  },
  sidebarIcon: { fontSize: 22 },
  sidebarTitle: {
    fontSize: 15,
    fontWeight: 700,
    color: "var(--color-primary)",
  },
  nav: {
    flex: 1,
    padding: "12px 8px",
    display: "flex",
    flexDirection: "column",
    gap: 2,
  },
  navItem: {
    display: "flex",
    alignItems: "center",
    gap: 10,
    padding: "9px 12px",
    borderRadius: "var(--radius-md)",
    fontSize: 13,
    color: "var(--color-text-secondary)",
    textDecoration: "none",
    transition: "background var(--transition-fast)",
  } as React.CSSProperties,
  navItemActive: {
    background: "var(--color-primary)",
    color: "#fff",
    fontWeight: 600,
  },
  navIcon: { fontSize: 15, flexShrink: 0 },
  sidebarFooter: {
    padding: "12px 14px 16px",
    borderTop: "1px solid var(--color-border-subtle)",
    display: "flex",
    flexDirection: "column",
    gap: 6,
  },
  lockBtn: {
    marginTop: 8,
    padding: "8px 10px",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    background: "transparent",
    color: "var(--color-text-secondary)",
    fontSize: 12,
    cursor: "pointer",
    textAlign: "left" as const,
    width: "100%",
  },
  main: {
    flex: 1,
    overflow: "auto",
  },
};

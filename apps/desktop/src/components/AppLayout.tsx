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
import { useTheme } from "../hooks/useTheme";
import logoIconLight from "../assets/ai-keyvault-icon.svg";
import logoTextLight from "../assets/ai-keyvault-text.svg";
import logoIconDark from "../assets/ai-keyvault-icon-dark.svg";
import logoTextDark from "../assets/ai-keyvault-text-dark.svg";
import es from "../i18n/es.json";

const NAV_ITEMS = [
  { to: "/dashboard", icon: "🏠", label: es.nav.dashboard },
  { to: "/alerts", icon: "🔔", label: es.nav.alerts },
  { to: "/ping", icon: "📡", label: es.nav.ping },
  { to: "/chats", icon: "💬", label: es.nav.chats },
  { to: "/accounts", icon: "🗝️", label: es.nav.accounts },
  { to: "/devices", icon: "💻", label: "Mis dispositivos" },
  { to: "/cooldown", icon: "⏱️", label: "Cooldown" },
  { to: "/stats", icon: "📊", label: es.nav.stats },
  { to: "/settings", icon: "⚙️", label: es.nav.settings },
];

export default function AppLayout() {
  const navigate = useNavigate();
  const health = useServerHealth();
  const theme = useTheme();
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
          <img
            src={theme === "dark" ? logoIconDark : logoIconLight}
            alt="AI KeyVault"
            style={styles.sidebarLogoIcon}
          />
          <img
            src={theme === "dark" ? logoTextDark : logoTextLight}
            alt="AI KeyVault"
            style={styles.sidebarLogoText}
          />
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
            style={{ ...styles.lockBtn, marginTop: 8 }}
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
    height: "100vh",
    overflow: "hidden",
    background: "var(--color-bg-secondary)",
  },
  sidebar: {
    width: 220,
    height: "100%",
    background: "var(--color-bg)",
    borderRight: "1px solid var(--color-border)",
    display: "flex",
    flexDirection: "column",
    flexShrink: 0,
    overflow: "hidden",
  },
  sidebarHeader: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "16px 16px 14px",
    borderBottom: "1px solid var(--color-border-subtle)",
    flexShrink: 0,
  },
  sidebarLogoIcon: {
    width: 30,
    height: 30,
    borderRadius: 6,
    objectFit: "contain",
  },
  sidebarLogoText: {
    height: 22,
    borderRadius: 4,
    objectFit: "contain",
  },
  nav: {
    flex: 1,
    minHeight: 0,
    overflowY: "auto",
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
    color: "var(--color-on-primary)",
    fontWeight: 600,
  },
  navIcon: { fontSize: 15, flexShrink: 0 },
  sidebarFooter: {
    padding: "12px 14px 16px",
    borderTop: "1px solid var(--color-border-subtle)",
    display: "flex",
    flexDirection: "column",
    gap: 6,
    flexShrink: 0,
  },
  lockBtn: {
    marginTop: 8,
    padding: "8px 10px",
    border: "1px solid var(--color-btn-border)",
    borderRadius: "var(--radius-md)",
    background: "var(--color-btn-bg)",
    color: "var(--color-btn-text)",
    fontSize: 12,
    cursor: "pointer",
    textAlign: "left" as const,
    width: "100%",
  },
  main: {
    flex: 1,
    height: "100%",
    overflowY: "auto",
  },
};

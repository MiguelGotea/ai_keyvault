import { useEffect, useRef } from "react";
import { Routes, Route, Navigate, useLocation, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { VaultStatus } from "./types/api";
import UnlockPage from "./pages/UnlockPage";
import DashboardPage from "./pages/DashboardPage";
import AlertsPage from "./pages/AlertsPage";
import PingPage from "./pages/PingPage";
import ChatsPage from "./pages/ChatsPage";
import StatsPage from "./pages/StatsPage";
import SettingsPage from "./pages/SettingsPage";
import AccountsPage from "./pages/AccountsPage";
import DevicesPage from "./pages/DevicesPage";
import AppLayout from "./components/AppLayout";

/**
 * App — enrutador principal de AI KeyVault.
 *
 * Fase 14: escucha el evento "autolock" emitido por Rust cuando expira el timer
 * de inactividad. Al recibirlo bloquea la bóveda y navega a /unlock.
 */
function App() {
  const navigate = useNavigate();
  const location = useLocation();
  // Ref para evitar múltiples suscripciones al rerender
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // Aplicar tema guardado (claro / oscuro) al iniciar (Fase 25)
    try {
      const raw = localStorage.getItem("akv_settings");
      const theme = raw ? JSON.parse(raw).theme : "dark";
      document.documentElement.setAttribute("data-theme", theme === "light" ? "light" : "dark");
    } catch {
      document.documentElement.setAttribute("data-theme", "dark");
    }
  }, []);

  useEffect(() => {
    let active = true;
    // Suscribirse al evento de autobloqueo emitido desde Rust
    listen<void>("autolock", async () => {
      try {
        await invoke("lock_vault");
        await invoke("cancel_autolock_timer");
      } catch (e) {
        console.error("Error al bloquear tras autolock:", e);
      }
      navigate("/unlock", { replace: true });
    })
      .then((unlisten) => {
        if (active) {
          unlistenRef.current = unlisten;
        } else {
          unlisten();
        }
      })
      .catch((e) => {
        console.error("Error suscribiendo evento autolock:", e);
      });

    // Al restaurar/enfocar la ventana (si WebView2 estuvo suspendido minimizado),
    // sincronizar con el estado real de Rust.
    const checkLockedOnFocus = () => {
      if (location.pathname === "/unlock") return;
      invoke<VaultStatus>("get_vault_status")
        .then((status) => {
          if (status.locked) {
            navigate("/unlock", { replace: true });
          }
        })
        .catch((e) => {
          console.error("Error verificando estado de bóveda al enfocar:", e);
        });
    };

    window.addEventListener("focus", checkLockedOnFocus);
    document.addEventListener("visibilitychange", checkLockedOnFocus);

    return () => {
      active = false;
      unlistenRef.current?.();
      window.removeEventListener("focus", checkLockedOnFocus);
      document.removeEventListener("visibilitychange", checkLockedOnFocus);
    };
  }, [navigate, location.pathname]);

  return (
    <Routes>
      {/* Pantalla de desbloqueo — sin layout */}
      <Route path="/unlock" element={<UnlockPage />} />

      {/* Área protegida — con sidebar layout */}
      <Route element={<AppLayout />}>
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/alerts" element={<AlertsPage />} />
        <Route path="/ping" element={<PingPage />} />
        <Route path="/chats" element={<ChatsPage />} />
        <Route path="/accounts" element={<AccountsPage />} />
        <Route path="/devices" element={<DevicesPage />} />
        <Route path="/stats" element={<StatsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>

      {/* Redirigir raíz a unlock */}
      <Route path="/" element={<Navigate to="/unlock" replace />} />
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </Routes>
  );
}

export default App;

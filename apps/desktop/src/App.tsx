import { useEffect, useRef } from "react";
import { Routes, Route, Navigate, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import UnlockPage from "./pages/UnlockPage";
import DashboardPage from "./pages/DashboardPage";
import AlertsPage from "./pages/AlertsPage";
import PingPage from "./pages/PingPage";
import ChatsPage from "./pages/ChatsPage";
import StatsPage from "./pages/StatsPage";
import SettingsPage from "./pages/SettingsPage";
import AppLayout from "./components/AppLayout";

/**
 * App — enrutador principal de AI KeyVault.
 *
 * Fase 14: escucha el evento "autolock" emitido por Rust cuando expira el timer
 * de inactividad. Al recibirlo bloquea la bóveda y navega a /unlock.
 */
function App() {
  const navigate = useNavigate();
  // Ref para evitar múltiples suscripciones al rerender
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // Suscribirse al evento de autobloqueo emitido desde Rust
    listen<void>("autolock", async () => {
      try {
        await invoke("lock_vault");
        await invoke("cancel_autolock_timer");
      } catch {
        // Si la bóveda ya estaba bloqueada, no es error
      }
      navigate("/unlock", { replace: true });
    }).then((unlisten) => {
      unlistenRef.current = unlisten;
    });

    return () => {
      // Limpiar la suscripción al desmontar
      unlistenRef.current?.();
    };
  }, [navigate]);

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

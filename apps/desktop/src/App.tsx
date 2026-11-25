import { Routes, Route, Navigate } from "react-router-dom";
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
 *   / → /unlock
 *   /unlock → pantalla de desbloqueo con bóveda local (Fase 12)
 *   /dashboard → Dashboard con cuota de todas las cuentas (Fase 13)
 *   /alerts → Log de alertas (Fase 14)
 *   /ping → Ping mínimo (Fase 14)
 *   /chats → Chats activos (Fase 15)
 *   /stats → Estadísticas avanzadas (Fase 15)
 *   /settings → Configuración (Fase 15)
 */
function App() {
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

import { Routes, Route, Navigate } from "react-router-dom";
import UnlockPage from "./pages/UnlockPage";
import DashboardPage from "./pages/DashboardPage";

/**
 * App — enrutador principal de AI KeyVault.
 * Las rutas se expanden en cada fase:
 *   Fase 10: /unlock (placeholder), /dashboard (placeholder)
 *   Fase 12: /unlock con lógica real de bóveda
 *   Fase 13: /dashboard con tarjetas reales
 *   Fase 14: /account/:id con pestañas
 *   Fase 15: /ping, /alerts, /chats, /devices, /settings
 *   Fase 16: /stats
 */
function App() {
  return (
    <Routes>
      <Route path="/unlock" element={<UnlockPage />} />
      <Route path="/dashboard" element={<DashboardPage />} />
      <Route path="/" element={<Navigate to="/unlock" replace />} />
    </Routes>
  );
}

export default App;

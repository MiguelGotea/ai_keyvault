/** UnlockPage — Fase 10: placeholder mínimo para que compile el routing.
 *  La lógica real de desbloqueo con clave maestra se implementa en Fase 12.
 */
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import type { ServerHealth } from "../types/api";
import es from "../i18n/es.json";

export default function UnlockPage() {
  const navigate = useNavigate();
  const [serverStatus, setServerStatus] = useState<string>("—");

  async function checkServer() {
    try {
      const health = await invoke<ServerHealth>("ping_server", {
        serverUrl: "http://198.211.97.243:3010",
      });
      setServerStatus(health.online ? `✅ ${health.status}` : `❌ ${health.status}`);
    } catch (e) {
      setServerStatus(`❌ Error: ${e}`);
    }
  }

  return (
    <div style={styles.container}>
      <div style={styles.card}>
        <h1 style={styles.title}>🔐 {es.app.nombre}</h1>
        <p style={styles.subtitle}>{es.app.descripcion}</p>

        <div style={styles.statusRow}>
          <span style={styles.label}>Estado del servidor:</span>
          <span style={styles.value}>{serverStatus}</span>
        </div>

        <button style={styles.btn} onClick={checkServer}>
          Verificar conexión con VPS
        </button>

        <button
          style={{ ...styles.btn, ...styles.btnPrimary, marginTop: 8 }}
          onClick={() => navigate("/dashboard")}
        >
          → Ir al Dashboard (Fase 10 — sin autenticación real)
        </button>

        <p style={styles.note}>
          ⚠️ Fase 10 — Scaffold. La autenticación real se implementa en Fase 12.
        </p>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: "100vh",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: "var(--color-bg-secondary)",
    padding: 24,
  },
  card: {
    background: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-xl)",
    padding: 40,
    maxWidth: 440,
    width: "100%",
    display: "flex",
    flexDirection: "column",
    gap: 16,
  },
  title: {
    fontSize: 24,
    fontWeight: 700,
    color: "var(--color-primary)",
  },
  subtitle: {
    fontSize: 14,
    color: "var(--color-text-secondary)",
  },
  statusRow: {
    display: "flex",
    gap: 8,
    alignItems: "center",
    fontSize: 13,
  },
  label: { color: "var(--color-text-secondary)" },
  value: { fontFamily: "var(--font-mono)", fontSize: 12 },
  btn: {
    padding: "10px 16px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border)",
    background: "var(--color-bg-secondary)",
    color: "var(--color-text)",
    fontSize: 14,
    cursor: "pointer",
  },
  btnPrimary: {
    background: "var(--color-primary)",
    color: "#fff",
    border: "none",
  },
  note: {
    fontSize: 12,
    color: "var(--color-text-disabled)",
    marginTop: 8,
  },
};

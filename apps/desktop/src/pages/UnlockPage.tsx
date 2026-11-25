/** UnlockPage — Fase 11: pantalla de desbloqueo con verificación real del servidor.
 *  La lógica de desbloqueo con clave maestra se implementa en Fase 12.
 *  Por ahora: verifica conexión con VPS via invoke() → Rust → reqwest.
 */
import { useNavigate } from "react-router-dom";
import { useServerHealth } from "../hooks/useServerHealth";
import ServerStatusBadge from "../components/ServerStatusBadge";
import es from "../i18n/es.json";

export default function UnlockPage() {
  const navigate = useNavigate();
  const health = useServerHealth();

  return (
    <div style={styles.container}>
      <div style={styles.card}>
        {/* Logo / título */}
        <div style={styles.header}>
          <span style={styles.icon}>🔐</span>
          <h1 style={styles.title}>{es.app.nombre}</h1>
          <p style={styles.subtitle}>{es.app.descripcion}</p>
        </div>

        {/* Estado del servidor */}
        <div style={styles.statusRow}>
          <span style={styles.label}>Conexión VPS:</span>
          <ServerStatusBadge health={health} />
          <button style={styles.recheckBtn} onClick={health.recheck} disabled={health.checking}>
            ↻
          </button>
        </div>

        {health.error && (
          <div style={styles.errorBox}>
            ⚠️ {health.error}
          </div>
        )}

        {/* Campo de clave maestra — placeholder Fase 11 */}
        <div style={styles.inputGroup}>
          <label style={styles.inputLabel}>{es.vault.clave_maestra}</label>
          <input
            type="password"
            placeholder="••••••••••••"
            style={styles.input}
            disabled
            title="Implementado en Fase 12"
          />
          <p style={styles.inputHint}>
            ⚠️ Autenticación real en Fase 12
          </p>
        </div>

        {/* Botón de desbloqueo — placeholder */}
        <button
          style={{ ...styles.btn, ...styles.btnPrimary }}
          onClick={() => navigate("/dashboard")}
          title="En Fase 12 validará la clave maestra"
        >
          {es.vault.desbloquear} →
        </button>

        <p style={styles.note}>
          Fase 11 — Capa de comunicación Rust ↔ VPS ↔ React activa
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
    gap: 20,
    boxShadow: "var(--shadow-lg)",
  },
  header: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 8,
    textAlign: "center",
  },
  icon: { fontSize: 48, lineHeight: 1 },
  title: {
    fontSize: 22,
    fontWeight: 700,
    color: "var(--color-primary)",
    margin: 0,
  },
  subtitle: {
    fontSize: 13,
    color: "var(--color-text-secondary)",
    margin: 0,
  },
  statusRow: {
    display: "flex",
    gap: 8,
    alignItems: "center",
    padding: "10px 14px",
    background: "var(--color-bg-secondary)",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border-subtle)",
  },
  label: {
    fontSize: 12,
    color: "var(--color-text-secondary)",
    flexShrink: 0,
  },
  recheckBtn: {
    marginLeft: "auto",
    background: "none",
    border: "none",
    cursor: "pointer",
    color: "var(--color-text-secondary)",
    fontSize: 14,
    padding: "2px 6px",
    borderRadius: "var(--radius-sm)",
  },
  errorBox: {
    padding: "10px 14px",
    background: "#FEF2F2",
    border: "1px solid #FECACA",
    borderRadius: "var(--radius-md)",
    fontSize: 12,
    color: "#991B1B",
  },
  inputGroup: {
    display: "flex",
    flexDirection: "column",
    gap: 6,
  },
  inputLabel: {
    fontSize: 13,
    fontWeight: 500,
    color: "var(--color-text)",
  },
  input: {
    padding: "10px 14px",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    fontSize: 15,
    outline: "none",
    background: "var(--color-bg)",
    color: "var(--color-text)",
    fontFamily: "var(--font-mono)",
  },
  inputHint: {
    fontSize: 11,
    color: "var(--color-text-disabled)",
    margin: 0,
  },
  btn: {
    padding: "12px 16px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border)",
    background: "var(--color-bg-secondary)",
    color: "var(--color-text)",
    fontSize: 14,
    cursor: "pointer",
    fontWeight: 500,
  },
  btnPrimary: {
    background: "var(--color-primary)",
    color: "#fff",
    border: "none",
  },
  note: {
    fontSize: 11,
    color: "var(--color-text-disabled)",
    textAlign: "center",
    margin: 0,
  },
};

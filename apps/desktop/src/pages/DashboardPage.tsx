/** DashboardPage — Fase 11: placeholder con llamada real al VPS via invoke().
 *  Muestra las lecturas de cuota del servidor para verificar el canal Rust ↔ VPS.
 *  La UI completa del dashboard se implementa en Fase 12–13.
 */
import { useNavigate } from "react-router-dom";
import { useCommand } from "../hooks/useCommand";
import { useServerHealth } from "../hooks/useServerHealth";
import ServerStatusBadge from "../components/ServerStatusBadge";
import type { PoolSummary } from "../types/api";
import es from "../i18n/es.json";

const TEST_ACCOUNT = "test@agy";  // Cambiar por cuenta real en Fase 12

export default function DashboardPage() {
  const navigate = useNavigate();
  const health = useServerHealth();

  // Prueba real de la capa de comunicación Rust ↔ VPS
  const quota = useCommand<PoolSummary[]>("get_stats_summary", {
    accountId: TEST_ACCOUNT,
    days: 7,
  });

  return (
    <div style={styles.page}>
      {/* Header */}
      <header style={styles.header}>
        <h1 style={styles.title}>🔐 {es.app.nombre}</h1>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <ServerStatusBadge health={health} compact />
          <button style={styles.btnOutline} onClick={() => navigate("/unlock")}>
            🔒 Bloquear
          </button>
        </div>
      </header>

      {/* Contenido */}
      <main style={styles.main}>
        <section style={styles.section}>
          <h2 style={styles.sectionTitle}>Fase 11 — Verificación de canal Rust ↔ VPS</h2>
          <p style={styles.hint}>
            Las tarjetas de cuota reales se implementan en Fase 12–13.
            Este bloque confirma que <code>invoke("get_stats_summary")</code> llega al VPS.
          </p>

          {quota.loading && (
            <div style={styles.loadingBox}>⏳ Consultando VPS…</div>
          )}

          {quota.error && (
            <div style={styles.errorBox}>
              <strong>Error al consultar cuota:</strong><br />
              {quota.error}
            </div>
          )}

          {quota.data && quota.data.length === 0 && (
            <div style={styles.emptyBox}>
              Sin lecturas de cuota para <code>{TEST_ACCOUNT}</code>
            </div>
          )}

          {quota.data && quota.data.length > 0 && (
            <div style={styles.cardGrid}>
              {quota.data.map((pool) => (
                <div
                  key={`${pool.account_id}-${pool.pool}`}
                  style={styles.quotaCard}
                >
                  <div style={styles.cardProvider}>{pool.provider} / {pool.pool}</div>
                  <div style={styles.cardPercent}>
                    <span
                      style={{
                        color: pool.current_percent >= 95
                          ? "var(--color-danger)"
                          : pool.current_percent >= 80
                          ? "var(--color-warning)"
                          : "var(--color-success)",
                        fontWeight: 700,
                        fontSize: 28,
                      }}
                    >
                      {pool.current_percent.toFixed(1)}%
                    </span>
                    <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                      usado
                    </span>
                  </div>
                  {pool.burn_rate_per_hour !== null && (
                    <div style={styles.cardMeta}>
                      Burn: {pool.burn_rate_per_hour?.toFixed(2)}%/h
                    </div>
                  )}
                  {pool.estimated_exhaustion_at && (
                    <div style={styles.cardMeta}>
                      Agota: {new Date(pool.estimated_exhaustion_at).toLocaleString("es-MX")}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          <button
            style={{ ...styles.btn, marginTop: 16 }}
            onClick={quota.refetch}
            disabled={quota.loading}
          >
            ↻ Refrescar cuota
          </button>
        </section>
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: {
    minHeight: "100vh",
    display: "flex",
    flexDirection: "column",
    background: "var(--color-bg-secondary)",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "14px 24px",
    background: "var(--color-bg)",
    borderBottom: "1px solid var(--color-border)",
  },
  title: {
    fontSize: 18,
    fontWeight: 700,
    color: "var(--color-primary)",
    margin: 0,
  },
  main: {
    flex: 1,
    padding: 24,
    maxWidth: 960,
    width: "100%",
    margin: "0 auto",
  },
  section: {
    background: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-lg)",
    padding: 24,
  },
  sectionTitle: {
    fontSize: 15,
    fontWeight: 600,
    color: "var(--color-text)",
    margin: "0 0 8px",
  },
  hint: {
    fontSize: 12,
    color: "var(--color-text-secondary)",
    margin: "0 0 16px",
  },
  loadingBox: {
    padding: "12px 16px",
    background: "var(--color-bg-secondary)",
    borderRadius: "var(--radius-md)",
    fontSize: 13,
    color: "var(--color-text-secondary)",
  },
  errorBox: {
    padding: "12px 16px",
    background: "#FEF2F2",
    border: "1px solid #FECACA",
    borderRadius: "var(--radius-md)",
    fontSize: 12,
    color: "#991B1B",
  },
  emptyBox: {
    padding: "12px 16px",
    background: "var(--color-bg-secondary)",
    borderRadius: "var(--radius-md)",
    fontSize: 13,
    color: "var(--color-text-disabled)",
    fontStyle: "italic",
  },
  cardGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
    gap: 12,
  },
  quotaCard: {
    background: "var(--color-bg-secondary)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    padding: "14px 16px",
    display: "flex",
    flexDirection: "column",
    gap: 4,
  },
  cardProvider: {
    fontSize: 11,
    color: "var(--color-text-secondary)",
    fontFamily: "var(--font-mono)",
  },
  cardPercent: {
    display: "flex",
    alignItems: "baseline",
    gap: 4,
    margin: "4px 0",
  },
  cardMeta: {
    fontSize: 11,
    color: "var(--color-text-disabled)",
    fontFamily: "var(--font-mono)",
  },
  btn: {
    padding: "9px 14px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border)",
    background: "var(--color-bg-secondary)",
    color: "var(--color-text)",
    fontSize: 13,
    cursor: "pointer",
  },
  btnOutline: {
    padding: "7px 12px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border)",
    background: "transparent",
    color: "var(--color-text-secondary)",
    fontSize: 12,
    cursor: "pointer",
  },
};

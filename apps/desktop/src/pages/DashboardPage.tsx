/**
 * DashboardPage — Fase 13: Dashboard real con tarjetas de cuota.
 * Fase 14: reset del timer de autobloqueo en interacción del usuario.
 * Muestra resumen de todas las cuentas vía get_stats_compare.
 * Permite iniciar el poller y cambiar cuenta activa.
 */
import { useEffect, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import QuotaCard from "../components/QuotaCard";
import type { PoolSummary, SwitchRecord } from "../types/api";
import es from "../i18n/es.json";

export default function DashboardPage() {
  const [pollerStarted, setPollerStarted] = useState(false);
  const [pollerError, setPollerError] = useState<string | null>(null);
  const [switchingAccount, setSwitchingAccount] = useState<string | null>(null);
  const [switchMsg, setSwitchMsg] = useState<string | null>(null);

  // Comparativa de cuota de todas las cuentas (últimos 7 días)
  const quota = useCommand<PoolSummary[]>("get_stats_compare", { days: 7 });

  // Historial de switches recientes
  const switchHistory = useCommand<SwitchRecord[]>("get_switch_history", { limit: 5 });

  // Lanzar poller al abrir el dashboard (si la bóveda está desbloqueada)
  useEffect(() => {
    if (!pollerStarted) {
      invoke("start_poller")
        .then(() => setPollerStarted(true))
        .catch((e: unknown) => {
          // Si el sidecar no está compilado aún, solo logueamos
          const msg = String(e);
          if (msg.includes("sidecar")) {
            setPollerError("Sidecar no compilado aún — compila el poller MSVC primero");
          }
        });
    }
  }, [pollerStarted]);

  // Fase 14: reset del timer de autobloqueo en cada interacción del usuario.
  // Usamos throttle manual con useCallback para no llamar Rust en cada pixel.
  const throttleRef = { ts: 0 };
  const handleActivity = useCallback(() => {
    const now = Date.now();
    if (now - throttleRef.ts < 30_000) return; // throttle 30 s
    throttleRef.ts = now;
    const minutes = parseInt(localStorage.getItem("autolock_minutes") ?? "0");
    if (minutes > 0) {
      invoke("reset_autolock_timer", { minutes }).catch(() => {});
    }
  }, []);

  useEffect(() => {
    window.addEventListener("mousemove", handleActivity, { passive: true });
    window.addEventListener("keydown", handleActivity, { passive: true });
    return () => {
      window.removeEventListener("mousemove", handleActivity);
      window.removeEventListener("keydown", handleActivity);
    };
  }, [handleActivity]);

  // Agrupar pools por cuenta
  const byAccount = (quota.data ?? []).reduce<Record<string, PoolSummary[]>>((acc, pool) => {
    const key = `${pool.provider}:${pool.account_id}`;
    if (!acc[key]) acc[key] = [];
    acc[key].push(pool);
    return acc;
  }, {});

  async function handleSwitch(accountId: string, provider: string) {
    setSwitchingAccount(`${provider}:${accountId}`);
    setSwitchMsg(null);
    try {
      await invoke("switch_account", {
        accountId,
        provider,
        deviceId: "desktop-dev",
      });
      setSwitchMsg(`✅ Switch registrado para ${accountId}`);
      switchHistory.refetch();
    } catch (e) {
      setSwitchMsg(`❌ Error: ${String(e)}`);
    } finally {
      setSwitchingAccount(null);
    }
  }

  return (
    <div style={styles.page}>
      {/* Header */}
      <div style={styles.pageHeader}>
        <h2 style={styles.pageTitle}>{es.dashboard.titulo}</h2>
        <div style={styles.headerActions}>
          <button
            style={styles.refreshBtn}
            onClick={quota.refetch}
            disabled={quota.loading}
          >
            {quota.loading ? "⏳" : "↻"} Actualizar
          </button>
        </div>
      </div>

      {/* Banner de poller */}
      {pollerError && (
        <div style={styles.warningBanner}>
          ⚠️ {pollerError}
        </div>
      )}

      {/* Switch message */}
      {switchMsg && (
        <div style={{
          ...styles.infoBar,
          color: switchMsg.startsWith("✅") ? "var(--color-success)" : "var(--color-danger)"
        }}>
          {switchMsg}
        </div>
      )}

      {/* Sin cuentas */}
      {!quota.loading && quota.data?.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 40 }}>📊</span>
          <p>{es.dashboard.sin_cuentas}</p>
          <p style={{ fontSize: 12, color: "var(--color-text-disabled)" }}>
            El poller recopila datos automáticamente.
            Espera unos minutos o reinicia la PC con AI KeyVault abierto.
          </p>
        </div>
      )}

      {/* Error */}
      {quota.error && (
        <div style={styles.errorBox}>
          <strong>Error al obtener cuota:</strong> {quota.error}
          <button onClick={quota.refetch} style={styles.retryBtn}>
            Reintentar
          </button>
        </div>
      )}

      {/* Tarjetas por cuenta */}
      {Object.entries(byAccount).map(([key, pools]) => {
        const [provider, accountId] = key.split(":");
        const hasAlert = pools.some(p => p.current_percent >= 80);
        return (
          <section key={key} style={styles.accountSection}>
            <div style={styles.accountHeader}>
              <div>
                <span style={styles.accountProvider}>
                  {provider === "agy" ? "Antigravity" : "Codex"}
                </span>
                <span style={styles.accountId}>{accountId}</span>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                {hasAlert && <span style={styles.alertDot}>⚠️</span>}
                <button
                  style={styles.switchBtn}
                  onClick={() => handleSwitch(accountId, provider)}
                  disabled={switchingAccount === key}
                >
                  {switchingAccount === key ? "Cambiando…" : es.dashboard.cambiar_cuenta}
                </button>
              </div>
            </div>
            <div style={styles.cardGrid}>
              {pools.map(pool => (
                <QuotaCard
                  key={pool.pool}
                  pool={pool}
                />
              ))}
            </div>
          </section>
        );
      })}

      {/* Historial de switches */}
      {switchHistory.data && switchHistory.data.length > 0 && (
        <section style={{ ...styles.accountSection, marginTop: 24 }}>
          <div style={styles.accountHeader}>
            <span style={styles.accountProvider}>Historial de switches recientes</span>
          </div>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>Cuenta</th>
                <th style={styles.th}>Proveedor</th>
                <th style={styles.th}>Estado</th>
                <th style={styles.th}>Fecha</th>
              </tr>
            </thead>
            <tbody>
              {switchHistory.data.map(r => (
                <tr key={r.id}>
                  <td style={styles.td}>{r.account_id}</td>
                  <td style={styles.td}>{r.provider}</td>
                  <td style={styles.td}>
                    <span style={{ color: r.success ? "var(--color-success)" : "var(--color-danger)" }}>
                      {r.success ? "✅ OK" : "❌ Falló"}
                    </span>
                    {r.reverted && " (revertido)"}
                  </td>
                  <td style={styles.td}>
                    {new Date(r.switched_at).toLocaleString("es-MX")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: {
    padding: 24,
    display: "flex",
    flexDirection: "column",
    gap: 20,
  },
  pageHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
  },
  pageTitle: {
    fontSize: 20,
    fontWeight: 700,
    color: "var(--color-text)",
    margin: 0,
  },
  headerActions: { display: "flex", gap: 8 },
  refreshBtn: {
    padding: "7px 14px",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    background: "var(--color-bg)",
    color: "var(--color-text-secondary)",
    fontSize: 13,
    cursor: "pointer",
  },
  warningBanner: {
    padding: "10px 16px",
    background: "#FFFBEB",
    border: "1px solid #FDE68A",
    borderRadius: "var(--radius-md)",
    fontSize: 12,
    color: "#92400E",
  },
  infoBar: {
    padding: "8px 14px",
    background: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    fontSize: 13,
  },
  empty: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 12,
    padding: 48,
    color: "var(--color-text-secondary)",
    textAlign: "center",
  },
  errorBox: {
    padding: "12px 16px",
    background: "#FEF2F2",
    border: "1px solid #FECACA",
    borderRadius: "var(--radius-md)",
    fontSize: 13,
    color: "#991B1B",
    display: "flex",
    gap: 12,
    alignItems: "center",
  },
  retryBtn: {
    padding: "5px 10px",
    border: "1px solid #FECACA",
    borderRadius: "var(--radius-sm)",
    background: "white",
    color: "#991B1B",
    fontSize: 12,
    cursor: "pointer",
  },
  accountSection: {
    background: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-lg)",
    padding: "18px 20px",
    display: "flex",
    flexDirection: "column",
    gap: 14,
  },
  accountHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    gap: 12,
  },
  accountProvider: {
    fontSize: 11,
    fontWeight: 700,
    color: "var(--color-primary)",
    textTransform: "uppercase" as const,
    letterSpacing: "0.06em",
    marginRight: 10,
  },
  accountId: {
    fontSize: 13,
    color: "var(--color-text-secondary)",
    fontFamily: "var(--font-mono)",
  },
  alertDot: { fontSize: 14 },
  switchBtn: {
    padding: "6px 12px",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    background: "var(--color-bg-secondary)",
    color: "var(--color-text-secondary)",
    fontSize: 12,
    cursor: "pointer",
  },
  cardGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
    gap: 14,
  },
  table: {
    width: "100%",
    borderCollapse: "collapse" as const,
    fontSize: 12,
  },
  th: {
    padding: "7px 10px",
    textAlign: "left" as const,
    color: "var(--color-text-disabled)",
    fontWeight: 600,
    borderBottom: "1px solid var(--color-border)",
    fontSize: 11,
    textTransform: "uppercase" as const,
    letterSpacing: "0.05em",
  },
  td: {
    padding: "8px 10px",
    borderBottom: "1px solid var(--color-border-subtle)",
    color: "var(--color-text-secondary)",
    fontFamily: "var(--font-mono)",
  },
};

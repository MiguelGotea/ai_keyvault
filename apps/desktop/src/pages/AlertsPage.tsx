/**
 * AlertsPage — Log de alertas del VPS.
 * Solo muestra alertas de cuentas que están en la bóveda.
 */
import { useMemo } from "react";
import { useCommand } from "../hooks/useCommand";
import type { AlertRecord, AccountInfo } from "../types/api";
import es from "../i18n/es.json";

export default function AlertsPage() {
  // Cuentas de la bóveda (fuente de verdad)
  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});
  const alerts = useCommand<AlertRecord[]>("get_alerts", {});

  // D-02: filtrar alertas por UUID de bóveda y mostrar su alias
  const accountById = useMemo(
    () => new Map((vaultAccounts.data ?? []).map(a => [a.id, a])),
    [vaultAccounts.data]
  );

  const filteredAlerts = useMemo(
    () => (alerts.data ?? []).filter(a => accountById.has(a.account_id)),
    [alerts.data, accountById]
  );

  const isLoading = alerts.loading || vaultAccounts.loading;

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <h2 style={styles.title}>{es.alertas.titulo}</h2>
        <button style={styles.refreshBtn} onClick={alerts.refetch} disabled={isLoading}>
          {isLoading ? "⏳" : "↻"} Actualizar
        </button>
      </div>

      {isLoading && <div style={styles.hint}>Cargando alertas…</div>}
      {alerts.error && <div style={styles.error}>{alerts.error}</div>}

      {!isLoading && (vaultAccounts.data ?? []).length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 36 }}>🗝️</span>
          <p>No hay cuentas en la bóveda. Agrega una cuenta primero.</p>
        </div>
      )}

      {!isLoading && (vaultAccounts.data ?? []).length > 0 && filteredAlerts.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 36 }}>✅</span>
          <p>{es.alertas.sin_alertas}</p>
        </div>
      )}

      {filteredAlerts.length > 0 && (
        <div style={styles.list}>
          {filteredAlerts.map(alert => (
            <div
              key={alert.id}
              style={{
                ...styles.alertCard,
                borderLeft: `4px solid ${alert.level === "critical" || alert.level === "crit" ? "var(--color-danger)" : "var(--color-warning)"}`,
              }}
            >
              <div style={styles.alertTop}>
                <span style={{
                  ...styles.levelBadge,
                  background: alert.level === "critical" || alert.level === "crit" ? "var(--color-danger)" : "var(--color-warning)",
                }}>
                  {alert.level === "critical" || alert.level === "crit" ? es.alertas.nivel_crit : es.alertas.nivel_warn}
                </span>
                <span style={styles.alertProvider}>{alert.provider} / {alert.pool}</span>
                <span style={styles.alertTime}>
                  {new Date(alert.detected_at).toLocaleString("es-MX")}
                </span>
              </div>
              <div style={styles.alertBody}>
                <strong>{accountById.get(alert.account_id)?.alias ?? alert.account_id}</strong>
                {" — "}{alert.percent_used.toFixed(1)}% usado
                {alert.reset_time && (
                  <span style={styles.alertMsg}> · Reinicia: {new Date(alert.reset_time).toLocaleString("es-MX")}</span>
                )}
                {alert.notified_toast && <span style={styles.notifBadge}>🔔 Toast</span>}
                {alert.notified_ntfy && <span style={styles.notifBadge}>📲 Ntfy</span>}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 20 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  refreshBtn: { padding: "7px 14px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  hint: { color: "var(--color-text-secondary)", fontSize: 13 },
  error: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  empty: { display: "flex", flexDirection: "column", alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column", gap: 10 },
  alertCard: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", padding: "12px 16px", display: "flex", flexDirection: "column", gap: 6 },
  alertTop: { display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" as const },
  levelBadge: { fontSize: 10, fontWeight: 700, color: "#fff", padding: "2px 7px", borderRadius: 999 },
  alertProvider: { fontSize: 12, color: "var(--color-text-secondary)", fontFamily: "var(--font-mono)" },
  alertTime: { fontSize: 11, color: "var(--color-text-disabled)", marginLeft: "auto" },
  alertBody: { fontSize: 13, color: "var(--color-text)", display: "flex", flexWrap: "wrap" as const, gap: 6, alignItems: "center" },
  alertMsg: { color: "var(--color-text-secondary)", fontSize: 12 },
  notifBadge: { fontSize: 10, padding: "1px 6px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: 999 },
};

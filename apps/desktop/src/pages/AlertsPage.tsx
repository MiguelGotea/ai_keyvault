/**
 * AlertsPage — Log de alertas del VPS con sugerencia de cuenta alternativa (Fase 21).
 * Solo muestra alertas de cuentas que están en la bóveda.
 */
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import type { AlertRecord, AccountInfo, PollerStatus, PoolSummary, SwitchResult } from "../types/api";
import es from "../i18n/es.json";

interface AlternativeCandidate {
  account: AccountInfo;
  percentUsed: number | null;
}

export default function AlertsPage() {
  const [switchingId, setSwitchingId] = useState<string | null>(null);
  const [switchMsg, setSwitchMsg] = useState<string | null>(null);
  const [mutedUntil, setMutedUntil] = useState<string | null>(null);
  const [muteError, setMuteError] = useState<string | null>(null);

  // Cuentas de la bóveda (fuente de verdad)
  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});
  const alerts = useCommand<AlertRecord[]>("get_alerts", {});
  const compare = useCommand<PoolSummary[]>("get_stats_compare", { days: 7 });
  const pollerStatus = useCommand<PollerStatus>("get_poller_status", {});

  useEffect(() => {
    invoke<string | null>("get_alerts_muted_until")
      .then(setMutedUntil)
      .catch(e => setMuteError(`No se pudo leer estado de silencio: ${String(e)}`));
  }, []);

  async function setMuteHours(hours: number) {
    setMuteError(null);
    try {
      const until = await invoke<string | null>("set_alerts_muted_hours", { hours });
      setMutedUntil(until);
    } catch (e) {
      setMuteError(String(e));
    }
  }

  // Alertas de fin de suscripción (7 y 1 día)
  const subscriptionWarnings = useMemo(() => {
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const out: { account: AccountInfo; daysLeft: number; dateStr: string }[] = [];
    for (const acc of vaultAccounts.data ?? []) {
      if (!acc.subscription_ends_at) continue;
      const target = new Date(`${acc.subscription_ends_at}T00:00:00`).getTime();
      if (Number.isNaN(target)) continue;
      const daysLeft = Math.ceil((target - today) / 86_400_000);
      if (daysLeft <= 7) {
        out.push({ account: acc, daysLeft, dateStr: acc.subscription_ends_at });
      }
    }
    return out;
  }, [vaultAccounts.data]);

  // D-02: filtrar alertas por UUID de bóveda y mostrar su alias
  const accountById = useMemo(
    () => new Map((vaultAccounts.data ?? []).map(a => [a.id, a])),
    [vaultAccounts.data]
  );

  const filteredAlerts = useMemo(
    () => (alerts.data ?? []).filter(a => accountById.has(a.account_id)),
    [alerts.data, accountById]
  );

  // Índice de uso por `${account_id}:${pool}` para sugerir alternativa
  const usageByAccountPool = useMemo(() => {
    const map = new Map<string, number>();
    for (const p of compare.data ?? []) {
      map.set(`${p.account_id}:${p.pool}`, p.current_percent_used);
    }
    return map;
  }, [compare.data]);

  function findAlternative(alert: AlertRecord): AlternativeCandidate | null {
    const candidates = (vaultAccounts.data ?? []).filter(
      a => a.provider === alert.provider && a.id !== alert.account_id
    );
    if (candidates.length === 0) return null;

    let best: AlternativeCandidate | null = null;
    for (const cand of candidates) {
      const pct = usageByAccountPool.get(`${cand.id}:${alert.pool}`) ?? null;
      if (pct !== null && pct >= 80) continue;
      if (!best) {
        best = { account: cand, percentUsed: pct };
      } else {
        const bestScore = best.percentUsed ?? 50;
        const candScore = pct ?? 50;
        if (candScore < bestScore) {
          best = { account: cand, percentUsed: pct };
        }
      }
    }
    return best;
  }

  async function handleSwitchAlternative(account: AccountInfo) {
    setSwitchingId(account.id);
    setSwitchMsg(null);
    try {
      const result = await invoke<SwitchResult>("switch_account", {
        accountId: account.id,
        provider: account.provider,
        deviceId: "desktop-win",
      });
      if (result.success) {
        setSwitchMsg(`✅ Cambiado a '${account.alias}'. ${result.restart_notice ?? ""}`);
      } else {
        setSwitchMsg(`❌ ${result.error ?? "Error desconocido al cambiar de cuenta."}`);
      }
    } catch (e) {
      setSwitchMsg(`❌ Error: ${String(e)}`);
    } finally {
      setSwitchingId(null);
    }
  }

  const isLoading = alerts.loading || vaultAccounts.loading;

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <h2 style={styles.title}>{es.alertas.titulo}</h2>
        <button
          style={styles.refreshBtn}
          onClick={() => { alerts.refetch(); compare.refetch(); }}
          disabled={isLoading}
        >
          {isLoading ? "⏳" : "↻"} Actualizar
        </button>
      </div>

      {isLoading && <div style={styles.hint}>Cargando alertas…</div>}
      {alerts.error && <div style={styles.error}>{alerts.error}</div>}
      {compare.error && <div style={styles.error}>No se pudo consultar cuota para sugerencias: {compare.error}</div>}
      {muteError && <div style={styles.error}>⚠️ {muteError}</div>}

      {/* Controles para silenciar alertas por X horas (Fase 22) */}
      <div style={styles.muteBar}>
        <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
          {mutedUntil
            ? `🔕 Alertas silenciadas hasta ${new Date(mutedUntil).toLocaleString("es-MX")}`
            : "🔔 Notificaciones de alerta activas"}
        </span>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {[1, 4, 8, 24].map(h => (
            <button key={h} style={styles.muteBtn} onClick={() => setMuteHours(h)}>
              Silenciar {h} h
            </button>
          ))}
          {mutedUntil && (
            <button style={{ ...styles.muteBtn, borderColor: "var(--color-primary)", color: "var(--color-primary)" }} onClick={() => setMuteHours(0)}>
              Reactivar
            </button>
          )}
        </div>
      </div>

      {/* Alerta de consulta fallando / login vencido */}
      {(pollerStatus.data?.state === "error" || pollerStatus.data?.error) && (
        <div style={{ ...styles.alertCard, borderLeft: "4px solid var(--color-danger)" }}>
          <div style={styles.alertTop}>
            <span style={{ ...styles.levelBadge, background: "var(--color-danger)" }}>CONSULTA / LOGIN</span>
            <span style={styles.alertProvider}>poller</span>
          </div>
          <div style={styles.alertBody}>
            <strong>Consulta fallando o sesión vencida:</strong>{" "}
            {pollerStatus.data.error ?? "El poller reportó un error."}
          </div>
        </div>
      )}

      {/* Alertas de fin de suscripción (7 y 1 día) */}
      {subscriptionWarnings.map(w => (
        <div
          key={`sub-${w.account.id}`}
          style={{
            ...styles.alertCard,
            borderLeft: `4px solid ${w.daysLeft <= 1 ? "var(--color-danger)" : "var(--color-warning)"}`,
          }}
        >
          <div style={styles.alertTop}>
            <span style={{ ...styles.levelBadge, background: w.daysLeft <= 1 ? "var(--color-danger)" : "var(--color-warning)" }}>
              {w.daysLeft <= 1 ? "CRÍTICO" : "SUSCRIPCIÓN"}
            </span>
            <span style={styles.alertProvider}>{w.account.provider} · {w.dateStr}</span>
          </div>
          <div style={styles.alertBody}>
            <strong>{w.account.alias}</strong>
            {w.daysLeft < 0
              ? ` — Suscripción vencida hace ${Math.abs(w.daysLeft)} día(s).`
              : w.daysLeft === 0
              ? " — Suscripción vence hoy."
              : w.daysLeft === 1
              ? " — Suscripción vence mañana (1 día)."
              : ` — Suscripción vence en ${w.daysLeft} días.`}
          </div>
        </div>
      ))}

      {switchMsg && (
        <div style={{
          ...styles.switchBanner,
          color: switchMsg.startsWith("✅") ? "var(--color-success)" : "var(--color-danger)",
        }}>
          <span>{switchMsg}</span>
          <button style={styles.closeBtn} onClick={() => setSwitchMsg(null)}>✕</button>
        </div>
      )}

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
          {filteredAlerts.map(alert => {
            const alt = findAlternative(alert);
            return (
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
              {alt && (
                <div style={styles.altRow}>
                  <span style={styles.altText}>
                    💡 Alternativa disponible: <strong>{alt.account.alias}</strong>
                    {alt.percentUsed !== null ? ` (${alt.percentUsed.toFixed(1)}% usado)` : " (sin lecturas recientes)"}
                  </span>
                  <button
                    style={styles.switchBtn}
                    disabled={switchingId === alt.account.id}
                    onClick={() => handleSwitchAlternative(alt.account)}
                  >
                    {switchingId === alt.account.id ? "Cambiando…" : "⚡ Cambiar a esa cuenta"}
                  </button>
                </div>
              )}
            </div>
            );
          })}
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
  switchBanner: { padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, display: "flex", justifyContent: "space-between", alignItems: "center" },
  closeBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-disabled)" },
  altRow: { marginTop: 6, paddingTop: 8, borderTop: "1px solid var(--color-border)", display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, flexWrap: "wrap" as const },
  altText: { fontSize: 12, color: "var(--color-text-secondary)" },
  switchBtn: { padding: "5px 12px", border: "none", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 12, cursor: "pointer", fontWeight: 600 },
  muteBar: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", flexWrap: "wrap" as const },
  muteBtn: { padding: "4px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "var(--color-bg-secondary)", color: "var(--color-text-secondary)", fontSize: 11, cursor: "pointer" },
};

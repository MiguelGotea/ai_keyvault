/**
 * DashboardPage — Dashboard principal de AI KeyVault.
 *
 * FUENTE DE VERDAD: Las cuentas vienen de la BÓVEDA (list_vault_accounts).
 * Los datos de cuota del VPS (get_stats_compare) se cruzan por alias/account_id.
 *
 * Si una cuenta de la bóveda no tiene datos en el VPS aún → "Sin datos de cuota".
 * Si hay datos en el VPS que no están en la bóveda → se ignoran.
 *
 * "Usar en esta PC" → switch_account con el UUID de la bóveda.
 */
import { useEffect, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import QuotaCard from "../components/QuotaCard";
import type { PoolSummary, AccountInfo, SwitchResult } from "../types/api";

export default function DashboardPage() {
  const [pollerStarted, setPollerStarted] = useState(false);
  const [pollerError, setPollerError] = useState<string | null>(null);
  const [switchingAccount, setSwitchingAccount] = useState<string | null>(null);
  const [switchMsg, setSwitchMsg] = useState<string | null>(null);

  // 1. Cuentas de la bóveda (fuente de verdad)
  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  // 2. Datos de cuota del VPS (todos los pools)
  const quota = useCommand<PoolSummary[]>("get_stats_compare", { days: 7 });

  // Lanzar poller al abrir
  useEffect(() => {
    if (!pollerStarted) {
      invoke("start_poller")
        .then(() => setPollerStarted(true))
        .catch((e: unknown) => {
          const msg = String(e);
          if (msg.includes("sidecar") || msg.includes("No such")) {
            setPollerError("Poller no compilado — el monitoreo automático no está activo.");
          }
        });
    }
  }, [pollerStarted]);

  // Autobloqueo: reset en cada interacción
  const throttleRef = { ts: 0 };
  const handleActivity = useCallback(() => {
    const now = Date.now();
    if (now - throttleRef.ts < 30_000) return;
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

  // Cruzar bóveda con VPS: para cada cuenta de la bóveda buscar sus pools en el VPS
  const vaultList = vaultAccounts.data ?? [];
  const quotaPools = quota.data ?? [];

  // Índice de pools por account_id del VPS
  const poolsByVpsId = quotaPools.reduce<Record<string, PoolSummary[]>>((acc, pool) => {
    if (!acc[pool.account_id]) acc[pool.account_id] = [];
    acc[pool.account_id].push(pool);
    return acc;
  }, {});

  async function handleSwitch(account: AccountInfo) {
    setSwitchingAccount(account.id);
    setSwitchMsg(null);
    try {
      const result = await invoke<SwitchResult>("switch_account", {
        accountId: account.id,
        provider: account.provider,
        deviceId: "desktop-dev",
      });
      if (result.success) {
        setSwitchMsg(`✅ Cambiado a '${account.alias}'. ${result.restart_notice ?? ""}`);
      } else {
        setSwitchMsg(`❌ ${result.error ?? "Error desconocido al cambiar de cuenta."}`);
      }
    } catch (e) {
      setSwitchMsg(`❌ Error: ${String(e)}`);
    } finally {
      setSwitchingAccount(null);
    }
  }

  const isLoading = vaultAccounts.loading;
  const hasError = vaultAccounts.error;

  return (
    <div style={styles.page}>
      {/* Header */}
      <div style={styles.pageHeader}>
        <h2 style={styles.pageTitle}>Mis cuentas</h2>
        <div style={styles.headerActions}>
          <button
            style={styles.refreshBtn}
            onClick={() => { vaultAccounts.refetch(); quota.refetch(); }}
            disabled={isLoading || quota.loading}
          >
            {isLoading || quota.loading ? "⏳" : "↻"} Actualizar
          </button>
        </div>
      </div>

      {/* Banner de poller */}
      {pollerError && (
        <div style={styles.warningBanner}>
          ⚠️ {pollerError}
        </div>
      )}

      {/* Mensaje de switch */}
      {switchMsg && (
        <div style={{
          ...styles.infoBar,
          color: switchMsg.startsWith("✅") ? "var(--color-success)" : "var(--color-danger)"
        }}>
          {switchMsg}
          <button style={styles.closeMsgBtn} onClick={() => setSwitchMsg(null)}>✕</button>
        </div>
      )}

      {/* Error */}
      {hasError && (
        <div style={styles.errorBox}>
          <strong>Error al cargar la bóveda:</strong>{" "}
          {hasError.includes("bloqueada") ? "La bóveda está bloqueada. Desbloquea primero desde el inicio." : hasError}
          <button onClick={() => vaultAccounts.refetch()} style={styles.retryBtn}>Reintentar</button>
        </div>
      )}

      {/* Sin cuentas */}
      {!isLoading && !hasError && vaultList.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 48 }}>🗝️</span>
          <p style={{ fontWeight: 600 }}>No hay cuentas en tu bóveda</p>
          <p style={{ fontSize: 12, color: "var(--color-text-disabled)", maxWidth: 320, textAlign: "center" }}>
            Ve a <strong>Mis Cuentas</strong> para agregar tus cuentas de Antigravity.
            Una vez agregadas aparecerán aquí con sus datos de cuota.
          </p>
        </div>
      )}

      {/* Tarjetas de cuentas de la bóveda */}
      {vaultList.map(account => {
        // Buscar pools del VPS que correspondan a esta cuenta (por alias)
        const pools = poolsByVpsId[account.alias] ?? [];
        const hasAlert = pools.some(p => p.current_percent_used >= 80);
        const isSwitching = switchingAccount === account.id;

        return (
          <section key={account.id} style={styles.accountSection}>
            <div style={styles.accountHeader}>
              <div>
                <span style={styles.accountProvider}>
                  {account.provider === "agy" ? "ANTIGRAVITY" : "CODEX"}
                </span>
                <span style={styles.accountAlias}>{account.alias}</span>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                {hasAlert && <span style={styles.alertDot}>⚠️</span>}
                <button
                  style={styles.switchBtn}
                  onClick={() => handleSwitch(account)}
                  disabled={isSwitching}
                  title="Escribir esta credencial en el Credential Manager de Windows"
                >
                  {isSwitching ? "Cambiando…" : "⚡ Usar en esta PC"}
                </button>
              </div>
            </div>

            {/* Pools de cuota del VPS */}
            {pools.length > 0 ? (
              <div style={styles.cardGrid}>
                {pools.map(pool => (
                  <QuotaCard key={pool.pool} pool={pool} />
                ))}
              </div>
            ) : (
              <div style={styles.noDataMsg}>
                <span style={{ fontSize: 20 }}>📡</span>
                <span>Sin datos de cuota aún — el poller los recopilará automáticamente.</span>
              </div>
            )}
          </section>
        );
      })}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 20 },
  pageHeader: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  pageTitle: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  headerActions: { display: "flex", gap: 8 },
  refreshBtn: { padding: "7px 14px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  warningBanner: { padding: "10px 16px", background: "#FFFBEB", border: "1px solid #FDE68A", borderRadius: "var(--radius-md)", fontSize: 12, color: "#92400E" },
  infoBar: { padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 },
  closeMsgBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-disabled)", padding: "0 2px" },
  errorBox: { padding: "12px 16px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 13, color: "#991B1B", display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" as const },
  retryBtn: { padding: "5px 10px", border: "1px solid #FECACA", borderRadius: "var(--radius-sm)", background: "white", color: "#991B1B", fontSize: 12, cursor: "pointer" },
  empty: { display: "flex", flexDirection: "column" as const, alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  accountSection: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: "18px 20px", display: "flex", flexDirection: "column" as const, gap: 14 },
  accountHeader: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12 },
  accountProvider: { fontSize: 10, fontWeight: 700, color: "var(--color-primary)", textTransform: "uppercase" as const, letterSpacing: "0.08em", marginRight: 10 },
  accountAlias: { fontSize: 14, color: "var(--color-text-secondary)", fontWeight: 500 },
  alertDot: { fontSize: 14 },
  switchBtn: { padding: "6px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 12, cursor: "pointer", fontWeight: 600 },
  cardGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 14 },
  noDataMsg: { display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 12, color: "var(--color-text-disabled)" },
};

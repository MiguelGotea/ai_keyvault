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
import { useEffect, useCallback, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import AccountDetailModal from "../components/AccountDetailModal";
import type { PoolSummary, AccountInfo, PollerStatus, SwitchResult } from "../types/api";

function formatRelativeAge(isoDate: string | null | undefined): string {
  if (!isoDate) return "sin lecturas";
  const ts = new Date(isoDate).getTime();
  if (Number.isNaN(ts)) return "fecha inválida";
  const diffSec = Math.max(0, Math.floor((Date.now() - ts) / 1000));
  if (diffSec < 60) return `hace ${diffSec} s`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `hace ${diffMin} min`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `hace ${diffHr} h`;
  return `hace ${Math.floor(diffHr / 24)} d`;
}

export default function DashboardPage() {
  const [pollerStarted, setPollerStarted] = useState(false);
  const [pollerError, setPollerError] = useState<string | null>(null);
  const [autolockError, setAutolockError] = useState<string | null>(null);
  const [switchingAccount, setSwitchingAccount] = useState<string | null>(null);
  const [switchMsg, setSwitchMsg] = useState<string | null>(null);
  const [detailAccount, setDetailAccount] = useState<AccountInfo | null>(null);

  // 1. Cuentas de la bóveda (fuente de verdad)
  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  // 2. Datos de cuota del VPS (todos los pools)
  const quota = useCommand<PoolSummary[]>("get_stats_compare", { days: 7 });

  // 3. Estado del poller (IPC)
  const pollerStatus = useCommand<PollerStatus>("get_poller_status", {});

  // Lanzar poller al abrir (sin silenciar ningún error)
  useEffect(() => {
    if (!pollerStarted) {
      invoke("start_poller")
        .then(() => {
          setPollerStarted(true);
          setPollerError(null);
          pollerStatus.refetch();
        })
        .catch((e: unknown) => {
          setPollerError(`No se pudo iniciar el poller: ${String(e)}`);
          pollerStatus.refetch();
        });
    }
  }, [pollerStarted, pollerStatus.refetch]);

  useEffect(() => {
    const timer = setInterval(() => pollerStatus.refetch(), 15_000);
    return () => clearInterval(timer);
  }, [pollerStatus.refetch]);

  // Autobloqueo: reset en cada interacción
  const throttleRef = useRef(0);
  const handleActivity = useCallback(() => {
    const now = Date.now();
    if (now - throttleRef.current < 30_000) return;
    throttleRef.current = now;
    const minutes = parseInt(localStorage.getItem("autolock_minutes") ?? "0");
    if (minutes > 0) {
      invoke("reset_autolock_timer", { minutes }).catch((e: unknown) => {
        setAutolockError(`Error al reiniciar autobloqueo: ${String(e)}`);
      });
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

      {/* Estado del poller y errores visibles */}
      <div style={styles.pollerBar}>
        <span>
          <strong>Poller:</strong>{" "}
          {pollerStatus.data
            ? pollerStatus.data.state === "running"
              ? "Corriendo"
              : pollerStatus.data.state === "paused"
              ? "Pausado"
              : pollerStatus.data.state === "error"
              ? "Error"
              : "Detenido"
            : "Consultando…"}
          {" · "}
          Último envío: {formatRelativeAge(pollerStatus.data?.last_poll_at)}
        </span>
      </div>

      {(pollerError || pollerStatus.error || pollerStatus.data?.error) && (
        <div style={styles.warningBanner}>
          ⚠️ {pollerError ?? pollerStatus.error ?? pollerStatus.data?.error}
        </div>
      )}

      {quota.error && (
        <div style={styles.errorBox}>
          <strong>Error al consultar cuota del VPS:</strong> {quota.error}
          <button onClick={() => quota.refetch()} style={styles.retryBtn}>Reintentar</button>
        </div>
      )}

      {autolockError && (
        <div style={styles.warningBanner}>
          ⚠️ {autolockError}
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

      {/* Lista compacta de cuentas en una sola línea */}
      {vaultList.length > 0 && (
        <div style={styles.compactList}>
          {vaultList.map(account => {
            // D-02: cruzar pools del VPS por UUID opaco de la bóveda (nunca por alias)
            const pools = poolsByVpsId[account.id] ?? [];
            const primaryPool = pools.reduce<PoolSummary | null>(
              (max, p) => (!max || p.current_percent_used > max.current_percent_used ? p : max),
              null
            );
            const pct = primaryPool ? Math.round(primaryPool.current_percent_used) : null;
            const pctColor =
              pct === null
                ? "var(--color-text-disabled)"
                : pct >= 95
                ? "var(--color-danger)"
                : pct >= 80
                ? "var(--color-warning)"
                : "var(--color-success)";
            const nextReset = primaryPool?.reset_time
              ? new Date(primaryPool.reset_time).toLocaleString("es-MX", {
                  month: "short",
                  day: "numeric",
                  hour: "2-digit",
                  minute: "2-digit",
                })
              : "—";
            const isSwitching = switchingAccount === account.id;

            return (
              <div key={account.id} style={styles.compactRow}>
                <span style={styles.accountProvider}>
                  {account.provider === "agy" ? "AGY" : "CODEX"}
                </span>
                <div style={styles.identityCol}>
                  <span style={styles.accountAlias}>{account.alias}</span>
                  {account.email && <span style={styles.accountEmail}>{account.email}</span>}
                </div>
                <span style={{ ...styles.pctPill, color: pctColor, borderColor: pctColor }}>
                  {pct !== null ? `${pct}% usado` : "Sin datos"}
                </span>
                <span style={styles.resetCol} title="Próximo reinicio">
                  ↻ {nextReset}
                </span>
                <div style={styles.rowActions}>
                  <button
                    style={styles.detailBtn}
                    onClick={() => setDetailAccount(account)}
                    title="Ver detalle de pools, burn rate y última lectura"
                  >
                    Detalles
                  </button>
                  <button
                    style={styles.switchBtn}
                    onClick={() => handleSwitch(account)}
                    disabled={isSwitching}
                    title="Escribir esta credencial en esta PC"
                  >
                    {isSwitching ? "Cambiando…" : "⚡ Usar"}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Modal de detalle con 6 pestañas (Fase 24) */}
      {detailAccount && (
        <AccountDetailModal
          account={detailAccount}
          pools={poolsByVpsId[detailAccount.id] ?? []}
          onClose={() => setDetailAccount(null)}
          onSwitch={handleSwitch}
          switching={switchingAccount === detailAccount.id}
        />
      )}
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
  pollerBar: { padding: "8px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 12, color: "var(--color-text-secondary)" },
  lastReadTag: { fontSize: 11, color: "var(--color-text-disabled)", marginLeft: 4 },
  infoBar: { padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 },
  closeMsgBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-disabled)", padding: "0 2px" },
  errorBox: { padding: "12px 16px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 13, color: "#991B1B", display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" as const },
  retryBtn: { padding: "5px 10px", border: "1px solid #FECACA", borderRadius: "var(--radius-sm)", background: "white", color: "#991B1B", fontSize: 12, cursor: "pointer" },
  empty: { display: "flex", flexDirection: "column" as const, alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  compactList: { display: "flex", flexDirection: "column" as const, gap: 8 },
  compactRow: { display: "flex", alignItems: "center", gap: 14, padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", flexWrap: "wrap" as const },
  identityCol: { display: "flex", alignItems: "baseline", gap: 8, flex: 1, minWidth: 180 },
  accountEmail: { fontSize: 12, color: "var(--color-text-disabled)", fontFamily: "var(--font-mono)" },
  pctPill: { fontSize: 12, fontWeight: 700, padding: "2px 10px", borderRadius: 999, border: "1px solid" },
  resetCol: { fontSize: 12, color: "var(--color-text-secondary)", minWidth: 120 },
  rowActions: { display: "flex", gap: 6, marginLeft: "auto" },
  detailBtn: { padding: "5px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg-secondary)", color: "var(--color-text-secondary)", fontSize: 12, cursor: "pointer" },
  modalOverlay: { position: "fixed" as const, inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000 },
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 22, width: "min(680px, 92vw)", maxHeight: "85vh", overflowY: "auto" as const, display: "flex", flexDirection: "column" as const, gap: 16 },
  modalHeader: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  accountProvider: { fontSize: 10, fontWeight: 700, color: "var(--color-primary)", textTransform: "uppercase" as const, letterSpacing: "0.08em", marginRight: 6 },
  accountAlias: { fontSize: 14, color: "var(--color-text)", fontWeight: 600 },
  alertDot: { fontSize: 14 },
  switchBtn: { padding: "6px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 12, cursor: "pointer", fontWeight: 600 },
  cardGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 14 },
  noDataMsg: { display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 12, color: "var(--color-text-disabled)" },
};

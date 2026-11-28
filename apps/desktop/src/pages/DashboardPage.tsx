/**
 * DashboardPage — Dashboard principal de AI KeyVault.
 *
 * Separado por pestañas de proveedor (Antigravity / Codex) en formato tabla ordenable.
 * FUENTE DE VERDAD: Las cuentas vienen de la BÓVEDA (list_vault_accounts).
 * Los datos de cuota del VPS (get_stats_compare) se cruzan por UUID opaco.
 */
import { useEffect, useCallback, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import { useTheme } from "../hooks/useTheme";
import AccountDetailModal from "../components/AccountDetailModal";
import { isNoAgentOnPc } from "../utils/settings";
import agyLight from "../assets/agy-light.png";
import agyDark from "../assets/agy-dark.png";
import codexLight from "../assets/codex-light.png";
import codexDark from "../assets/codex-dark.png";
import type { PoolSummary, AccountInfo, PollerStatus, SwitchResult } from "../types/api";

type ProviderTab = "agy" | "codex";
type SortField = "name" | "email" | "gemini" | "claude" | "quota";
type SortDir = "asc" | "desc";

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

function formatDateOnly(isoDate: string | null | undefined): string {
  if (!isoDate) return "—";
  const d = new Date(isoDate);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleDateString("es-MX", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

function getPctColor(pct: number | null): string {
  if (pct === null) return "var(--color-text-disabled)";
  if (pct >= 95) return "var(--color-danger)";
  if (pct >= 80) return "var(--color-warning)";
  return "var(--color-success)";
}

function QuotaCell({ pool }: { pool: PoolSummary | undefined }) {
  if (!pool) {
    return <span style={styles.noDataText}>Sin datos</span>;
  }
  const pct = Math.round(pool.current_percent_used);
  const color = getPctColor(pct);
  const dateStr = formatDateOnly(pool.reset_time);

  return (
    <div style={styles.quotaCell}>
      <span style={{ ...styles.pctPill, color, borderColor: color }}>
        {pct}% usado
      </span>
      <span style={styles.resetDate} title="Fecha de renovación">
        ↻ {dateStr}
      </span>
    </div>
  );
}

export default function DashboardPage() {
  const theme = useTheme();
  const [activeTab, setActiveTab] = useState<ProviderTab>("agy");
  const [sortField, setSortField] = useState<SortField>("name");
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const [pollerStarted, setPollerStarted] = useState(false);
  const [pollerError, setPollerError] = useState<string | null>(null);
  const [autolockError, setAutolockError] = useState<string | null>(null);
  const [switchingAccount, setSwitchingAccount] = useState<string | null>(null);
  const [switchMsg, setSwitchMsg] = useState<string | null>(null);
  const [canUndoSwitch, setCanUndoSwitch] = useState(false);
  const [undoingSwitch, setUndoingSwitch] = useState(false);
  const [detailAccount, setDetailAccount] = useState<AccountInfo | null>(null);
  const noAgentOnPc = isNoAgentOnPc();

  // 1. Cuentas de la bóveda (fuente de verdad)
  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  // 2. Datos de cuota del VPS (todos los pools)
  const quota = useCommand<PoolSummary[]>("get_stats_compare", { days: 7 });

  // 3. Estado del poller (IPC)
  const pollerStatus = useCommand<PollerStatus>("get_poller_status", {});

  // Lanzar poller al abrir salvo que esté activo "Sin agente en esta PC" (Fase 26)
  useEffect(() => {
    if (noAgentOnPc) return;
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
  }, [noAgentOnPc, pollerStarted, pollerStatus.refetch]);

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

  const vaultList = vaultAccounts.data ?? [];
  const quotaPools = quota.data ?? [];

  const agyAccounts = vaultList.filter(a => a.provider === "agy");
  const codexAccounts = vaultList.filter(a => a.provider === "codex");

  // Índice de pools por account_id del VPS
  const poolsByVpsId = useMemo(
    () =>
      quotaPools.reduce<Record<string, PoolSummary[]>>((acc, pool) => {
        if (!acc[pool.account_id]) acc[pool.account_id] = [];
        acc[pool.account_id].push(pool);
        return acc;
      }, {}),
    [quotaPools]
  );

  function handleSort(field: SortField) {
    if (sortField === field) {
      setSortDir(d => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortField(field);
      setSortDir("asc");
    }
  }

  function sortIndicator(field: SortField): string {
    if (sortField !== field) return "";
    return sortDir === "asc" ? " ▲" : " ▼";
  }

  const visibleAccounts = useMemo(() => {
    const base = activeTab === "agy" ? [...agyAccounts] : [...codexAccounts];
    const mult = sortDir === "asc" ? 1 : -1;

    base.sort((a, b) => {
      if (sortField === "name") {
        return mult * a.alias.localeCompare(b.alias, "es", { sensitivity: "base" });
      }
      if (sortField === "email") {
        return mult * (a.email ?? "").localeCompare(b.email ?? "", "es", { sensitivity: "base" });
      }
      const poolsA = poolsByVpsId[a.id] ?? [];
      const poolsB = poolsByVpsId[b.id] ?? [];

      if (sortField === "gemini") {
        const pa = poolsA.find(p => p.pool === "gemini-weekly")?.current_percent_used ?? -1;
        const pb = poolsB.find(p => p.pool === "gemini-weekly")?.current_percent_used ?? -1;
        return mult * (pa - pb);
      }
      if (sortField === "claude") {
        const pa =
          poolsA.find(p => p.pool === "claude-gpt-weekly" || p.pool === "3p-weekly")
            ?.current_percent_used ?? -1;
        const pb =
          poolsB.find(p => p.pool === "claude-gpt-weekly" || p.pool === "3p-weekly")
            ?.current_percent_used ?? -1;
        return mult * (pa - pb);
      }
      // "quota" (Codex)
      const pa = (poolsA.find(p => p.pool === "default") ?? poolsA[0])?.current_percent_used ?? -1;
      const pb = (poolsB.find(p => p.pool === "default") ?? poolsB[0])?.current_percent_used ?? -1;
      return mult * (pa - pb);
    });

    return base;
  }, [activeTab, agyAccounts, codexAccounts, poolsByVpsId, sortField, sortDir]);

  async function handleSwitch(account: AccountInfo) {
    if (noAgentOnPc) return;
    setSwitchingAccount(account.id);
    setSwitchMsg(null);
    setCanUndoSwitch(false);
    try {
      const result = await invoke<SwitchResult>("switch_account", {
        accountId: account.id,
        provider: account.provider,
        deviceId: "desktop-dev",
      });
      if (result.success) {
        setSwitchMsg(`✅ Cambiado a '${account.alias}'. ${result.restart_notice ?? ""}`);
        setCanUndoSwitch(Boolean(result.can_undo));
        quota.refetch();
      } else {
        setSwitchMsg(`❌ ${result.error ?? "Error desconocido al cambiar de cuenta."}`);
      }
    } catch (e) {
      setSwitchMsg(`❌ Error: ${String(e)}`);
    } finally {
      setSwitchingAccount(null);
    }
  }

  async function handleUndoSwitch() {
    setUndoingSwitch(true);
    try {
      const result = await invoke<SwitchResult>("undo_last_switch", {
        deviceId: "desktop-dev",
      });
      setCanUndoSwitch(false);
      setSwitchMsg(`↩ Cambio deshecho (restaurado '${result.account_id}'). ${result.restart_notice ?? ""}`);
    } catch (e) {
      setSwitchMsg(`❌ No se pudo deshacer: ${String(e)}`);
    } finally {
      setUndoingSwitch(false);
    }
  }

  const isLoading = vaultAccounts.loading;
  const hasError = vaultAccounts.error;
  const agyIcon = theme === "dark" ? agyDark : agyLight;
  const codexIcon = theme === "dark" ? codexDark : codexLight;

  return (
    <div style={styles.page}>
      {/* Header */}
      <div style={styles.pageHeader}>
        <h2 style={styles.pageTitle}>Inicio — Cuotas y cuentas</h2>
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

      {/* Pestañas por proveedor */}
      <div style={styles.tabsRow}>
        <button
          style={{
            ...styles.tabBtn,
            ...(activeTab === "agy" ? styles.tabBtnActive : {}),
          }}
          onClick={() => { setActiveTab("agy"); setSortField("name"); setSortDir("asc"); }}
        >
          <img src={agyIcon} alt="Antigravity" style={styles.tabIcon} />
          <span>Antigravity ({agyAccounts.length})</span>
        </button>
        <button
          style={{
            ...styles.tabBtn,
            ...(activeTab === "codex" ? styles.tabBtnActive : {}),
          }}
          onClick={() => { setActiveTab("codex"); setSortField("name"); setSortDir("asc"); }}
        >
          <img src={codexIcon} alt="Codex" style={styles.tabIcon} />
          <span>Codex ({codexAccounts.length})</span>
        </button>
      </div>

      {/* Estado del poller y errores visibles */}
      <div style={styles.pollerBar}>
        <span>
          <strong>Poller:</strong>{" "}
          {noAgentOnPc
            ? "Desactivado (sin agente en esta PC)"
            : pollerStatus.data
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
          color: switchMsg.startsWith("❌") ? "var(--color-danger)" : "var(--color-success)"
        }}>
          <span style={{ flex: 1 }}>{switchMsg}</span>
          {canUndoSwitch && (
            <button
              style={{ ...styles.detailBtn, marginRight: 8 }}
              onClick={handleUndoSwitch}
              disabled={undoingSwitch}
            >
              {undoingSwitch ? "Deshaciendo…" : "↩ Deshacer"}
            </button>
          )}
          <button style={styles.closeMsgBtn} onClick={() => { setSwitchMsg(null); setCanUndoSwitch(false); }}>✕</button>
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

      {/* Sin cuentas para el proveedor */}
      {!isLoading && !hasError && visibleAccounts.length === 0 && (
        <div style={styles.empty}>
          <img
            src={activeTab === "agy" ? agyIcon : codexIcon}
            alt={activeTab}
            style={{ width: 48, height: 48, objectFit: "contain" }}
          />
          <p style={{ fontWeight: 600 }}>
            No hay cuentas de {activeTab === "agy" ? "Antigravity" : "Codex"} en tu bóveda
          </p>
          <p style={{ fontSize: 12, color: "var(--color-text-disabled)", maxWidth: 340, textAlign: "center" }}>
            Ve a <strong>Mis Cuentas</strong> para agregar tus cuentas de{" "}
            {activeTab === "agy" ? "Antigravity" : "Codex"}.
            Una vez agregadas aparecerán aquí con sus datos de cuota.
          </p>
        </div>
      )}

      {/* Tabla de Antigravity */}
      {activeTab === "agy" && visibleAccounts.length > 0 && (
        <div style={styles.tableWrapper}>
          <table style={styles.table}>
            <thead>
              <tr style={styles.theadRow}>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("name")}>
                  Nombre{sortIndicator("name")}
                </th>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("email")}>
                  Correo{sortIndicator("email")}
                </th>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("gemini")}>
                  Gemini Weekly{sortIndicator("gemini")}
                </th>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("claude")}>
                  Claude / GPT Weekly{sortIndicator("claude")}
                </th>
                <th style={{ ...styles.th, textAlign: "right" }}>Acciones</th>
              </tr>
            </thead>
            <tbody>
              {visibleAccounts.map(account => {
                const pools = poolsByVpsId[account.id] ?? [];
                const geminiWeekly = pools.find(p => p.pool === "gemini-weekly");
                const claudeWeekly = pools.find(
                  p => p.pool === "claude-gpt-weekly" || p.pool === "3p-weekly"
                );
                const isSwitching = switchingAccount === account.id;

                return (
                  <tr key={account.id} style={styles.tr}>
                    <td style={styles.td}>
                      <span style={styles.accountAlias}>{account.alias}</span>
                    </td>
                    <td style={styles.td}>
                      <span style={styles.accountEmail}>{account.email || "—"}</span>
                    </td>
                    <td style={styles.td}>
                      <QuotaCell pool={geminiWeekly} />
                    </td>
                    <td style={styles.td}>
                      <QuotaCell pool={claudeWeekly} />
                    </td>
                    <td style={{ ...styles.td, textAlign: "right" }}>
                      <div style={styles.rowActions}>
                        <button
                          style={styles.detailBtn}
                          onClick={() => setDetailAccount(account)}
                          title="Ver detalle de pools, burn rate y última lectura"
                        >
                          Detalles
                        </button>
                        <button
                          style={{
                            ...styles.switchBtn,
                            opacity: noAgentOnPc ? 0.5 : 1,
                            cursor: noAgentOnPc ? "not-allowed" : "pointer",
                          }}
                          onClick={() => handleSwitch(account)}
                          disabled={isSwitching || noAgentOnPc}
                          title={noAgentOnPc ? "Modo sin agente en esta PC activo (solo consulta)" : "Escribir esta credencial en esta PC"}
                        >
                          {isSwitching ? "Cambiando…" : "⚡ Usar"}
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Tabla de Codex */}
      {activeTab === "codex" && visibleAccounts.length > 0 && (
        <div style={styles.tableWrapper}>
          <table style={styles.table}>
            <thead>
              <tr style={styles.theadRow}>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("name")}>
                  Nombre{sortIndicator("name")}
                </th>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("email")}>
                  Correo{sortIndicator("email")}
                </th>
                <th style={{ ...styles.th, ...styles.thSortable }} onClick={() => handleSort("quota")}>
                  Cuota{sortIndicator("quota")}
                </th>
                <th style={{ ...styles.th, textAlign: "right" }}>Acciones</th>
              </tr>
            </thead>
            <tbody>
              {visibleAccounts.map(account => {
                const pools = poolsByVpsId[account.id] ?? [];
                const codexPool = pools.find(p => p.pool === "default") ?? pools[0];
                const isSwitching = switchingAccount === account.id;

                return (
                  <tr key={account.id} style={styles.tr}>
                    <td style={styles.td}>
                      <span style={styles.accountAlias}>{account.alias}</span>
                    </td>
                    <td style={styles.td}>
                      <span style={styles.accountEmail}>{account.email || "—"}</span>
                    </td>
                    <td style={styles.td}>
                      <QuotaCell pool={codexPool} />
                    </td>
                    <td style={{ ...styles.td, textAlign: "right" }}>
                      <div style={styles.rowActions}>
                        <button
                          style={styles.detailBtn}
                          onClick={() => setDetailAccount(account)}
                          title="Ver detalle de pools, burn rate y última lectura"
                        >
                          Detalles
                        </button>
                        <button
                          style={{
                            ...styles.switchBtn,
                            opacity: noAgentOnPc ? 0.5 : 1,
                            cursor: noAgentOnPc ? "not-allowed" : "pointer",
                          }}
                          onClick={() => handleSwitch(account)}
                          disabled={isSwitching || noAgentOnPc}
                          title={noAgentOnPc ? "Modo sin agente en esta PC activo (solo consulta)" : "Escribir esta credencial en esta PC"}
                        >
                          {isSwitching ? "Cambiando…" : "⚡ Usar"}
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
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
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 16 },
  pageHeader: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  pageTitle: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  headerActions: { display: "flex", gap: 8 },
  refreshBtn: { padding: "7px 14px", border: "1px solid var(--color-btn-border)", borderRadius: "var(--radius-md)", background: "var(--color-btn-bg)", color: "var(--color-btn-text)", fontSize: 13, cursor: "pointer" },
  tabsRow: { display: "flex", gap: 8, borderBottom: "1px solid var(--color-border)", paddingBottom: 10 },
  tabBtn: {
    display: "inline-flex",
    alignItems: "center",
    gap: 8,
    padding: "8px 16px",
    border: "1px solid var(--color-btn-border)",
    borderRadius: "var(--radius-md)",
    background: "var(--color-btn-bg)",
    color: "var(--color-btn-text)",
    fontSize: 13,
    fontWeight: 500,
    cursor: "pointer",
  },
  tabBtnActive: {
    background: "var(--color-primary)",
    color: "var(--color-on-primary)",
    borderColor: "var(--color-primary)",
    fontWeight: 700,
  },
  tabIcon: { width: 18, height: 18, objectFit: "contain" },
  warningBanner: { padding: "10px 16px", background: "var(--color-bg)", border: "1px solid var(--color-secondary)", borderRadius: "var(--radius-md)", fontSize: 12, color: "var(--color-text)" },
  pollerBar: { padding: "8px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 12, color: "var(--color-text-secondary)" },
  infoBar: { padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 },
  closeMsgBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-disabled)", padding: "0 2px" },
  errorBox: { padding: "12px 16px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 13, color: "#991B1B", display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" as const },
  retryBtn: { padding: "5px 10px", border: "1px solid #FECACA", borderRadius: "var(--radius-sm)", background: "white", color: "#991B1B", fontSize: 12, cursor: "pointer" },
  empty: { display: "flex", flexDirection: "column" as const, alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  tableWrapper: {
    background: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    overflowX: "auto",
  },
  table: {
    width: "100%",
    borderCollapse: "collapse",
    fontSize: 13,
  },
  theadRow: {
    borderBottom: "1px solid var(--color-border)",
    background: "var(--color-bg-tertiary)",
  },
  th: {
    textAlign: "left" as const,
    padding: "10px 14px",
    fontSize: 11,
    fontWeight: 700,
    textTransform: "uppercase" as const,
    letterSpacing: "0.05em",
    color: "var(--color-text-secondary)",
    whiteSpace: "nowrap" as const,
  },
  thSortable: {
    cursor: "pointer",
    userSelect: "none" as const,
  },
  tr: {
    borderBottom: "1px solid var(--color-border-subtle)",
  },
  td: {
    padding: "11px 14px",
    verticalAlign: "middle" as const,
    color: "var(--color-text)",
  },
  accountAlias: { fontSize: 14, color: "var(--color-text)", fontWeight: 600 },
  accountEmail: { fontSize: 12, color: "var(--color-text-disabled)", fontFamily: "var(--font-mono)" },
  quotaCell: { display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" as const },
  noDataText: { fontSize: 12, color: "var(--color-text-disabled)" },
  pctPill: { fontSize: 12, fontWeight: 700, padding: "2px 10px", borderRadius: 999, border: "1px solid", whiteSpace: "nowrap" as const },
  resetDate: { fontSize: 12, color: "var(--color-text-secondary)", whiteSpace: "nowrap" as const },
  rowActions: { display: "inline-flex", gap: 6, justifyContent: "flex-end" },
  detailBtn: { padding: "5px 10px", border: "1px solid var(--color-btn-border)", borderRadius: "var(--radius-md)", background: "var(--color-btn-bg)", color: "var(--color-btn-text)", fontSize: 12, cursor: "pointer" },
  switchBtn: { padding: "6px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "var(--color-on-primary)", fontSize: 12, cursor: "pointer", fontWeight: 600 },
};

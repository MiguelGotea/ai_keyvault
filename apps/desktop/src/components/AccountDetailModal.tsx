/**
 * AccountDetailModal — Detalle de cuenta con 6 pestañas (Fase 24):
 * Resumen, Cuota y cooldown, Estadísticas, Chat, Credenciales e Historial.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import QuotaCard from "./QuotaCard";
import type {
  AccountInfo,
  ChatSession,
  HeatmapCell,
  PingRecord,
  PoolSummary,
  SwitchRecord,
} from "../types/api";

type TabId = "resumen" | "cuota" | "estadisticas" | "chat" | "credenciales" | "historial";

interface AccountCredentials {
  id: string;
  alias: string;
  email: string;
  password: string | null;
  notes: string | null;
  details: string | null;
  subscription_ends_at: string | null;
}

interface Props {
  account: AccountInfo;
  pools: PoolSummary[];
  onClose: () => void;
  onSwitch: (account: AccountInfo) => void;
  switching: boolean;
}

const TABS: { id: TabId; label: string }[] = [
  { id: "resumen", label: "Resumen" },
  { id: "cuota", label: "Cuota y cooldown" },
  { id: "estadisticas", label: "Estadísticas" },
  { id: "chat", label: "Chat" },
  { id: "credenciales", label: "Credenciales" },
  { id: "historial", label: "Historial" },
];

export default function AccountDetailModal({
  account,
  pools,
  onClose,
  onSwitch,
  switching,
}: Props) {
  const [tab, setTab] = useState<TabId>("resumen");
  const [copyMsg, setCopyMsg] = useState<string | null>(null);
  const [showPass, setShowPass] = useState(false);

  const summary = useCommand<PoolSummary[]>(
    "get_stats_summary",
    { accountId: account.id, days: 7 },
    { enabled: tab === "estadisticas" }
  );
  const heatmap = useCommand<HeatmapCell[]>(
    "get_stats_heatmap",
    { accountId: account.id, days: 7 },
    { enabled: tab === "estadisticas" }
  );
  const chats = useCommand<ChatSession[]>(
    "get_chats",
    { accountId: account.id },
    { enabled: tab === "chat" }
  );
  const credentials = useCommand<AccountCredentials>(
    "get_account_credentials",
    { accountUuid: account.id },
    { enabled: tab === "credenciales" }
  );
  const switches = useCommand<SwitchRecord[]>(
    "get_switch_history",
    { limit: 30 },
    { enabled: tab === "historial" }
  );
  const pings = useCommand<PingRecord[]>(
    "get_pings",
    { accountId: account.id },
    { enabled: tab === "historial" }
  );

  const latestRead = pools.reduce<string | null>((latest, p) => {
    if (!latest) return p.last_read_at;
    return new Date(p.last_read_at) > new Date(latest) ? p.last_read_at : latest;
  }, null);

  async function copySecret(value: string | null | undefined, label: string) {
    if (!value) {
      setCopyMsg(`⚠️ No hay ${label.toLowerCase()} guardado.`);
      return;
    }
    try {
      await navigator.clipboard.writeText(value);
      await invoke("schedule_clipboard_clear", { seconds: 30 });
      setCopyMsg(`📋 ${label} copiado (se limpiará en 30 s).`);
    } catch (e) {
      setCopyMsg(`❌ Error al copiar: ${String(e)}`);
    }
  }

  const accountSwitches = (switches.data ?? []).filter(
    s => s.to_account_id === account.id || s.from_account_id === account.id
  );

  return (
    <div
      style={styles.overlay}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={styles.modal}>
        {/* Header */}
        <div style={styles.header}>
          <div>
            <span style={styles.provider}>
              {account.provider === "agy" ? "ANTIGRAVITY" : "CODEX"}
            </span>
            <strong style={styles.alias}>{account.alias}</strong>
            {account.email && (
              <div style={styles.email}>{account.email}</div>
            )}
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <button
              style={styles.switchBtn}
              disabled={switching}
              onClick={() => onSwitch(account)}
            >
              {switching ? "Cambiando…" : "⚡ Usar en esta PC"}
            </button>
            <button style={styles.closeBtn} onClick={onClose}>✕</button>
          </div>
        </div>

        {/* Pestañas */}
        <div style={styles.tabs}>
          {TABS.map(t => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              style={{
                ...styles.tabBtn,
                ...(tab === t.id ? styles.tabBtnActive : {}),
              }}
            >
              {t.label}
            </button>
          ))}
        </div>

        {/* Contenido */}
        <div style={styles.body}>
          {tab === "resumen" && (
            <div style={styles.kvList}>
              <div style={styles.kvRow}><span>Proveedor</span><strong>{account.provider}</strong></div>
              <div style={styles.kvRow}><span>Alias</span><strong>{account.alias}</strong></div>
              <div style={styles.kvRow}><span>Correo</span><strong>{account.email ?? "—"}</strong></div>
              <div style={styles.kvRow}>
                <span>Última lectura</span>
                <strong>{latestRead ? new Date(latestRead).toLocaleString("es-MX") : "Sin lecturas"}</strong>
              </div>
              <div style={styles.kvRow}>
                <span>Fin de suscripción</span>
                <strong>{account.subscription_ends_at ?? "No configurado"}</strong>
              </div>
              <div style={styles.kvRow}>
                <span>Guardada el</span>
                <strong>{new Date(account.created_at).toLocaleString("es-MX")}</strong>
              </div>
            </div>
          )}

          {tab === "cuota" && (
            pools.length > 0 ? (
              <div style={styles.cardGrid}>
                {pools.map(pool => (
                  <QuotaCard key={pool.pool} pool={pool} />
                ))}
              </div>
            ) : (
              <div style={styles.empty}>Sin lecturas de cuota aún para esta cuenta.</div>
            )
          )}

          {tab === "estadisticas" && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {(summary.loading || heatmap.loading) && <div style={styles.hint}>Cargando estadísticas…</div>}
              {summary.error && <div style={styles.error}>{summary.error}</div>}
              {heatmap.error && <div style={styles.error}>{heatmap.error}</div>}
              {summary.data && summary.data.length > 0 ? (
                <div style={styles.kvList}>
                  {summary.data.map(p => (
                    <div key={p.pool} style={styles.kvRow}>
                      <span>{p.pool}</span>
                      <strong>
                        {p.current_percent_used.toFixed(1)}% · Burn {p.burn_rate_pct_per_hour.toFixed(2)}%/h
                      </strong>
                    </div>
                  ))}
                </div>
              ) : (
                !summary.loading && <div style={styles.empty}>Sin suficientes lecturas para estadísticas.</div>
              )}
              {heatmap.data && (
                <div style={styles.hint}>
                  Franjas con actividad en los últimos 7 días: {heatmap.data.length}
                </div>
              )}
            </div>
          )}

          {tab === "chat" && (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {chats.loading && <div style={styles.hint}>Cargando chats…</div>}
              {chats.error && <div style={styles.error}>{chats.error}</div>}
              {chats.data && chats.data.length === 0 && (
                <div style={styles.empty}>Sin sesiones de chat sincronizadas.</div>
              )}
              {chats.data?.map(c => (
                <div key={c.id} style={styles.listRow}>
                  <strong>{c.title ?? "(Sin título)"}</strong>
                  <span style={styles.hint}>
                    {new Date(c.updated_at).toLocaleString("es-MX")}
                  </span>
                </div>
              ))}
            </div>
          )}

          {tab === "credenciales" && (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {credentials.loading && <div style={styles.hint}>Descifrando credenciales…</div>}
              {credentials.error && <div style={styles.error}>{credentials.error}</div>}
              {copyMsg && <div style={styles.hint}>{copyMsg}</div>}
              {credentials.data && (
                <div style={styles.kvList}>
                  <div style={styles.kvRow}>
                    <span>Correo</span>
                    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <strong>{credentials.data.email}</strong>
                      <button style={styles.smallBtn} onClick={() => copySecret(credentials.data?.email, "Correo")}>
                        📋 Copiar
                      </button>
                    </div>
                  </div>
                  <div style={styles.kvRow}>
                    <span>Contraseña</span>
                    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <strong>
                        {credentials.data.password
                          ? showPass
                            ? credentials.data.password
                            : "••••••••••••"
                          : "No guardada"}
                      </strong>
                      {credentials.data.password && (
                        <>
                          <button style={styles.smallBtn} onClick={() => setShowPass(s => !s)}>
                            {showPass ? "🙈" : "👁️"}
                          </button>
                          <button style={styles.smallBtn} onClick={() => copySecret(credentials.data?.password, "Contraseña")}>
                            🔑 Copiar
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                  <div style={styles.kvRow}>
                    <span>Detalles</span>
                    <strong>{credentials.data.details ?? "—"}</strong>
                  </div>
                  <div style={styles.kvRow}>
                    <span>Notas</span>
                    <strong>{credentials.data.notes ?? "—"}</strong>
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === "historial" && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {(switches.loading || pings.loading) && <div style={styles.hint}>Cargando historial…</div>}
              {switches.error && <div style={styles.error}>{switches.error}</div>}
              {pings.error && <div style={styles.error}>{pings.error}</div>}
              <div>
                <div style={styles.subHeader}>Switches ({accountSwitches.length})</div>
                {accountSwitches.length === 0 ? (
                  <div style={styles.empty}>Sin switches registrados para esta cuenta.</div>
                ) : (
                  accountSwitches.map(sw => (
                    <div key={sw.id} style={styles.listRow}>
                      <span>{sw.success ? "✅" : "❌"} {sw.device_id}</span>
                      <span style={styles.hint}>{new Date(sw.switched_at).toLocaleString("es-MX")}</span>
                    </div>
                  ))
                )}
              </div>
              <div>
                <div style={styles.subHeader}>Pings ({pings.data?.length ?? 0})</div>
                {(!pings.data || pings.data.length === 0) ? (
                  <div style={styles.empty}>Sin pings registrados para esta cuenta.</div>
                ) : (
                  pings.data.map(p => (
                    <div key={p.id} style={styles.listRow}>
                      <span>{p.success ? "✅" : "❌"} {p.is_manual ? "Manual" : "Auto"} · {p.device_id}</span>
                      <span style={styles.hint}>{new Date(p.pinged_at).toLocaleString("es-MX")}</span>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  overlay: { position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000 },
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 22, width: "min(740px, 94vw)", maxHeight: "88vh", display: "flex", flexDirection: "column", gap: 14 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  provider: { fontSize: 10, fontWeight: 700, color: "var(--color-primary)", letterSpacing: "0.08em", marginRight: 8 },
  alias: { fontSize: 16, color: "var(--color-text)" },
  email: { fontSize: 12, color: "var(--color-text-secondary)", fontFamily: "var(--font-mono)", marginTop: 2 },
  switchBtn: { padding: "6px 12px", border: "none", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "var(--color-on-primary)", fontSize: 12, fontWeight: 600, cursor: "pointer" },
  closeBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 15, color: "var(--color-text-disabled)" },
  tabs: { display: "flex", gap: 6, flexWrap: "wrap", borderBottom: "1px solid var(--color-border)", paddingBottom: 8 },
  tabBtn: { padding: "5px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "var(--color-bg-secondary)", color: "var(--color-text-secondary)", fontSize: 12, cursor: "pointer" },
  tabBtnActive: { background: "var(--color-primary)", color: "var(--color-on-primary)", borderColor: "var(--color-primary)", fontWeight: 600 },
  body: { overflowY: "auto", flex: 1, paddingRight: 4 },
  kvList: { display: "flex", flexDirection: "column", gap: 8 },
  kvRow: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 12px", background: "var(--color-bg-secondary)", borderRadius: "var(--radius-sm)", fontSize: 12, color: "var(--color-text-secondary)", gap: 12 },
  cardGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 12 },
  listRow: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 10px", background: "var(--color-bg-secondary)", borderRadius: "var(--radius-sm)", fontSize: 12, color: "var(--color-text)", marginTop: 4 },
  subHeader: { fontSize: 12, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 },
  smallBtn: { padding: "3px 8px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "var(--color-bg)", color: "var(--color-text)", fontSize: 11, cursor: "pointer" },
  empty: { padding: 20, textAlign: "center", fontSize: 12, color: "var(--color-text-disabled)" },
  hint: { fontSize: 12, color: "var(--color-text-secondary)" },
  error: { padding: "8px 12px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-sm)", fontSize: 12, color: "#991B1B" },
};

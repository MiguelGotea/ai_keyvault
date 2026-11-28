/**
 * CooldownExperimentPage — Fase 27: Experimento de cooldown.
 * Permite monitorear los tiempos de reset reales frente a los reportados
 * por cada pool y registrar observaciones locales.
 */
import { useEffect, useMemo, useState } from "react";
import { useCommand } from "../hooks/useCommand";
import type { AccountInfo, PoolSummary } from "../types/api";

interface CooldownObservation {
  id: string;
  accountId: string;
  accountAlias: string;
  provider: string;
  pool: string;
  startedAt: string;
  reportedResetAt: string | null;
  percentAtStart: number;
  endedAt: string | null;
  notes: string;
}

const STORAGE_KEY = "akv_cooldown_experiments";

function loadObservations(): CooldownObservation[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function formatDurationMs(ms: number): string {
  const abs = Math.abs(Math.round(ms / 1000));
  const h = Math.floor(abs / 3600);
  const m = Math.floor((abs % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export default function CooldownExperimentPage() {
  const [observations, setObservations] = useState<CooldownObservation[]>(loadObservations);
  const [notesInput, setNotesInput] = useState("");
  const [nowMs, setNowMs] = useState(Date.now());

  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});
  const compare = useCommand<PoolSummary[]>("get_stats_compare", { days: 7 });

  useEffect(() => {
    const timer = setInterval(() => setNowMs(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(observations));
  }, [observations]);

  const accountById = useMemo(
    () => new Map((vaultAccounts.data ?? []).map(a => [a.id, a])),
    [vaultAccounts.data]
  );

  const vaultPools = useMemo(
    () => (compare.data ?? []).filter(p => accountById.has(p.account_id)),
    [compare.data, accountById]
  );

  function startObservation(pool: PoolSummary) {
    const alias = accountById.get(pool.account_id)?.alias ?? pool.account_id;
    const obs: CooldownObservation = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      accountId: pool.account_id,
      accountAlias: alias,
      provider: pool.provider,
      pool: pool.pool,
      startedAt: new Date().toISOString(),
      reportedResetAt: pool.reset_time,
      percentAtStart: pool.current_percent_used,
      endedAt: null,
      notes: notesInput.trim(),
    };
    setObservations(prev => [obs, ...prev]);
    setNotesInput("");
  }

  function finishObservation(id: string) {
    const endedAt = new Date().toISOString();
    setObservations(prev => prev.map(o => (o.id === id ? { ...o, endedAt } : o)));
  }

  function deleteObservation(id: string) {
    setObservations(prev => prev.filter(o => o.id !== id));
  }

  const completed = observations.filter(o => o.endedAt !== null);
  const avgDurationMs =
    completed.length > 0
      ? completed.reduce((acc, o) => {
          const d = new Date(o.endedAt!).getTime() - new Date(o.startedAt).getTime();
          return acc + Math.max(0, d);
        }, 0) / completed.length
      : 0;

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h2 style={styles.title}>⏱️ Experimento de cooldown</h2>
          <p style={styles.subtitle}>
            Registra el momento en que un pool entra en límite o espera de reinicio y compara el tiempo real contra el <code>reset_time</code> reportado.
          </p>
        </div>
        <button
          style={styles.refreshBtn}
          onClick={() => compare.refetch()}
          disabled={compare.loading}
        >
          {compare.loading ? "⏳" : "↻"} Actualizar pools
        </button>
      </div>

      {/* KPIs */}
      <div style={styles.kpiGrid}>
        <div style={styles.kpiCard}>
          <span style={styles.kpiLabel}>Observaciones totales</span>
          <span style={styles.kpiVal}>{observations.length}</span>
        </div>
        <div style={styles.kpiCard}>
          <span style={styles.kpiLabel}>Completadas</span>
          <span style={styles.kpiVal}>{completed.length}</span>
        </div>
        <div style={styles.kpiCard}>
          <span style={styles.kpiLabel}>Duración real promedio</span>
          <span style={styles.kpiVal}>
            {completed.length > 0 ? formatDurationMs(avgDurationMs) : "—"}
          </span>
        </div>
      </div>

      {/* Pools activos para iniciar observación */}
      <section style={styles.section}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
          <h3 style={styles.sectionTitle}>Pools actuales</h3>
          <input
            type="text"
            value={notesInput}
            onChange={e => setNotesInput(e.target.value)}
            placeholder="Nota opcional para la próxima captura (ej. tras 429)…"
            style={styles.noteInput}
          />
        </div>
        {compare.error && <div style={styles.error}>{compare.error}</div>}
        {vaultPools.length === 0 ? (
          <div style={styles.hint}>No hay lecturas de cuota disponibles todavía.</div>
        ) : (
          <div style={styles.poolGrid}>
            {vaultPools.map(pool => {
              const alias = accountById.get(pool.account_id)?.alias ?? pool.account_id;
              const resetMs = pool.reset_time ? new Date(pool.reset_time).getTime() : NaN;
              const remaining = !Number.isNaN(resetMs) ? resetMs - nowMs : null;
              return (
                <div key={`${pool.account_id}-${pool.pool}`} style={styles.poolCard}>
                  <div style={{ fontWeight: 700, fontSize: 13 }}>{alias}</div>
                  <div style={styles.poolMeta}>{pool.provider} / {pool.pool}</div>
                  <div style={{ fontSize: 20, fontWeight: 800, color: pool.current_percent_used >= 95 ? "var(--color-danger)" : "var(--color-text)" }}>
                    {pool.current_percent_used.toFixed(1)}%
                  </div>
                  <div style={styles.poolMeta}>
                    {remaining !== null
                      ? remaining > 0
                        ? `Reset reportado en ${formatDurationMs(remaining)}`
                        : "Reset reportado ya vencido"
                      : "Sin reset_time reportado"}
                  </div>
                  <button style={styles.actionBtn} onClick={() => startObservation(pool)}>
                    📸 Iniciar observación
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </section>

      {/* Bitácora de experimentos */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>Bitácora de mediciones</h3>
        {observations.length === 0 ? (
          <div style={styles.hint}>Aún no has iniciado mediciones de cooldown.</div>
        ) : (
          <div style={styles.list}>
            {observations.map(obs => {
              const startMs = new Date(obs.startedAt).getTime();
              const endMs = obs.endedAt ? new Date(obs.endedAt).getTime() : nowMs;
              const elapsedMs = Math.max(0, endMs - startMs);
              const repMs = obs.reportedResetAt ? new Date(obs.reportedResetAt).getTime() : NaN;
              const diffVsReported =
                obs.endedAt && !Number.isNaN(repMs) ? endMs - repMs : null;

              return (
                <div key={obs.id} style={styles.obsRow}>
                  <div style={{ flex: 1, minWidth: 220 }}>
                    <div style={{ fontWeight: 700, fontSize: 13 }}>
                      {obs.accountAlias} · <span style={{ color: "var(--color-text-secondary)" }}>{obs.provider}/{obs.pool}</span>
                    </div>
                    <div style={styles.poolMeta}>
                      Inicio: {new Date(obs.startedAt).toLocaleString("es-MX")} ({obs.percentAtStart.toFixed(1)}%)
                      {obs.reportedResetAt && ` · Reset esperado: ${new Date(obs.reportedResetAt).toLocaleTimeString("es-MX")}`}
                    </div>
                    {obs.notes && <div style={{ fontSize: 12, marginTop: 4 }}>📝 {obs.notes}</div>}
                  </div>
                  <div style={{ textAlign: "right" as const, fontSize: 12 }}>
                    <div style={{ fontWeight: 700, color: obs.endedAt ? "var(--color-success)" : "var(--color-warning)" }}>
                      {obs.endedAt ? `Real: ${formatDurationMs(elapsedMs)}` : `En curso: ${formatDurationMs(elapsedMs)}`}
                    </div>
                    {diffVsReported !== null && (
                      <div style={styles.poolMeta}>
                        Desviación: {diffVsReported >= 0 ? "+" : "-"}{formatDurationMs(diffVsReported)}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 6 }}>
                    {!obs.endedAt && (
                      <button style={styles.finishBtn} onClick={() => finishObservation(obs.id)}>
                        ✅ Fin
                      </button>
                    )}
                    <button style={styles.deleteBtn} onClick={() => deleteObservation(obs.id)} title="Eliminar">
                      ✕
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 18 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12, flexWrap: "wrap" },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  subtitle: { fontSize: 12, color: "var(--color-text-secondary)", margin: "4px 0 0" },
  refreshBtn: { padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text)", fontSize: 12, cursor: "pointer" },
  kpiGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))", gap: 12 },
  kpiCard: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", padding: "12px 14px", display: "flex", flexDirection: "column", gap: 4 },
  kpiLabel: { fontSize: 11, color: "var(--color-text-secondary)" },
  kpiVal: { fontSize: 22, fontWeight: 800, color: "var(--color-text)" },
  section: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 18, display: "flex", flexDirection: "column", gap: 12 },
  sectionTitle: { fontSize: 15, fontWeight: 600, color: "var(--color-text)", margin: 0 },
  noteInput: { padding: "6px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg-secondary)", color: "var(--color-text)", fontSize: 12, minWidth: 260 },
  poolGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(210px, 1fr))", gap: 12 },
  poolCard: { background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", padding: 12, display: "flex", flexDirection: "column", gap: 6 },
  poolMeta: { fontSize: 11, color: "var(--color-text-secondary)" },
  actionBtn: { marginTop: 4, padding: "6px 10px", background: "var(--color-primary)", color: "var(--color-on-primary)", border: "none", borderRadius: "var(--radius-md)", fontSize: 12, fontWeight: 600, cursor: "pointer" },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  obsRow: { display: "flex", alignItems: "center", gap: 12, padding: "10px 12px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", flexWrap: "wrap" },
  finishBtn: { padding: "5px 10px", background: "var(--color-success)", color: "#fff", border: "none", borderRadius: "var(--radius-md)", fontSize: 12, fontWeight: 600, cursor: "pointer" },
  deleteBtn: { padding: "5px 8px", background: "transparent", color: "var(--color-text-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 12, cursor: "pointer" },
  hint: { fontSize: 13, color: "var(--color-text-secondary)" },
  error: { padding: 10, background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", color: "#991B1B", fontSize: 12 },
};

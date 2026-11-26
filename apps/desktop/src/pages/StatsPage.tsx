/** StatsPage — Fase 15: Estadísticas avanzadas (heatmap, comparativa, semanal). */
import { useMemo, useState } from "react";
import { useCommand } from "../hooks/useCommand";
import { computeEfficiencyAndWaste, computeModelTokenStats } from "../utils/stats";
import type { AccountInfo, HeatmapCell, PoolSummary } from "../types/api";
import es from "../i18n/es.json";

const DAYS_OPTIONS = [7, 14, 30];
const DAYS_LABELS: Record<number, string> = { 7: "7 días", 14: "14 días", 30: "30 días" };
const DAYS_COLS = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

export default function StatsPage() {
  const [days, setDays] = useState(7);
  const [selectedIdx, setSelectedIdx] = useState(0);

  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});
  const accounts = vaultAccounts.data ?? [];
  const selectedAccount = accounts[selectedIdx];
  const accountById = useMemo(
    () => new Map(accounts.map(a => [a.id, a])),
    [accounts]
  );

  const compare = useCommand<PoolSummary[]>("get_stats_compare", { days });
  const vaultPools = useMemo(
    () => (compare.data ?? []).filter(p => accountById.has(p.account_id)),
    [compare.data, accountById]
  );
  const modelStats = useMemo(() => computeModelTokenStats(vaultPools), [vaultPools]);
  const efficiencyStats = useMemo(() => computeEfficiencyAndWaste(vaultPools), [vaultPools]);
  const totalTokensUsed = modelStats.reduce((s, m) => s + m.tokensUsed, 0);
  const avgEfficiency =
    efficiencyStats.length > 0
      ? efficiencyStats.reduce((s, e) => s + e.projectedEndPercent, 0) / efficiencyStats.length
      : 0;
  const avgWasted =
    efficiencyStats.length > 0
      ? efficiencyStats.reduce((s, e) => s + e.wastedPercent, 0) / efficiencyStats.length
      : 0;

  const heatmap = useCommand<HeatmapCell[]>(
    "get_stats_heatmap",
    { accountId: selectedAccount?.id ?? "", days },
    { enabled: !!selectedAccount?.id }
  );

  // Construir grilla 24h × 7d para el heatmap
  const heatGrid: number[][] = Array.from({ length: 24 }, () => Array(7).fill(0));
  heatmap.data?.forEach(cell => {
    if (cell.weekday >= 0 && cell.weekday < 7 && cell.hour >= 0 && cell.hour < 24) {
      heatGrid[cell.hour][cell.weekday] = cell.delta_percent;
    }
  });
  const maxVal = Math.max(...heatmap.data?.map(c => c.delta_percent) ?? [1], 1);

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <h2 style={styles.title}>{es.nav.stats}</h2>
        <div style={{ display: "flex", gap: 8 }}>
          {DAYS_OPTIONS.map(d => (
            <button
              key={d}
              style={{ ...styles.tabBtn, ...(days === d ? styles.tabBtnActive : {}) }}
              onClick={() => setDays(d)}
            >
              {DAYS_LABELS[d]}
            </button>
          ))}
        </div>
      </div>

      {/* KPIs ampliados (Fase 27) */}
      <div style={styles.kpiRow}>
        <div style={styles.compareCard}>
          <span style={styles.comparePool}>Tokens acumulados</span>
          <span style={styles.comparePct}>{totalTokensUsed.toLocaleString("es-MX")}</span>
        </div>
        <div style={styles.compareCard}>
          <span style={styles.comparePool}>Eficiencia proyectada promedio</span>
          <span style={{ ...styles.comparePct, color: "var(--color-success)" }}>
            {avgEfficiency.toFixed(1)}%
          </span>
        </div>
        <div style={styles.compareCard}>
          <span style={styles.comparePool}>Cuota desperdiciada estimada</span>
          <span style={{ ...styles.comparePct, color: "var(--color-warning)" }}>
            {avgWasted.toFixed(1)}%
          </span>
        </div>
      </div>

      {/* Tokens por modelo / pool (Fase 27) */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>Tokens por modelo / pool</h3>
        {modelStats.length === 0 ? (
          <div style={styles.hint}>Sin lecturas de tokens en la ventana seleccionada.</div>
        ) : (
          <div style={styles.compareGrid}>
            {modelStats.map(m => (
              <div key={`${m.provider}-${m.pool}`} style={styles.compareCard}>
                <div style={styles.compareAccount}>{m.provider.toUpperCase()} / {m.pool}</div>
                <div style={styles.comparePct}>{m.tokensUsed.toLocaleString("es-MX")}</div>
                <div style={styles.comparePool}>
                  {m.tokensLimit > 0
                    ? `de ${m.tokensLimit.toLocaleString("es-MX")} tokens (${m.accountsCount} cuenta/s)`
                    : `${m.avgPercentUsed.toFixed(1)}% uso promedio (${m.accountsCount} cuenta/s)`}
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Eficiencia y cuota desperdiciada (Fase 27) */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>Eficiencia y cuota desperdiciada al reset</h3>
        {efficiencyStats.length === 0 ? (
          <div style={styles.hint}>Sin datos suficientes para proyectar eficiencia.</div>
        ) : (
          <div style={styles.compareGrid}>
            {efficiencyStats.map(eff => (
              <div key={`eff-${eff.accountId}-${eff.pool}`} style={styles.compareCard}>
                <div style={styles.compareAccount}>
                  {accountById.get(eff.accountId)?.alias ?? eff.accountId}
                </div>
                <div style={styles.comparePool}>{eff.provider} / {eff.pool}</div>
                <div style={{ fontSize: 13, fontWeight: 700, color: eff.efficiencyLabel === "Óptima" ? "var(--color-success)" : eff.efficiencyLabel === "Moderada" ? "var(--color-warning)" : "var(--color-danger)" }}>
                  Eficiencia {eff.efficiencyLabel} ({eff.projectedEndPercent.toFixed(1)}%)
                </div>
                <div style={styles.compareMeta}>
                  Desperdicio est.: {eff.wastedPercent.toFixed(1)}%
                  {eff.wastedTokens > 0 ? ` (~${eff.wastedTokens.toLocaleString("es-MX")} tok)` : ""}
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Comparativa */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>Comparativa — últimos {days} días</h3>
        {compare.loading && <div style={styles.hint}>Cargando…</div>}
        {compare.data && compare.data.length > 0 && (
          <div style={styles.compareGrid}>
            {compare.data.map(pool => {
              const pct = pool.current_percent_used;
              const color = pct >= 95 ? "var(--color-danger)" : pct >= 80 ? "var(--color-warning)" : "var(--color-success)";
              return (
                <div key={`${pool.account_id}-${pool.pool}`} style={styles.compareCard}>
                  <div style={styles.compareAccount}>
                    {accountById.get(pool.account_id)?.alias ?? pool.account_id}
                  </div>
                  <div style={styles.comparePool}>{pool.provider} / {pool.pool}</div>
                  <div style={{ ...styles.comparePct, color }}>{pct.toFixed(1)}%</div>
                  <div style={styles.barTrack}>
                    <div style={{ ...styles.barFill, width: `${Math.min(pct, 100)}%`, background: color }} />
                  </div>
                  {pool.burn_rate_pct_per_hour > 0 && (
                    <div style={styles.compareMeta}>Burn {pool.burn_rate_pct_per_hour.toFixed(2)}%/h</div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </section>

      {/* Heatmap */}
      <section style={styles.section}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12 }}>
          <h3 style={styles.sectionTitle}>
            Heatmap de uso — {selectedAccount?.alias ?? "sin cuenta"}
          </h3>
          {accounts.length > 0 && (
            <select
              value={selectedIdx}
              onChange={e => setSelectedIdx(Number(e.target.value))}
              style={{ padding: "5px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text)", fontSize: 12 }}
            >
              {accounts.map((a, i) => (
                <option key={a.id} value={i}>
                  {a.provider === "agy" ? "Antigravity" : "Codex"}: {a.alias}
                </option>
              ))}
            </select>
          )}
        </div>
        <div style={styles.heatmapGrid}>
          {/* Headers días */}
          <div style={styles.heatCell} /> {/* corner */}
          {DAYS_COLS.map(d => (
            <div key={d} style={{ ...styles.heatCell, ...styles.heatHeader }}>{d}</div>
          ))}
          {/* Filas de horas */}
          {heatGrid.map((row, h) => (
            <>
              <div key={`h-${h}`} style={{ ...styles.heatCell, ...styles.heatHour }}>
                {h.toString().padStart(2, "0")}h
              </div>
              {row.map((val, d) => {
                const intensity = maxVal > 0 ? val / maxVal : 0;
                return (
                  <div
                    key={`${h}-${d}`}
                    style={{
                      ...styles.heatCell,
                      background: intensity > 0
                        ? `rgba(16,185,129,${0.1 + intensity * 0.9})`
                        : "var(--color-bg-tertiary)",
                    }}
                    title={`${DAYS_COLS[d]} ${h}h: ${val.toFixed(1)}%`}
                  />
                );
              })}
            </>
          ))}
        </div>
      </section>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 20 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  tabBtn: { padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 12, cursor: "pointer" },
  tabBtnActive: { background: "var(--color-primary)", color: "#fff", border: "none" },
  section: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 20, display: "flex", flexDirection: "column", gap: 14 },
  sectionTitle: { fontSize: 15, fontWeight: 600, color: "var(--color-text)", margin: 0 },
  hint: { color: "var(--color-text-secondary)", fontSize: 13 },
  kpiRow: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 },
  compareGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 12 },
  compareCard: { background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", padding: "12px 14px", display: "flex", flexDirection: "column", gap: 5 },
  compareAccount: { fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-secondary)" },
  comparePool: { fontSize: 12, color: "var(--color-text-disabled)" },
  comparePct: { fontSize: 24, fontWeight: 800 },
  barTrack: { height: 5, background: "var(--color-bg-tertiary)", borderRadius: 999, overflow: "hidden" },
  barFill: { height: "100%", borderRadius: 999 },
  compareMeta: { fontSize: 10, color: "var(--color-text-disabled)", fontFamily: "var(--font-mono)" },
  heatmapGrid: { display: "grid", gridTemplateColumns: "32px repeat(7, 1fr)", gap: 2, overflowX: "auto" },
  heatCell: { width: "100%", aspectRatio: "1", borderRadius: 2, minWidth: 20, minHeight: 20 },
  heatHeader: { background: "transparent", textAlign: "center" as const, fontSize: 10, color: "var(--color-text-disabled)", fontWeight: 600, aspectRatio: "unset" as unknown as string },
  heatHour: { background: "transparent", fontSize: 9, color: "var(--color-text-disabled)", display: "flex", alignItems: "center", justifyContent: "flex-end", paddingRight: 4, aspectRatio: "unset" as unknown as string },
};

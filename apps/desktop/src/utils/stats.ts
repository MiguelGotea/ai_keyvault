import type { PoolSummary } from "../types/api";

export interface ModelTokenStat {
  provider: string;
  pool: string;
  tokensUsed: number;
  tokensLimit: number;
  avgPercentUsed: number;
  accountsCount: number;
}

export interface PoolEfficiencyStat {
  accountId: string;
  provider: string;
  pool: string;
  currentPercent: number;
  projectedEndPercent: number;
  wastedPercent: number;
  wastedTokens: number;
  efficiencyLabel: "Óptima" | "Moderada" | "Baja";
  hoursUntilReset: number | null;
}

/** Agrupa tokens usados/límite por (provider, pool) a partir de los resúmenes de cuota. */
export function computeModelTokenStats(pools: PoolSummary[]): ModelTokenStat[] {
  const map = new Map<
    string,
    { provider: string; pool: string; tokensUsed: number; tokensLimit: number; pctSum: number; count: number }
  >();

  for (const p of pools) {
    const key = `${p.provider}::${p.pool}`;
    const existing = map.get(key) ?? {
      provider: p.provider,
      pool: p.pool,
      tokensUsed: 0,
      tokensLimit: 0,
      pctSum: 0,
      count: 0,
    };
    existing.tokensUsed += Math.max(0, p.tokens_used ?? 0);
    existing.tokensLimit += Math.max(0, p.tokens_limit ?? 0);
    existing.pctSum += p.current_percent_used;
    existing.count += 1;
    map.set(key, existing);
  }

  return Array.from(map.values())
    .map(item => ({
      provider: item.provider,
      pool: item.pool,
      tokensUsed: item.tokensUsed,
      tokensLimit: item.tokensLimit,
      avgPercentUsed: item.count > 0 ? item.pctSum / item.count : 0,
      accountsCount: item.count,
    }))
    .sort((a, b) => b.tokensUsed - a.tokensUsed || b.avgPercentUsed - a.avgPercentUsed);
}

/** Calcula eficiencia proyectada y cuota desperdiciada al reset para cada pool. */
export function computeEfficiencyAndWaste(
  pools: PoolSummary[],
  nowMs: number = Date.now()
): PoolEfficiencyStat[] {
  return pools.map(p => {
    let hoursUntilReset: number | null = null;
    if (p.reset_time) {
      const resetMs = new Date(p.reset_time).getTime();
      if (!Number.isNaN(resetMs) && resetMs > nowMs) {
        hoursUntilReset = (resetMs - nowMs) / 3_600_000;
      }
    }

    const projectedGain =
      hoursUntilReset !== null && p.burn_rate_pct_per_hour > 0
        ? p.burn_rate_pct_per_hour * hoursUntilReset
        : 0;

    const projectedEndPercent = Math.min(100, Math.max(0, p.current_percent_used + projectedGain));
    const wastedPercent = Math.max(0, 100 - projectedEndPercent);
    const wastedTokens =
      p.tokens_limit > 0 ? Math.round((wastedPercent / 100) * p.tokens_limit) : 0;

    const efficiencyLabel: "Óptima" | "Moderada" | "Baja" =
      projectedEndPercent >= 75 ? "Óptima" : projectedEndPercent >= 40 ? "Moderada" : "Baja";

    return {
      accountId: p.account_id,
      provider: p.provider,
      pool: p.pool,
      currentPercent: p.current_percent_used,
      projectedEndPercent,
      wastedPercent,
      wastedTokens,
      efficiencyLabel,
      hoursUntilReset,
    };
  });
}

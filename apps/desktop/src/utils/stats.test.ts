import { describe, it, expect } from "vitest";
import { computeModelTokenStats, computeEfficiencyAndWaste } from "./stats";
import type { PoolSummary } from "../types/api";

const SAMPLE_POOLS: PoolSummary[] = [
  {
    account_id: "acc-1",
    provider: "agy",
    pool: "claude",
    current_percent_used: 60,
    tokens_used: 600_000,
    tokens_limit: 1_000_000,
    burn_rate_pct_per_hour: 10,
    estimated_hours_to_100: 4,
    estimated_exhaustion_at: null,
    reset_time: "2026-11-26T14:00:00Z",
    last_read_at: "2026-11-26T12:00:00Z",
  },
  {
    account_id: "acc-2",
    provider: "agy",
    pool: "claude",
    current_percent_used: 20,
    tokens_used: 200_000,
    tokens_limit: 1_000_000,
    burn_rate_pct_per_hour: 0,
    estimated_hours_to_100: null,
    estimated_exhaustion_at: null,
    reset_time: null,
    last_read_at: "2026-11-26T12:00:00Z",
  },
];

describe("stats utils (Fase 27)", () => {
  it("agrupa tokens y promedio por modelo/pool", () => {
    const res = computeModelTokenStats(SAMPLE_POOLS);
    expect(res).toHaveLength(1);
    expect(res[0].tokensUsed).toBe(800_000);
    expect(res[0].tokensLimit).toBe(2_000_000);
    expect(res[0].avgPercentUsed).toBe(40);
    expect(res[0].accountsCount).toBe(2);
  });

  it("calcula eficiencia proyectada y cuota desperdiciada", () => {
    const nowMs = new Date("2026-11-26T12:00:00Z").getTime();
    const res = computeEfficiencyAndWaste(SAMPLE_POOLS, nowMs);
    expect(res[0].projectedEndPercent).toBeCloseTo(80, 5);
    expect(res[0].wastedPercent).toBeCloseTo(20, 5);
    expect(res[0].wastedTokens).toBe(200_000);
    expect(res[0].efficiencyLabel).toBe("Óptima");

    expect(res[1].projectedEndPercent).toBe(20);
    expect(res[1].wastedPercent).toBe(80);
    expect(res[1].efficiencyLabel).toBe("Baja");
  });
});

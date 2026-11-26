import { describe, expect, it } from "vitest";
import es from "../i18n/es.json";
import type { HeatmapCell, PoolSummary, QuotaReading } from "./api";

describe("i18n es.json", () => {
  it("contiene las secciones principales de la interfaz", () => {
    expect(es.app.nombre).toBe("AI KeyVault");
    expect(es.nav.dashboard).toBeTruthy();
    expect(es.unlock.titulo).toBeTruthy();
    expect(es.dashboard.titulo).toBeTruthy();
    expect(es.alertas.titulo).toBeTruthy();
    expect(es.chats.titulo).toBeTruthy();
  });
});

describe("PoolSummary contrato VPS", () => {
  it("acepta la estructura exacta devuelta por GET /api/stats/compare", () => {
    const sample: PoolSummary = {
      account_id: "cuenta@ejemplo.com",
      provider: "agy",
      pool: "gemini_weekly",
      current_percent_used: 82.5,
      tokens_used: 8250,
      tokens_limit: 10000,
      burn_rate_pct_per_hour: 1.5,
      estimated_hours_to_100: null,
      estimated_exhaustion_at: null,
      reset_time: "2026-12-01T00:00:00Z",
      last_read_at: "2026-11-25T13:00:00Z",
    };

    expect(sample.current_percent_used).toBeGreaterThanOrEqual(80);
    expect(sample.burn_rate_pct_per_hour).toBe(1.5);
    expect(sample.tokens_limit).toBe(10000);
  });

  it("acepta QuotaReading y HeatmapCell reales del VPS", () => {
    const reading: QuotaReading = {
      account_id: "00000000-0000-0000-0000-000000000001",
      provider: "agy",
      pool: "gemini-weekly",
      tokens_used: 250000,
      tokens_limit: 1000000,
      percent_used: 25,
      reset_time: null,
      source: "test-fake-pc",
      read_at: "2026-11-25T18:00:00Z",
    };
    const cell: HeatmapCell = {
      weekday: 2,
      weekday_name: "Mié",
      hour: 18,
      delta_percent: 12.5,
      samples: 3,
    };
    expect(reading.tokens_limit).toBe(1000000);
    expect(cell.delta_percent).toBe(12.5);
  });
});

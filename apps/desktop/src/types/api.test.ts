import { describe, expect, it } from "vitest";
import es from "../i18n/es.json";
import type { PoolSummary } from "./api";

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
});

/** PingPage — Fase 14: Ping mínimo para cuentas con cuota alta. */
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useCommand } from "../hooks/useCommand";
import es from "../i18n/es.json";

export default function PingPage() {
  const [pinging, setPinging] = useState<string | null>(null);
  const [results, setResults] = useState<Record<string, string>>({});

  const needs = useCommand<string[]>("get_pings_needs", { maxPct: 80 });

  async function doPing(accountId: string) {
    setPinging(accountId);
    try {
      await invoke("post_ping", {
        accountId,
        provider: "agy",
        deviceId: "desktop-dev",
        success: true,
        error: null,
        durationMs: null,
        isManual: true,
      });
      setResults(r => ({ ...r, [accountId]: "✅ Ping enviado" }));
    } catch (e) {
      setResults(r => ({ ...r, [accountId]: `❌ ${String(e)}` }));
    } finally {
      setPinging(null);
    }
  }

  return (
    <div style={styles.page}>
      <h2 style={styles.title}>{es.ping.titulo}</h2>
      <p style={styles.subtitle}>{es.ping.subtitulo}</p>
      <div style={styles.warningBox}>⚠️ {es.ping.advertencia}</div>

      {needs.loading && <div style={styles.hint}>Verificando cuentas…</div>}

      {needs.data?.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 36 }}>✅</span>
          <p>{es.ping.sin_cuentas}</p>
        </div>
      )}

      {needs.data && needs.data.length > 0 && (
        <div style={styles.list}>
          {needs.data.map(accountId => (
            <div key={accountId} style={styles.row}>
              <span style={styles.accountId}>{accountId}</span>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                {results[accountId] && (
                  <span style={{ fontSize: 12, color: results[accountId].startsWith("✅") ? "var(--color-success)" : "var(--color-danger)" }}>
                    {results[accountId]}
                  </span>
                )}
                <button
                  style={styles.pingBtn}
                  onClick={() => doPing(accountId)}
                  disabled={pinging === accountId}
                >
                  {pinging === accountId ? "Enviando…" : es.ping.boton_ping}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 16 },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  subtitle: { fontSize: 13, color: "var(--color-text-secondary)", margin: 0 },
  warningBox: { padding: "10px 14px", background: "#FFFBEB", border: "1px solid #FDE68A", borderRadius: "var(--radius-md)", fontSize: 12, color: "#92400E" },
  hint: { color: "var(--color-text-secondary)", fontSize: 13 },
  empty: { display: "flex", flexDirection: "column", alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  row: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 16px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)" },
  accountId: { fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text)" },
  pingBtn: { padding: "7px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 600 },
};

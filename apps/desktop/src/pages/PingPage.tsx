/**
 * PingPage — Ping mínimo para cuentas con cuota alta.
 * Solo muestra cuentas que están en la bóveda.
 */
import { invoke } from "@tauri-apps/api/core";
import { useState, useMemo } from "react";
import { useCommand } from "../hooks/useCommand";
import type { PingNeedItem, AccountInfo } from "../types/api";
import es from "../i18n/es.json";

export default function PingPage() {
  const [pinging, setPinging] = useState<string | null>(null);
  const [results, setResults] = useState<Record<string, string>>({});

  // Cuentas de la bóveda (fuente de verdad)
  const vaultAccounts = useCommand<AccountInfo[]>("list_vault_accounts", {});
  const needs = useCommand<PingNeedItem[]>("get_pings_needs", { maxPct: 80 });

  // D-02: filtrar por UUID de bóveda y mostrar alias
  const accountById = useMemo(
    () => new Map((vaultAccounts.data ?? []).map(a => [a.id, a])),
    [vaultAccounts.data]
  );

  const filteredNeeds = useMemo(
    () => (needs.data ?? []).filter(item => accountById.has(item.account_id)),
    [needs.data, accountById]
  );

  async function doPing(item: PingNeedItem) {
    const key = `${item.provider}:${item.account_id}:${item.pool}`;
    setPinging(key);
    try {
      await invoke("post_ping", {
        accountId: item.account_id,
        provider: item.provider,
        deviceId: "desktop-dev",
        success: true,
        error: null,
        durationMs: null,
        isManual: true,
      });
      setResults(r => ({ ...r, [key]: "✅ Ping enviado" }));
    } catch (e) {
      setResults(r => ({ ...r, [key]: `❌ ${String(e)}` }));
    } finally {
      setPinging(null);
    }
  }

  const isLoading = needs.loading || vaultAccounts.loading;

  return (
    <div style={styles.page}>
      <h2 style={styles.title}>{es.ping.titulo}</h2>
      <p style={styles.subtitle}>{es.ping.subtitulo}</p>
      <div style={styles.warningBox}>⚠️ {es.ping.advertencia}</div>

      {isLoading && <div style={styles.hint}>Verificando cuentas…</div>}
      {needs.error && <div style={styles.error}>{needs.error}</div>}

      {!isLoading && (vaultAccounts.data ?? []).length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 36 }}>🗝️</span>
          <p>No hay cuentas en la bóveda. Agrega una cuenta primero.</p>
        </div>
      )}

      {!isLoading && (vaultAccounts.data ?? []).length > 0 && filteredNeeds.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 36 }}>✅</span>
          <p>{es.ping.sin_cuentas}</p>
        </div>
      )}

      {filteredNeeds.length > 0 && (
        <div style={styles.list}>
          {filteredNeeds.map(item => {
            const key = `${item.provider}:${item.account_id}:${item.pool}`;
            return (
              <div key={key} style={styles.row}>
                <div style={styles.info}>
                  <span style={styles.accountId}>{accountById.get(item.account_id)?.alias ?? item.account_id}</span>
                  <span style={styles.meta}>
                    {item.provider} · {item.pool} · {item.percent_used.toFixed(1)}% usado
                  </span>
                  {item.reset_time && (
                    <span style={styles.meta}>
                      Reinicia: {new Date(item.reset_time).toLocaleString("es-MX")}
                    </span>
                  )}
                </div>
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                  {results[key] && (
                    <span style={{ fontSize: 12, color: results[key].startsWith("✅") ? "var(--color-success)" : "var(--color-danger)" }}>
                      {results[key]}
                    </span>
                  )}
                  <button
                    style={styles.pingBtn}
                    onClick={() => doPing(item)}
                    disabled={pinging === key}
                  >
                    {pinging === key ? "Enviando…" : es.ping.boton_ping}
                  </button>
                </div>
              </div>
            );
          })}
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
  error: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  empty: { display: "flex", flexDirection: "column", alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  row: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 16px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", gap: 12, flexWrap: "wrap" as const },
  info: { display: "flex", flexDirection: "column" as const, gap: 3 },
  accountId: { fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text)", fontWeight: 600 },
  meta: { fontSize: 11, color: "var(--color-text-disabled)" },
  pingBtn: { padding: "7px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "var(--color-on-primary)", fontSize: 13, cursor: "pointer", fontWeight: 600 },
};

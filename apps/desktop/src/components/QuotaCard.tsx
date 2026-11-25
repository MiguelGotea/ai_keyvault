/**
 * QuotaCard — Tarjeta de cuota para un pool de una cuenta.
 * Muestra: porcentaje usado, barra de progreso, burn rate, tiempo estimado.
 */
import type { PoolSummary } from "../types/api";

interface Props {
  pool: PoolSummary;
  onSelectAccount?: (accountId: string, provider: string) => void;
}

export default function QuotaCard({ pool, onSelectAccount }: Props) {
  const pct = Math.round(pool.current_percent_used);
  const isCrit = pct >= 95;
  const isWarn = pct >= 80 && !isCrit;

  const barColor = isCrit
    ? "var(--color-danger)"
    : isWarn
    ? "var(--color-warning)"
    : "var(--color-success)";

  const providerLabel: Record<string, string> = {
    agy: "Antigravity",
    codex: "Codex",
  };

  function formatETA(isoDate: string | null): string {
    if (!isoDate) return "—";
    const d = new Date(isoDate);
    const now = Date.now();
    const diff = d.getTime() - now;
    if (diff <= 0) return "Agotado";
    const hours = Math.floor(diff / 3_600_000);
    if (hours < 1) return `${Math.floor(diff / 60_000)} min`;
    if (hours < 24) return `${hours} h`;
    return `${Math.floor(hours / 24)} d`;
  }

  function formatResetTime(isoDate: string | null): string {
    if (!isoDate) return "—";
    return new Date(isoDate).toLocaleString("es-MX", {
      weekday: "short",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  return (
    <div
      style={{
        ...styles.card,
        borderColor: isCrit
          ? "var(--color-danger)"
          : isWarn
          ? "var(--color-warning)"
          : "var(--color-border)",
      }}
    >
      {/* Header */}
      <div style={styles.cardHeader}>
        <div>
          <div style={styles.provider}>
            {providerLabel[pool.provider] ?? pool.provider}
          </div>
          <div style={styles.poolName}>{pool.pool}</div>
        </div>
        <div style={{ ...styles.badge, background: barColor }}>
          {isCrit ? "Crítico" : isWarn ? "Atención" : "OK"}
        </div>
      </div>

      {/* Porcentaje */}
      <div style={styles.percentRow}>
        <span style={{ ...styles.percent, color: barColor }}>{pct}%</span>
        <span style={styles.percentLabel}>usado</span>
      </div>

      {/* Barra de progreso */}
      <div style={styles.barTrack}>
        <div
          style={{
            ...styles.barFill,
            width: `${Math.min(pct, 100)}%`,
            background: barColor,
          }}
        />
      </div>

      {/* Metadatos */}
      <div style={styles.meta}>
        {pool.burn_rate_pct_per_hour > 0 && (
          <div style={styles.metaItem}>
            <span style={styles.metaLabel}>Burn</span>
            <span style={styles.metaValue}>
              {pool.burn_rate_pct_per_hour.toFixed(2)}%/h
            </span>
          </div>
        )}
        {pool.estimated_exhaustion_at && (
          <div style={styles.metaItem}>
            <span style={styles.metaLabel}>Agota en</span>
            <span style={{ ...styles.metaValue, color: isCrit ? "var(--color-danger)" : undefined }}>
              {formatETA(pool.estimated_exhaustion_at)}
            </span>
          </div>
        )}
        {pool.reset_time && (
          <div style={styles.metaItem}>
            <span style={styles.metaLabel}>Reinicia</span>
            <span style={styles.metaValue}>{formatResetTime(pool.reset_time)}</span>
          </div>
        )}
        {pool.last_read_at && (
          <div style={styles.metaItem}>
            <span style={styles.metaLabel}>Leído</span>
            <span style={styles.metaValue}>
              {new Date(pool.last_read_at).toLocaleTimeString("es-MX", {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </span>
          </div>
        )}
      </div>

      {/* Acción */}
      {onSelectAccount && (
        <button
          style={styles.actionBtn}
          onClick={() => onSelectAccount(pool.account_id, pool.provider)}
        >
          Ver detalle →
        </button>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  card: {
    background: "var(--color-bg)",
    border: "1px solid",
    borderRadius: "var(--radius-lg)",
    padding: "16px 18px",
    display: "flex",
    flexDirection: "column",
    gap: 10,
    minWidth: 200,
  },
  cardHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "flex-start",
    gap: 8,
  },
  provider: {
    fontSize: 11,
    color: "var(--color-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    fontWeight: 600,
  },
  poolName: {
    fontSize: 13,
    fontWeight: 600,
    color: "var(--color-text)",
    marginTop: 2,
  },
  badge: {
    fontSize: 10,
    fontWeight: 700,
    color: "#fff",
    padding: "2px 7px",
    borderRadius: 999,
    flexShrink: 0,
  },
  percentRow: {
    display: "flex",
    alignItems: "baseline",
    gap: 5,
  },
  percent: {
    fontSize: 32,
    fontWeight: 800,
    lineHeight: 1,
    fontVariantNumeric: "tabular-nums",
  },
  percentLabel: {
    fontSize: 12,
    color: "var(--color-text-secondary)",
  },
  barTrack: {
    height: 6,
    background: "var(--color-bg-tertiary)",
    borderRadius: 999,
    overflow: "hidden",
  },
  barFill: {
    height: "100%",
    borderRadius: 999,
    transition: "width 0.4s ease",
  },
  meta: {
    display: "flex",
    flexDirection: "column",
    gap: 4,
  },
  metaItem: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    fontSize: 11,
  },
  metaLabel: { color: "var(--color-text-disabled)" },
  metaValue: { color: "var(--color-text-secondary)", fontFamily: "var(--font-mono)", fontSize: 10 },
  actionBtn: {
    marginTop: 4,
    padding: "7px 10px",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    background: "var(--color-bg-secondary)",
    color: "var(--color-text-secondary)",
    fontSize: 12,
    cursor: "pointer",
    textAlign: "right" as const,
  },
};

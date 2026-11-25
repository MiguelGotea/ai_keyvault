/**
 * ServerStatusBadge — Indicador visual de la conexión con el VPS.
 * Muestra: ● Online / ● Offline / ◌ Verificando…
 *
 * Se usa en el header del dashboard y en la pantalla de unlock.
 */
import type { ServerHealthState } from "../hooks/useServerHealth";

interface Props {
  health: ServerHealthState;
  compact?: boolean;
}

export default function ServerStatusBadge({ health, compact = false }: Props) {
  const { online, checking, lastChecked } = health;

  let dot = "◌";
  let label = "Verificando…";
  let color = "var(--color-text-disabled)";

  if (!checking) {
    if (online === true) {
      dot = "●";
      label = compact ? "Online" : "Servidor online";
      color = "var(--color-success)";
    } else if (online === false) {
      dot = "●";
      label = compact ? "Offline" : "Servidor sin respuesta";
      color = "var(--color-danger)";
    }
  }

  const timeLabel = lastChecked
    ? `Verificado ${lastChecked.toLocaleTimeString("es-MX", { hour: "2-digit", minute: "2-digit" })}`
    : "";

  return (
    <span
      style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 12 }}
      title={timeLabel}
    >
      <span style={{ color, fontSize: 10 }}>{dot}</span>
      <span style={{ color: "var(--color-text-secondary)" }}>{label}</span>
    </span>
  );
}

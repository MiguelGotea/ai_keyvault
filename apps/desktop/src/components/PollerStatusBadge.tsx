/**
 * PollerStatusBadge — Muestra el estado del sidecar poller en el header.
 * Se actualiza automáticamente cada 15 segundos consultando el estado vía IPC.
 */
import { useEffect } from "react";
import { useCommand } from "../hooks/useCommand";
import type { PollerStatus } from "../types/api";

interface Props {
  compact?: boolean;
}

export default function PollerStatusBadge({ compact = false }: Props) {
  const { data, refetch } = useCommand<PollerStatus>("get_poller_status");

  // Refresco cada 15 segundos
  useEffect(() => {
    const timer = setInterval(refetch, 15_000);
    return () => clearInterval(timer);
  }, [refetch]);

  if (!data) return null;

  const icons: Record<string, string> = {
    running: "●",
    paused: "⏸",
    stopped: "○",
    error: "✕",
  };
  const colors: Record<string, string> = {
    running: "var(--color-success)",
    paused: "var(--color-warning)",
    stopped: "var(--color-text-disabled)",
    error: "var(--color-danger)",
  };
  const labels: Record<string, string> = {
    running: compact ? "Poller" : "Poller activo",
    paused: "Poller pausado",
    stopped: "Poller detenido",
    error: "Poller error",
  };

  return (
    <span
      style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 12 }}
      title={data.error ?? labels[data.state]}
    >
      <span style={{ color: colors[data.state], fontSize: 10 }}>
        {icons[data.state]}
      </span>
      <span style={{ color: "var(--color-text-secondary)" }}>
        {labels[data.state]}
      </span>
    </span>
  );
}

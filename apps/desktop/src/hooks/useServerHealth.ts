/**
 * useServerHealth — Hook que verifica la conexión con el VPS al montar.
 * Refresca automáticamente cada 30 segundos.
 */
import { useState, useEffect, useCallback } from "react";
import { useCommand } from "./useCommand";

export interface ServerHealthState {
  online: boolean | null;   // null = aún verificando
  checking: boolean;
  error: string | null;
  lastChecked: Date | null;
  recheck: () => void;
}

export function useServerHealth(): ServerHealthState {
  const [lastChecked, setLastChecked] = useState<Date | null>(null);

  const { data, loading, error, refetch } = useCommand<boolean>(
    "check_server_health",
    {},
    { enabled: true }
  );

  const recheck = useCallback(() => {
    refetch();
    setLastChecked(new Date());
  }, [refetch]);

  useEffect(() => {
    if (data !== null) setLastChecked(new Date());
  }, [data]);

  // Refresco automático cada 30 segundos
  useEffect(() => {
    const timer = setInterval(recheck, 30_000);
    return () => clearInterval(timer);
  }, [recheck]);

  return {
    online: data,
    checking: loading,
    error,
    lastChecked,
    recheck,
  };
}

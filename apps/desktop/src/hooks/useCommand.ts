/**
 * useCommand — Hook genérico para invocar comandos Tauri desde React.
 *
 * Encapsula invoke() con estados de loading/error/data y limpieza al desmontar.
 *
 * USO:
 *   const { data, loading, error, refetch } = useCommand<QuotaReading[]>(
 *     "get_quota",
 *     { accountId: "user@example.com" }
 *   );
 *
 * NOTAS:
 * - args se serializa a camelCase en la llamada (Tauri convierte a snake_case en Rust)
 * - error es string | null (Tauri serializa Result<_, String> del lado Rust)
 */

import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback, useRef } from "react";

export interface CommandState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

/**
 * Hook que llama a un comando Tauri automáticamente al montar el componente.
 * Se vuelve a llamar cuando cambian `cmd` o `args`.
 */
export function useCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options: { enabled?: boolean } = {}
): CommandState<T> {
  const { enabled = true } = options;
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const execute = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<T>(cmd, args ?? {});
      if (mountedRef.current) setData(result);
    } catch (e) {
      if (mountedRef.current) setError(typeof e === "string" ? e : String(e));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [cmd, JSON.stringify(args), enabled]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    execute();
  }, [execute]);

  return { data, loading, error, refetch: execute };
}

/**
 * Hook para llamadas lazy (el fetch no arranca automáticamente).
 * Devuelve también `run(args?)` para dispararlo manualmente.
 */
export function useCommandLazy<T>(
  cmd: string
): CommandState<T> & { run: (args?: Record<string, unknown>) => Promise<T | null> } {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const run = useCallback(async (args?: Record<string, unknown>): Promise<T | null> => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<T>(cmd, args ?? {});
      if (mountedRef.current) setData(result);
      return result;
    } catch (e) {
      const msg = typeof e === "string" ? e : String(e);
      if (mountedRef.current) setError(msg);
      return null;
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [cmd]);

  const refetch = useCallback(() => { run(); }, [run]);

  return { data, loading, error, refetch, run };
}

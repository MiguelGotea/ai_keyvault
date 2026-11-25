/** types/api.ts — Tipos compartidos entre los comandos Rust y el frontend React.
 *  Deben mantenerse sincronizados con las structs en src-tauri/src/api_client.rs
 *  y con las respuestas del backend en server/src/routes.rs
 *
 *  Convención: Tauri serializa snake_case de Rust → camelCase o snake_case en JS.
 *  Usamos snake_case para coincidir directamente con las respuestas JSON del VPS.
 */

// ── Health ──────────────────────────────────────────────────────────────────

export interface ServerHealth {
  online: boolean;
  status: string;
  server_url: string;
}

// ── Vault (local) ────────────────────────────────────────────────────────────

export interface VaultStatus {
  initialized: boolean;
  locked: boolean;
  version: string;
  accounts_count: number;
}

export interface AccountInfo {
  id: string;
  provider: string;
  alias: string;
  created_at: string;
  updated_at: string;
}

export interface UnlockResult {
  success: boolean;
  error: string | null;
  accounts_count: number;
}

export interface CreateVaultResult {
  success: boolean;
  recovery_key: string | null;
  error: string | null;
}

// ── Quota ────────────────────────────────────────────────────────────────────

export interface QuotaReading {
  id: number;
  account_id: string;
  provider: string;
  pool: string;
  percent_used: number;
  percent_remaining: number;
  reset_time: string | null;
  source: string;
  read_at: string;
}

export interface PoolSummary {
  account_id: string;
  provider: string;
  pool: string;
  current_percent: number;
  burn_rate_per_hour: number | null;
  estimated_hours_to_100: number | null;
  estimated_exhaustion_at: string | null;
  reset_time: string | null;
  last_read_at: string | null;
  source: string | null;
}

// ── Stats ────────────────────────────────────────────────────────────────────

export interface HeatmapCell {
  weekday: number;   // 0=lunes … 6=domingo
  hour_utc: number;  // 0–23
  total_delta: number;
}

// ── Chats ────────────────────────────────────────────────────────────────────

export interface ChatSession {
  id: string;
  account_id: string;
  provider: string;
  title: string | null;
  created_at: string;
  updated_at: string;
  device_id: string;
  archived: boolean;
}

// ── Alerts ───────────────────────────────────────────────────────────────────

export type AlertLevel = "Warn" | "Crit";

export interface AlertRecord {
  id: number;
  account_id: string;
  provider: string;
  pool: string;
  level: AlertLevel;
  percent_used: number;
  message: string | null;
  device_id: string;
  alerted_at: string;
}

// ── Switch ───────────────────────────────────────────────────────────────────

export interface SwitchRecord {
  id: number;
  account_id: string;
  provider: string;
  device_id: string;
  success: boolean;
  reverted: boolean;
  error_message: string | null;
  switched_at: string;
}

// ── Pings ────────────────────────────────────────────────────────────────────

export interface PingRecord {
  id: number;
  account_id: string;
  provider: string;
  device_id: string;
  success: boolean;
  error: string | null;
  duration_ms: number | null;
  is_manual: boolean;
  pinged_at: string;
}

// ── Devices ──────────────────────────────────────────────────────────────────

export interface Device {
  id: string;
  device_id: string;
  public_key_hex: string;
  registered_at: string;
  last_seen_at: string | null;
}

// ── Poller IPC (Fase 13) ─────────────────────────────────────────────────────

export type PollerState = "running" | "paused" | "stopped" | "error";

export interface PollerStatus {
  state: PollerState;
  last_poll_at: string | null;
  next_poll_at: string | null;
  error: string | null;
  pid: number | null;
}

// ── Vault sync (Fase 12) ─────────────────────────────────────────────────────

export interface VaultSyncStatus {
  last_sync_at: string | null;
  pending_changes: boolean;
}

// ── Helpers de UI ────────────────────────────────────────────────────────────

/** Proveedor de cuenta reconocido por la app */
export type Provider = "agy" | "codex";

/** Resumen de cuenta para mostrar en la lista del dashboard */
export interface AccountSummary {
  account_id: string;
  provider: Provider;
  pools: PoolSummary[];
  alerts_count: number;
  last_seen: string | null;
}

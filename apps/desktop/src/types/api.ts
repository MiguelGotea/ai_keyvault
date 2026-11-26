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
  account_id: string;
  provider: string;
  pool: string;
  tokens_used: number;
  tokens_limit: number;
  percent_used: number;
  reset_time: string | null;
  source: string;
  read_at: string;
}

export interface PoolSummary {
  account_id: string;
  provider: string;
  pool: string;
  /** Porcentaje de uso actual (0-100). Campo VPS: current_percent_used. */
  current_percent_used: number;
  tokens_used: number;
  tokens_limit: number;
  /** Tasa de quema en %/hora. Campo VPS: burn_rate_pct_per_hour. */
  burn_rate_pct_per_hour: number;
  /** Calculado en el desktop — puede ser null si burn_rate == 0. */
  estimated_hours_to_100: number | null;
  estimated_exhaustion_at: string | null;
  reset_time: string | null;
  last_read_at: string;
}

// ── Stats ────────────────────────────────────────────────────────────────────

export interface HeatmapCell {
  weekday: number;       // 0=lunes … 6=domingo
  weekday_name: string;
  hour: number;          // 0–23 UTC
  delta_percent: number;
  samples: number;
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

export type AlertLevel = "warn" | "critical" | "crit";

export interface AlertRecord {
  id: number;
  account_id: string;
  provider: string;
  pool: string;
  level: AlertLevel;
  percent_used: number;
  reset_time: string | null;
  notified_toast: boolean;
  notified_ntfy: boolean;
  detected_at: string;  // campo real del VPS (no alerted_at)
}

// ── Switch ───────────────────────────────────────────────────────────────────

/** Coincide con el struct SwitchRecord real del VPS (server/src/routes.rs). */
export interface SwitchRecord {
  id: number;
  to_account_id: string;
  from_account_id: string | null;
  provider: string;
  device_id: string;
  success: boolean;
  reason: string | null;
  switched_at: string;
}

/** Resultado del comando switch_account de Tauri (src-tauri/src/commands/switch_cmd.rs). */
export interface SwitchResult {
  success: boolean;
  error: string | null;
  account_id: string;
  provider: string;
  restart_notice: string | null;
}

// ── Pings ────────────────────────────────────────────────────────────────────

export interface PingNeedItem {
  account_id: string;
  provider: string;
  pool: string;
  percent_used: number;
  needs_ping: boolean;
  last_read_at: string;
  reset_time: string | null;
}

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

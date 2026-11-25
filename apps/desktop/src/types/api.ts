/** types/api.ts — Tipos compartidos entre los comandos Rust y el frontend React.
 *  Deben mantenerse sincronizados con las structs en src-tauri/src/commands/*.rs
 */

// ── Health ──────────────────────────────────────────────────────────────────

export interface ServerHealth {
  online: boolean;
  status: string;
  server_url: string;
}

// ── Vault ────────────────────────────────────────────────────────────────────

export interface VaultStatus {
  initialized: boolean;
  locked: boolean;
  version: string;
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

// ── Stats ────────────────────────────────────────────────────────────────────

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

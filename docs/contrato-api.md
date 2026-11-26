# Contrato API VPS ↔ Rust ↔ TypeScript

> Fuente de verdad del código del servidor: `server/src/main.rs`, `server/src/routes.rs`, `server/src/stats.rs`.
> Toda estructura modificada aquí debe actualizar `apps/desktop/src-tauri/src/api_client.rs`, `apps/desktop/src/types/api.ts` y los fixtures en `apps/desktop/src-tauri/tests/fixtures/`.

## Identidad (`D-02`)
- `account_id`: **UUID opaco de la bóveda** (`00000000-0000-0000-0000-000000000001`). El VPS nunca recibe correos.
- `device_id`: identificador del dispositivo (`hostname` / ID registrado).

## Endpoints consumidos por la app y el poller

| Método y ruta | Consumidor | Respuesta / Payload | Notas |
|---|---|---|---|
| `GET /health` | App / Poller | `{ status, service, version, timestamp }` | |
| `POST /api/devices/register` | App | Req: `{ device_id, device_name, public_key_hex }` · Resp: `{ device_id, registered, message }` | `device_name` obligatorio |
| `POST /api/vault/sync` | App | Req: `{ device_id, encrypted_payload }` · Resp: `{ version, checksum_sha256, updated_at }` | Conocimiento cero |
| `GET /api/vault/sync` | App | `{ version, device_id, encrypted_payload, checksum_sha256, updated_at }` | HTTP 400 si no hay snapshot |
| `POST /api/quota/lease` | Poller | Req: `{ account_id, device_id, lease_seconds }` · Resp: `{ acquired, locked_by, locked_until }` | |
| `POST /api/quota/readings` | Poller | Req: `{ account_id, provider, pool, tokens_used, tokens_limit, percent_used, reset_time, source }` | |
| `GET /api/quota/readings/:account_id` | App | `Vec<{ account_id, provider, pool, tokens_used, tokens_limit, percent_used, reset_time, source, read_at }>` | Sin `id` ni `percent_remaining` |
| `GET /api/stats/compare?days=N` | App | `Vec<PoolSummary>` | `current_percent_used`, `burn_rate_pct_per_hour`, `sample_count` |
| `GET /api/stats/summary/:account_id?days=N` | App | `Vec<PoolSummary>` | Igual que `/compare` |
| `GET /api/stats/heatmap/:account_id?days=N` | App | `Vec<{ weekday, weekday_name, hour, delta_percent, samples }>` | `hour` (0–23 UTC), `delta_percent` |
| `GET /api/stats/weekly/:account_id?days=N` | App | `WeeklyReport` (JSON) | |
| `GET /api/stats/export/:account_id` | *(pendiente UI)* | CSV / JSON | |
| `POST /api/chats` | Poller | Req: `{ account_id, device_id, chats: [...] }` | |
| `GET /api/chats/:account_id` | App | `Vec<{ id, provider, title, created_at, updated_at, archived }>` | `archived`: bool o 0/1 (sin `account_id`) |
| `POST /api/alerts` | Poller | Req: `{ account_id, provider, pool, level, percent_used, reset_time, notified_toast, notified_ntfy }` | `level`: `"warn"` \| `"critical"` |
| `GET /api/alerts` | App | `Vec<{ id, account_id, provider, pool, level, percent_used, reset_time, notified_toast, notified_ntfy, detected_at }>` | Booleans pueden venir como bool o 0/1 |
| `POST /api/switch` | App | Req: `{ to_account_id, from_account_id, provider, device_id, success, reason }` | |
| `GET /api/switch/history?limit=N` | App | `Vec<{ id, to_account_id, from_account_id, provider, device_id, success, reason, switched_at }>` | `success`: bool o 0/1 |
| `GET /api/pings/needs?max_pct=N` | App | `Vec<{ account_id, provider, pool, percent_used, needs_ping, last_read_at, reset_time }>` | |
| `GET /api/pings/:account_id` | App | `Vec<{ id, account_id, provider, device_id, success, error, duration_ms, is_manual, pinged_at }>` | `success`/`is_manual`: bool o 0/1 |
| `POST /api/pings` | App / Poller | Req: `{ account_id, provider, device_id, success, error, duration_ms, is_manual }` | |
| `POST /api/auth/register` | App | Req: `{ username, display_name, kdf_salt_hex, kdf_memory_kib, kdf_iterations, kdf_parallelism, auth_key_hex, device_id, device_name, encrypted_payload }` · Resp: `{ registered, username, version }` | Fase 19 |
| `POST /api/auth/params` | App | Req: `{ username }` · Resp: `{ kdf_salt_hex, kdf_memory_kib, kdf_iterations, kdf_parallelism }` | No revela si el usuario existe |
| `POST /api/auth/login` | App | Req: `{ username, auth_key_hex, device_id, device_name }` · Resp: `{ username, display_name, version, encrypted_payload, checksum_sha256, updated_at }` | Bloqueo progresivo tras 5 fallos |
| `POST /api/auth/vault` | App | Req: `{ username, auth_key_hex, device_id, encrypted_payload }` · Resp: `{ version, checksum_sha256, updated_at }` | Sync autenticado por usuario |

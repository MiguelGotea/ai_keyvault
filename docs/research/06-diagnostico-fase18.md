# Diagnóstico Fase 18.0 — Reconexión del flujo de datos

- **Fecha de verificación:** 2026-11-25
- **Alcance:** Sub-paso `18.0` de `PLAN2.md` (solo lectura/diagnóstico, sin modificar código).
- **Versión analizada:** `apps/desktop` v0.2.4 (commit `deced6c`), backend etapa 1 (`server/`, `crates/`, `providers/`).

---

## Tabla resumen: Esperado / Real / Causa

| Área | Esperado | Real | Causa | Estado |
|------|----------|------|-------|--------|
| **1. Integridad del backend (etapa 1)** | `server/`, `crates/`, `providers/`, `.github/` intactos desde el último commit pre-desktop (`8ed355d`), salvo cambios aditivos aprobados. | `server/` y `providers/` sin cambios. En `crates/core/src/vault.rs` se agregaron `state_mut()` y `decrypt_account_tokens()` (+25 líneas aditivas). En `.github/workflows/ci.yml` se ajustó CI (+94/-15 líneas). | Cambios aditivos mínimos necesarios para F17 y CI de `apps/desktop`. No se perdió funcionalidad del backend. | `[VERIFICADO]` |
| **2. VPS y esquema de BD** | VPS activo en `198.211.97.243:3010`, 8 tablas presentes (`devices`, `vault_snapshots`, `quota_readings`, `quota_leases`, `chat_sessions`, `ping_log`, `switch_log`, `alert_log`) y vacías (D-13). | Servicio `ai-keyvault` `active`; `/health` responde `healthy` (`v0.1.0`); `keyvault.db` ocupa `52K` (`Nov 25 18:49`, `read-command`). Endpoints de lectura (`/api/stats/compare`, `/api/alerts`, `/api/switch/history`, `/api/pings/needs`) devuelven `[]` y `/api/vault/sync` devuelve HTTP 400 (`No existe ninguna bóveda sincronizada aún`). | BD limpia y servidor respondiendo correctamente. *(Nota: `sqlite3` CLI no está instalado en el VPS y `ssh-mcp` `run-command` bloquea `python3 -c` por política de grupo `prod`.)* | `[VERIFICADO]` (HTTP + `ls -lh`) / `[SUPUESTO]` (tablas `devices` y `quota_leases`) |
| **3. Poller (sidecar) y logs** | La app lanza `ai-keyvault-poller` con env vars (`AKYV_SERVER_URL`, `AKYV_DEVICE_ID`, `AKYV_POLL_INTERVAL`), captura stdout/stderr y muestra estado/errores/último envío en UI. | El binario `ai-keyvault-poller-x86_64-pc-windows-msvc.exe` (7.09 MB) existe en `binaries/`. `start_poller()` lo lanza sin env vars (usa defaults), descarta `_rx` (no lee stdout/stderr, `last_poll_at` siempre `None`), `pause_poller`/`resume_poller` solo cambian estado local en memoria sin avisar al proceso, y `DashboardPage.tsx` silencia cualquier error que no contenga `"sidecar"` o `"No such"`. | Implementación incompleta en `commands/poller.rs` y captura silenciosa en `DashboardPage.tsx` (F13). | `[VERIFICADO]` |
| **4. Identidad y cruce de llaves (D-02)** | Toda la cadena (poller → VPS → app) usa la misma llave de cuenta (UUID opaco de la bóveda con mapa local correo→UUID). | **3 llaves distintas:** (a) Poller sube `email` (`agy --print /account` o CredMan `id_token_email`, o `auth.json` `email`). (b) `DashboardPage`, `AlertsPage`, `PingPage`, `ChatsPage` consultan/filtran por `account.alias`. (c) `switch_account` envía `account.id` (UUID) en `/api/switch`. (d) `StatsPage` tiene `"test@agy"` hardcodeado para el heatmap. | Deriva entre F13–F17 antes de cerrar D-02. Sin coincidencia de llaves, ninguna lectura del poller aparece en la UI. | `[VERIFICADO]` |
| **5. Comandos Tauri vs. backend y contratos** | Todos los endpoints del VPS expuestos con structs Rust/TS alineadas y sin errores de deserialización. | Falta comando para `GET /api/stats/export/:account_id` (CSV). Desalineaciones activas de contrato: `QuotaReading` (`id` y `percent_remaining` no existen en server; faltan `tokens_used`, `tokens_limit`), `HeatmapCell` (`hour_utc`/`total_delta` vs `hour`/`delta_percent`), `DeviceRegistration` (falta `device_name`), `AlertRecord.level` (poller envía `"critical"`, UI compara `"crit"`). Además `PingPage` llama `post_ping` con `success: true` sin ejecutar ping real, `deviceId` está hardcodeado a `"desktop-dev"`, y `switch_codex` conserva fallback JSON sintético. | Falta de suite de tests de contrato con fixtures reales (pendiente hasta `18.3`). | `[VERIFICADO]` |
| **6. Limpieza de estado local y `ai_keyvault_data`** | PC-A, PC-B y VPS limpios; `ai_keyvault_data` sin historial con correos sin cifrar. | En VPS: `keyvault.db` 52K y endpoints vacíos. En `ai_keyvault_data`: solo commit `6b85038 chore: setup inicial del repo de respaldo cifrado` (sin commits viejos en local ni remoto, confirmado por el autor). En PC local: `AppData\Roaming\com.miguelgotea.ai-keyvault` vacío; `AppData\Local\com.miguelgotea.ai-keyvault` y `~/.ai-keyvault` eliminados/desinstalados (confirmado por el autor). | Estado limpio confirmado para arrancar desde cero (D-13). | `[VERIFICADO]` (VPS y `ai_keyvault_data` local) / Confirmado por autor (PCs y remoto) |

---

## Evidencia detallada

### 1. Integridad del backend (`[VERIFICADO]`)
- **Último commit pre-desktop:** `8ed355d AI KeyVault Update 2026-11-25 07:28:45` (padre de `f9458a8`).
- **Comando:** `git -C ai_keyvault diff --stat f9458a8^..HEAD -- server/ crates/ providers/ .github/`
- **Salida:**
  ```text
   .github/workflows/ci.yml | 94 ++++++++++++++++++++++++++++++++++++++++--------
   crates/core/src/vault.rs | 25 +++++++++++++
   2 files changed, 104 insertions(+), 15 deletions(-)
  ```
- **Detalle `crates/core/src/vault.rs`:** únicamente agrega `pub fn state_mut(&mut self)` y `pub fn decrypt_account_tokens(&self, account_id: Uuid)`.

### 2. VPS y esquema de BD (`[VERIFICADO]` HTTP / `[SUPUESTO]` tablas sin endpoint directo)
- **Comando (vía `ssh-mcp` `run-command`):**
  `systemctl is-active ai-keyvault; curl -s http://127.0.0.1:3010/health; ...`
- **Salida:**
  - `systemctl is-active ai-keyvault`: `active`
  - `GET /health`: `{"service":"ai-keyvault-server","status":"healthy","timestamp":"2026-11-26T01:22:04.194037465+00:00","version":"0.1.0"}`
  - `GET /api/stats/compare?days=7`: `[]`
  - `GET /api/alerts`: `[]`
  - `GET /api/switch/history?limit=10`: `[]`
  - `GET /api/pings/needs?max_pct=100`: `[]`
  - `GET /api/vault/sync`: `{"error":"No existe ninguna bóveda sincronizada aún"}` (`HTTP:400`)
- **Pendiente de verificar por el autor:** inspección directa de `devices` y `quota_leases` si se desea confirmar 0 filas tras instalar `sqlite3` o ejecutar consulta local en el VPS.

### 3. Poller y errores silenciosos (`[VERIFICADO]`)
- `apps/desktop/src-tauri/binaries/ai-keyvault-poller-x86_64-pc-windows-msvc.exe` existe (`7089664` bytes).
- `apps/desktop/src-tauri/src/commands/poller.rs`:
  - `start_poller` (L111-114) ejecuta `app.shell().sidecar("ai-keyvault-poller")?.spawn()` sin `.envs(...)`.
  - L117: `Ok((_rx, child))` descarta el receptor de eventos stdout/stderr; `last_poll_at` nunca se actualiza.
  - `pause_poller` / `resume_poller` (L143-163) solo cambian `PollerStatus.state` en memoria.
- `apps/desktop/src/pages/DashboardPage.tsx`:
  - L35-40: `.catch((e) => { if (msg.includes("sidecar") || msg.includes("No such")) setPollerError(...) })` silencia cualquier otro fallo de arranque.
  - L52: `invoke("reset_autolock_timer", { minutes }).catch(() => {})`.
- `apps/desktop/src/pages/UnlockPage.tsx`:
  - L103 y L106: `.catch(() => {})` en `start_autolock_timer` y `download_vault_sync`.
- No hay procesos `*poller*` ni `*keyvault*` corriendo en el momento del diagnóstico (`Get-Process` sin resultados).

### 4. Cruce de llaves (`[VERIFICADO]`)
- **Poller (`crates/poller/src/collector.rs`):**
  - `collect_agy` (L41): `account_id = read_account_email().unwrap_or_else(|| "agy-unknown".to_string())` (correo).
  - `collect_codex` (L90-93): `account_id = auth.email.clone().unwrap_or_else(|| "codex-unknown".to_string())` (correo).
- **Frontend (`apps/desktop/src/pages/`):**
  - `DashboardPage.tsx` L158: `poolsByVpsId[account.alias]` (alias).
  - `AlertsPage.tsx` L17,22: `vaultAliases.has(a.account_id)` (alias).
  - `PingPage.tsx` L21,26: `vaultAliases.has(item.account_id)` (alias).
  - `ChatsPage.tsx` L16: `accountId: selected?.alias ?? ""` (alias).
  - `StatsPage.tsx` L13: `const [account] = useState("test@agy")` (hardcodeado).
- **Switch (`DashboardPage.tsx` L81 + `commands/switch_cmd.rs` L271):** envía `account.id` (UUID de bóveda) como `to_account_id`.

### 5. Comandos Tauri vs. backend y contratos (`[VERIFICADO]`)
- **Sin comando Tauri:** `GET /api/stats/export/:account_id` (CSV) existe en `server/src/routes.rs`, ausente en `commands/api.rs`.
- **Comandos sin uso en frontend** (grep en `apps/desktop/src/**/*.ts*`): `register_device`, `get_quota`, `get_pings`, `get_switch_history`, `get_stats_summary`, `get_stats_weekly`.
- **Incompatibilidades de contrato detectadas:**
  - `api_client.rs::QuotaReading` vs `routes.rs::QuotaReadingItem`: `id` y `percent_remaining` no existen en server; faltan `tokens_used` y `tokens_limit`.
  - `api_client.rs::HeatmapCell` (`hour_utc`, `total_delta`) vs `stats.rs::HeatmapCell` (`weekday_name`, `hour`, `delta_percent`, `samples`).
  - `api_client.rs::DeviceRegistration` (`device_id`, `public_key_hex`) vs `routes.rs::RegisterDeviceRequest` (requiere además `device_name`).
  - `scheduler.rs` L164 envía `level: "critical"`, `AlertsPage.tsx` L61 compara `alert.level === "crit"`.
- **Otros hallazgos:**
  - `PingPage.tsx` L34-42 llama `post_ping` con `success: true` sin ejecutar `agy` ping real.
  - `DashboardPage.tsx` L83 y `PingPage.tsx` L37 hardcodean `deviceId: "desktop-dev"`.
  - `switch_cmd.rs` L354-360 mantiene fallback JSON sintético `_akv_switch` en `switch_codex` si `tokens_json` es `None`.

### 6. Limpieza de estado (`[VERIFICADO]` parcial / `[SUPUESTO]` local y PC-B)
- `ai_keyvault_data`: `git -C ai_keyvault_data log -n 5 --oneline` muestra únicamente `6b85038 chore: setup inicial del repo de respaldo cifrado`. *(Nota: verificar si existe otro remoto o rama con historial antiguo antes de purgar.)*
- Local Windows: `AppData\Roaming\com.miguelgotea.ai-keyvault`, `AppData\Local\AI KeyVault` y `AppData\Local\com.miguelgotea.ai-keyvault` existen en disco; `C:\Users\User\.ai-keyvault` dio `PermissionDenied` en sandbox. Se requiere verificación manual del autor en PC-A y PC-B.

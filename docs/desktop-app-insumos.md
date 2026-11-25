# docs/desktop-app-insumos.md — Insumos para Segunda Etapa: App de Escritorio

> Documento generado al cierre de la Primera Etapa (2026-11-25).
> El backend está 100% operativo. Este doc es el spec de entrada para el chat
> de desarrollo de la app de escritorio (Tauri 2).

---

## Estado del backend al entrar a esta etapa

### Servidor VPS

- **IP:** `198.211.97.243` **Puerto:** `3010` (sin Caddy/TLS por ahora — pendiente subdominio)
- **Ruta:** `/opt/ai-keyvault/`
- **BD SQLite:** `/opt/ai-keyvault/data/keyvault.db`
- **Servicio:** `systemd ai-keyvault` — activo y en arranque automático
- **Deploy:** GitHub Actions → `cross build --target x86_64-unknown-linux-gnu` → `scp` → `systemctl restart`

### Tablas en la BD

| Tabla | Descripción |
|-------|-------------|
| `devices` | Dispositivos registrados (Ed25519) |
| `vault_snapshots` | Blob cifrado de la bóveda (conocimiento cero) |
| `quota_readings` | Serie temporal de lecturas de cuota |
| `quota_leases` | Leases anti-duplicado por cuenta |
| `chat_sessions` | Metadatos de chats (agy + Codex) |
| `ping_log` | Historial de pings (manual y automático) |
| `switch_log` | Historial de cambios de cuenta |
| `alert_log` | Historial de alertas WARN/CRIT disparadas |

### API REST completa

```
GET  /health
# Dispositivos
POST /api/devices/register
# Bóveda
POST /api/vault/sync         (upload cifrado)
GET  /api/vault/sync         (download cifrado)
# Cuota
POST /api/quota/lease
POST /api/quota/readings
GET  /api/quota/readings/:account_id
# Chats
POST /api/chats
GET  /api/chats/:account_id
# Pings
POST /api/pings
GET  /api/pings/:account_id?limit=20
GET  /api/pings/needs?max_pct=5
# Switch de cuenta
POST /api/switch
GET  /api/switch/history?limit=20
# Alertas
POST /api/alerts
GET  /api/alerts?account_id=&limit=20
# Estadísticas
GET  /api/stats/compare?days=7
GET  /api/stats/summary/:account_id?days=7
GET  /api/stats/heatmap/:account_id?days=30
GET  /api/stats/weekly/:account_id
GET  /api/stats/export/:account_id?format=csv|json&days=30
```

### Crates disponibles como librerías

| Crate | Uso en la app desktop |
|-------|----------------------|
| `ai-keyvault-core` | Bóveda, cifrado, clave maestra, clave de respaldo |
| `ai-keyvault-provider-agy` | Leer cuota, chats, ping, Credential Manager, switch |
| `ai-keyvault-provider-codex` | Leer auth.json, chats SQLite, switch |
| `ai-keyvault-alerter` | Reutilizable para alertas desde la app |
| `ai-keyvault-poller` | El poller puede correr como proceso hijo de Tauri |

---

## Decisiones técnicas ya tomadas (no reabrir)

| Tema | Decisión | ADR |
|------|----------|-----|
| Framework desktop | **Tauri 2** (Rust + TypeScript/React) | ADR-001 |
| Cifrado | Argon2id + XChaCha20-Poly1305 | ADR-002 |
| Sincronización | LWW + Ed25519 + lease | ADR-003 |
| Ping/Alertas | Configurable por hipótesis H1/H2/H3 | ADR-004 |
| Idioma UI | 100% español, i18n en `es.json` | PLAN §2 |
| Tema | Claro + Oscuro intercambiables | PLAN §2 |
| Colores | `#064E3B` primario, `#D6A84F` secundario | CLAUDE.md §4 |
| Hipótesis cooldown | **H1 confirmada** — el reloj empieza con el primer uso | research/04 |

---

## Pantallas requeridas (Segunda Etapa)

### 1. Inicio / Desbloqueo
- Input de **clave maestra** (nunca se guarda en disco)
- Autobloqueo configurable (default 15 min de inactividad)
- Opción de recuperación con **clave de respaldo** (offline)

### 2. Dashboard principal
- Resumen de todas las cuentas: % usado, pool activo, próximo reset
- Indicador visual: verde / amarillo (≥80%) / rojo (≥95%)
- Botón "Cambiar cuenta" → flujo de switch
- Botón "Ping manual" para cuentas con 0% de uso
- Badge de cuentas que necesitan ping (`GET /api/pings/needs`)

### 3. Detalle de cuenta
- Gráfica de burn rate (serie temporal)
- Heatmap de uso por día/hora
- Historial de pings y switches
- Pools individuales con reset_time

### 4. Cambio de cuenta (Switch)
- Lista de cuentas guardadas en la bóveda
- Botón "Activar" → `AccountSwitcher::switch()` local → `POST /api/switch`
- Validación automática (`agy models` / `codex verify`)
- Revertir automático si falla
- **Solo una cuenta activa a la vez** — Credential Manager tiene 1 slot

### 5. Gestión de bóveda
- Agregar cuenta nueva:
  - agy: ingresar email + activar sesión (agy ya debe estar abierto) → el switcher lee del Credential Manager
  - Codex: ingresar email + activar sesión → el switcher lee de auth.json
- Editar / eliminar cuenta
- **Importar** desde Credential Manager / auth.json existente (ya hay código para esto)
- Exportar bóveda cifrada (para backup manual)

### 6. Chats
- Lista de chats activos de **todas las PCs** (vía `GET /api/chats/:account_id`)
- Filtro por proveedor (agy / Codex) y por dispositivo
- Título + fecha de última actividad
- **No se muestra contenido** — solo metadatos

### 7. Estadísticas
- Comparativa de cuentas (bar chart `GET /api/stats/compare`)
- Burn rate proyectado (`GET /api/stats/summary`)
- Mapa de calor semanal (`GET /api/stats/heatmap`)
- Exportar CSV (`GET /api/stats/export?format=csv`)

### 8. Alertas
- Historial de alertas disparadas (`GET /api/alerts`)
- Toggle global WARN/CRIT
- Toggle ping automático (`AKYV_PING_ENABLED`)

### 9. Configuración
- URL del servidor VPS
- Device ID (nombre de esta PC)
- Intervalo de polling (default 10 min)
- Hora del ping automático (default 8:00)
- Máx pings por día (default 2)
- Toast habilitado (Windows)
- ntfy.sh tópico (campo de texto, nunca aparece en logs)
- Autobloqueo (minutos)
- Tema (claro / oscuro)

---

## Integración del Poller en la app

El poller **ya existe** como binario independiente (`ai-keyvault-poller`).
En la app Tauri puede correr de dos formas:

**Opción A (recomendada):** El poller corre como **proceso hijo** de Tauri:
```rust
// En el setup de Tauri
tauri::Builder::default()
    .setup(|app| {
        let poller = std::process::Command::new("ai-keyvault-poller")
            .envs(config.to_env_map())
            .spawn()?;
        app.manage(poller);
        Ok(())
    })
```

**Opción B:** El poller corre como **servicio de sistema** (ya instalado por el setup).

---

## Switch de cuenta — flujo técnico detallado

```
Usuario hace clic en "Activar cuenta@gmail.com"
  ↓
App lee la credencial cifrada de la bóveda local (crate core)
  ↓
App llama a AccountSwitcher::switch(&new_credential) [providers/agy/src/switcher.rs]
  ↓
  Paso 1: Lee credencial actual del Credential Manager (para poder revertir)
  Paso 2: Escribe nueva credencial en Credential Manager (gemini:antigravity)
  Paso 3: Verifica con `agy models` que el token es válido
  Paso 4a (éxito): SwitchResult::Success { models }
  Paso 4b (fallo): Revierte credencial anterior → SwitchResult::Reverted { reason }
  ↓
App registra el resultado en VPS: POST /api/switch
  ↓
App actualiza la UI: "Cuenta activa: cuenta@gmail.com"
```

---

## Ping — flujo técnico detallado

```
Ping manual (botón en UI):
  Usuario hace clic en "Ping" para cuenta@gmail.com (que tiene 0% uso)
  ↓
  App llama a execute_ping("cuenta@gmail.com", "k") [providers/agy/src/ping.rs]
  ↓
  Ejecuta `agy --print /chat "k"` — consume ~1 token
  ↓
  App registra resultado: POST /api/pings

Ping automático (scheduler):
  AKYV_PING_ENABLED=true + hora configurada
  ↓
  pinger::maybe_ping_auto() en cada ciclo del scheduler
  ↓
  Solo actúa en cuentas con percent_used < AKYV_PING_MAX_PCT (default 5%)
```

---

## Variables de entorno — referencia completa

| Variable | Default | Descripción |
|----------|---------|-------------|
| `AKYV_SERVER_URL` | `http://198.211.97.243:3010` | URL del servidor VPS |
| `AKYV_DEVICE_ID` | hostname | Nombre de este dispositivo |
| `AKYV_POLL_INTERVAL` | `600` | Segundos entre ciclos de cuota |
| `AKYV_WARN_THRESHOLD` | `80.0` | % para alerta WARN |
| `AKYV_CRIT_THRESHOLD` | `95.0` | % para alerta CRIT |
| `AKYV_COOLDOWN_MINUTES` | `60` | Minutos entre alertas repetidas |
| `AKYV_TOAST_ENABLED` | `true` | Toast de Windows habilitado |
| `AKYV_NTFY_TOPIC` | — | Tópico de ntfy.sh (**nunca al repo**) |
| `AKYV_NTFY_URL` | `https://ntfy.sh` | URL base de ntfy |
| `AKYV_PING_ENABLED` | `false` | Ping automático global |
| `AKYV_PING_HOUR` | `8` | Hora local para ping automático |
| `AKYV_PING_MAX_DAILY` | `2` | Máx pings automáticos/cuenta/día |
| `AKYV_PING_MAX_PCT` | `5.0` | % máximo de uso para pingear |
| `AKYV_PING_MESSAGE` | `k` | Mensaje mínimo a enviar |
| `DATABASE_URL` | `sqlite:./data/keyvault.db` | BD del servidor (VPS) |
| `SERVER_MASTER_KEY` | — | Clave maestra del servidor (**solo VPS**) |
| `LOG_LEVEL` | `info` | Nivel de logs |

---

## Hipótesis de cooldown — resultado del experimento

**H1 confirmada** (2026-11-25) con datos reales del VPS:
- El `reset_time` aparece **solo cuando hay consumo** (`percent_used > 0`)
- Cada pool tiene su propio `reset_time` independiente
- El reset ocurre ~7 días después del **primer uso** del pool
- Pools sin uso tienen `reset_time` nulo → el reloj no ha arrancado
- **El ping tiene sentido** para cuentas con `percent_used < 5%`

Ver `docs/research/04-experimento-cooldown.md` para datos completos.

---

## Pendientes para la segunda etapa

### Must-have
- [ ] `apps/desktop/` — scaffolding Tauri 2 con React + TypeScript
- [ ] Pantalla de desbloqueo con clave maestra
- [ ] Dashboard de cuentas con estados de cuota
- [ ] Switch de cuenta con confirmación
- [ ] Gestión de bóveda (agregar/editar/eliminar cuentas)
- [ ] Vista de chats (metadatos multi-PC)
- [ ] Configuración (servidor, polling, alertas, ping)

### Should-have
- [ ] Estadísticas con gráficas (burn rate, heatmap)
- [ ] Historial de alertas y switches en la UI
- [ ] Ping manual desde la UI
- [ ] Ping automático toggle + configuración
- [ ] Autobloqueo (timeout de inactividad)
- [ ] Portapapeles limpiado automático (30s)

### Nice-to-have
- [ ] Instalador de Windows (GitHub Actions, SmartScreen warning documentado)
- [ ] Versión portable (zip, sin instalador)
- [ ] Importar cuentas desde Credential Manager/auth.json existentes
- [ ] Exportar bóveda cifrada (backup manual)
- [ ] Subdominio + TLS en VPS (Caddy, pendiente desde Fase 0)
- [ ] Soporte multi-idioma (ya preparado con i18n)

### Segunda etapa - Fase 9 (Proveedores adicionales)
- [ ] Adaptador Claude (Anthropic)
- [ ] Adaptador Gemini (Google AI Studio)
- [ ] Adaptador OpenCode / Grok
- [ ] Codex token renewal automático (OAuth refresh)

---

## Notas para el nuevo chat

1. **El backend no necesita cambios** para arrancar la app desktop — consume la API tal cual.
2. **Leer CLAUDE.md** al abrir el proyecto — tiene todas las reglas de trabajo.
3. **Leer este doc completo** antes de codificar cualquier pantalla.
4. El servidor corre en IP directa (no hay TLS aún) — para producción real agregar Caddy + subdominio.
5. El poller es un binario separado; la app desktop lo puede lanzar como proceso hijo.
6. Los crates `ai-keyvault-provider-agy` y `ai-keyvault-core` son las librerías principales para la app.

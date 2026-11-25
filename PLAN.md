# PLAN.md — Plan de Desarrollo de AI KeyVault

**Versión:** 1.0.0 — Primera Etapa Completada
**Última actualización:** 2026-11-25
**Estado:** ✅ Backend operativo en VPS `198.211.97.243:3010`. ⏳ Segunda etapa: app de escritorio Tauri 2.

---

## Resumen ejecutivo

AI KeyVault es una bóveda + tablero personal para administrar cuentas de agentes de IA de programación. Stack: **Tauri 2** (Rust + TypeScript) para escritorio (segunda etapa), **Rust/axum** para el servidor VPS, **SQLite** como BD, **XChaCha20-Poly1305** para cifrado, **Argon2id** como KDF. Monorepo en `ai_keyvault`.

**Estado al cierre de primera etapa (2026-11-25):**
- Fases 0–8 completadas ✅ — Backend 100% operativo
- Fases A (switch de cuenta), C (chats Codex), D (switch log), E (alert log) completadas ✅
- Fase F (re-login automático) documentada como procedimiento manual ✅
- Fase 9 (proveedores adicionales) pospuesta a segunda etapa ⏸
- Hipótesis H1 confirmada: cooldown empieza con el primer uso de la semana
- Siguiente etapa: app de escritorio Tauri 2 (ver `docs/desktop-app-insumos.md`)

---

## Estado al cierre — Primera Etapa Completada ✅

### Fases completadas

| Fase | Nombre | Estado |
|------|--------|--------|
| 0 | Investigación y base | ✅ Completada |
| 1 | Núcleo seguro (crates/core) | ✅ Completada |
| 2 | Servidor VPS | ✅ Completada |
| 3 | Adaptadores agy + Codex | ✅ Completada |
| 4 | Cuota y planes (poller) | ✅ Completada |
| 5 | Alertas | ✅ Completada |
| 6 | Agente de chats (agy) | ✅ Completada |
| 7 | Estadísticas | ✅ Completada |
| 8 | Ping mínimo | ✅ Completada |
| A | Switch de cuenta (agy + Codex) | ✅ Completada |
| C | Chats Codex (reader + endpoints) | ✅ Completada |
| D | Switch log (tabla + endpoints REST) | ✅ Completada |
| E | Alert log (tabla + endpoints REST) | ✅ Completada |
| F | Re-login automático | 📄 Documentado como procedimiento manual |
| 9 | Proveedores adicionales | ⏸ Pospuesta — Segunda Etapa |

### Estado del VPS (verificado 2026-11-25)

| Elemento | Estado |
|----------|--------|
| Servidor activo `198.211.97.243:3010` | ✅ Operativo |
| `GET /health` | ✅ `{"status":"healthy"}` |
| `GET /api/alerts` | ✅ Tabla existe, `[]` sin alertas aún |
| `GET /api/switch/history` | ✅ Tabla existe, `[]` |
| `GET /api/pings/needs` | ✅ Operativo |
| Tablas BD | ✅ `devices`, `vault_snapshots`, `quota_readings`, `quota_leases`, `chat_sessions`, `ping_log`, `switch_log`, `alert_log` |
| Deploy automático (GitHub Actions) | ✅ `deploy-server.yml` en verde |
| Caddy instalado | ✅ (sin subdominio — acceso directo por IP:3010) |

### Hallazgos críticos confirmados

| Supuesto original | Realidad verificada |
|-------------------|---------------------|
| Pools agy: `gemini-5h`, `3p-5h`, `gemini-weekly`, `3p-weekly` | Solo ventanas semanales en cuentas personales. `claude-gpt-5h` opcional si aparece |
| Endpoint `cloudcode-pa.googleapis.com` disponible | HTTP 403 en cuentas personales — solo Enterprise |
| Cuota agy vía API JSON | Solo vía `agy --print /usage` (texto parseado) |
| Codex: JSON por sesión en `~/.codex/sessions/` | SQLite: `~/.codex/state_5.sqlite` tabla `threads` |
| Codex: endpoint `/wham/profiles/me` | HTTP 404 — endpoint descartado |
| agy token es JWT | Token opaco (`ya29`), vida ~1h, ruta: `gemini:antigravity` → `token.access_token` |
| **H1 (cooldown con primer uso)** | **Confirmada** — el contador de 167h aparece solo cuando hay consumo |
| Chats ChatGPT.com / Gemini.com trackeables | Solo agentes locales (agy `brain/`, Codex `state_5.sqlite`) — chats web viven en la nube sin API pública |

---

## Estado actual: Fase 0 — Investigación

### Completado ✅
- [x] CLAUDE.md con todas las reglas de trabajo
- [x] README.md con alcance ético y documentación
- [x] THIRD_PARTY_NOTICES.md con licencias de referencia
- [x] Estructura de directorios del monorepo
- [x] Logos copiados a `assets/branding/`
- [x] docs/research/01-agy-investigacion.md **(actualizado con hallazgos reales)**
- [x] docs/research/02-codex-investigacion.md **(actualizado con hallazgos reales)**
- [x] docs/research/03-otros-proveedores.md
- [x] docs/research/04-experimento-cooldown.md
- [x] docs/research/05-vps-estructura.md
- [x] docs/research/busquea_rutas.md **(verificación real de rutas)**
- [x] docs/research/supuesto-tokens-local.md **(consumo por modelo vía transcript.jsonl)**
- [x] resultados_codex.md **(estructura real de state_5.sqlite verificada)**
- [x] reusltados_agy.md **(Credential Manager, tokens, agy /usage verificado)**
- [x] docs/adr/ADR-001-stack.md (Tauri 2 elegido y justificado)
- [x] docs/adr/ADR-002-cifrado.md (Argon2id + XChaCha20-Poly1305)
- [x] docs/adr/ADR-003-sincronizacion.md (lease, CRDT LWW, Ed25519)
- [x] docs/adr/ADR-004-ping-alertas.md (configurable por hipótesis H1/H2/H3)
- [x] docs/threat-model.md (7 escenarios cubiertos)
- [x] agent/cooldown-logger/ (script Python ejecutado y actualizado)
- [x] .gitignore estricto
- [x] .github/workflows/ci.yml (CI con gitleaks)
- [x] .github/workflows/deploy-server.yml
- [x] GitHub Actions Secrets configurados
- [x] VPS setup: `/opt/ai-keyvault/{bin,config,data,logs,backups}` creados
- [x] VPS: usuario `ai-keyvault-deploy` con SSH configurado
- [x] VPS: `.env` del servidor creado
- [x] VPS: Caddy instalado (Caddyfile pendiente de configurar con subdominio)
- [x] Push inicial a ambos repos en GitHub

### Hallazgos críticos de la investigación (corrigen supuestos)

| Supuesto original | Realidad verificada |
|-------------------|---------------------|
| Pools de agy: `gemini-5h`, `3p-5h`, `gemini-weekly`, `3p-weekly` | **Solo ventanas semanales** en cuentas personales. No hay ventanas de 5h. |
| Endpoint `cloudcode-pa.googleapis.com` disponible | **HTTP 403** en cuentas personales — solo Enterprise |
| Cuota de agy vía API JSON | **Solo vía `agy /usage`** (texto parseado) o `transcript.jsonl` local |
| Codex: JSON por sesión en `~/.codex/sessions/` | **SQLite**: `~/.codex/state_5.sqlite` tabla `threads`. El número puede cambiar con la versión |
| Codex: endpoint `/wham/profiles/me` | **HTTP 404** — endpoint descartado |
| Codex: `tokens_used` desglosado en la tabla | Entero total en `threads`. Desglose real en `token_usage_record` del JSONL (`rollout_path`) |
| agy token es JWT | **Token opaco** (`ya29`), vida ~1h. Ruta: `gemini:antigravity` → `token.access_token` |
| Codex token: plan y `subscription_end` en raíz del JWT | Están en `https://api.openai.com/auth` → `chatgpt_plan_type` y `chatgpt_subscription_active_until` |

### Hallazgo clave para el experimento de cooldown (agy)

La UI de `agy /usage` muestra:
- Antes de usar: `100.00% — Quota available` (sin `resetTime` visible)
- Después de usar Gemini: `98.41% — Refreshes in 167h 55m`
- Después de usar Claude+GPT: `97.48% — Refreshes in 167h 59m`

**Interpretación preliminar:** el contador de "167h" (~7 días) aparece solo cuando hay consumo. Esto apoya **H1** (el reloj empieza con el primer uso), pero necesita confirmación con más muestras para distinguirlo de H3 (horario fijo que solo se muestra cuando hay consumo).

---

## Secrets necesarios (a configurar)

### GitHub Actions Secrets — repo `ai_keyvault`

Ir a: Settings → Secrets and variables → Actions → New repository secret

| Secret | Descripción | Cómo obtener |
|--------|-------------|--------------|
| `VPS_HOST` | IP del VPS | `198.211.97.243` |
| `VPS_USER` | Usuario SSH para deploy | Crear usuario `ai-keyvault-deploy` en el VPS (ver Setup VPS) |
| `VPS_SSH_KEY` | Llave SSH privada del deploy | Generar con `ssh-keygen -t ed25519 -C "ai-keyvault-deploy"` |
| `VPS_PATH` | Ruta del app en VPS | `/opt/ai-keyvault` |

### GitHub Actions Secrets — repo `ai_keyvault_data`

| Secret | Descripción | Cómo obtener |
|--------|-------------|--------------|
| `BACKUP_WRITE_KEY` | Deploy key de solo escritura | Generar otra llave SSH, agregar como Deploy Key con permisos de escritura en el repo |

---

## Setup del VPS (primera vez — manual vía SSH)

```bash
# Conectar al VPS
ssh root@198.211.97.243

# 1. Crear directorio aislado
mkdir -p /opt/ai-keyvault/{bin,config,data,logs,backups}

# 2. Crear usuario de sistema
useradd --system --no-create-home --shell /usr/sbin/nologin ai-keyvault
chown -R ai-keyvault:ai-keyvault /opt/ai-keyvault/

# 3. Instalar Caddy (si no está instalado)
apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
apt update && apt install caddy

# 4. Crear .env del servidor (sin secretos en el repo)
cat > /opt/ai-keyvault/.env << 'EOF'
DATABASE_URL=sqlite:./data/keyvault.db
JWT_SECRET=CAMBIAR_POR_256_BITS_ALEATORIOS
SERVER_MASTER_KEY=CAMBIAR_POR_32_BYTES_ALEATORIOS_HEX
PORT=3010
LOG_LEVEL=info
EOF
chmod 600 /opt/ai-keyvault/.env
chown ai-keyvault:ai-keyvault /opt/ai-keyvault/.env

# 5. Configurar Caddy (agregar al Caddyfile)
# Editar /etc/caddy/Caddyfile y agregar:
# keyvault.tudominio.com {
#     reverse_proxy localhost:3010
# }
# systemctl reload caddy

# 6. Crear usuario de deploy con permisos limitados
useradd --no-create-home --shell /bin/bash ai-keyvault-deploy
# Agregar su llave pública SSH
mkdir -p /home/ai-keyvault-deploy/.ssh
# Pegar la llave pública aquí:
echo "ssh-ed25519 AAAA... ai-keyvault-deploy" >> /home/ai-keyvault-deploy/.ssh/authorized_keys
chmod 700 /home/ai-keyvault-deploy/.ssh
chmod 600 /home/ai-keyvault-deploy/.ssh/authorized_keys
chown -R ai-keyvault-deploy:ai-keyvault-deploy /home/ai-keyvault-deploy/

# 7. Dar permiso para reiniciar el servicio sin contraseña
echo "ai-keyvault-deploy ALL=(ALL) NOPASSWD: /bin/systemctl restart ai-keyvault, /bin/systemctl status ai-keyvault" >> /etc/sudoers.d/ai-keyvault-deploy

# 8. Configurar firewall
ufw allow 80/tcp
ufw allow 443/tcp
# Puerto 3010 NO se abre al exterior — solo Caddy lo usa internamente

# 9. Instalar fail2ban
apt install -y fail2ban
# Configurar jail para ai-keyvault-auth (ver docs/research/05-vps-estructura.md)
```

---

## Fases de desarrollo

### Fase 0 — Investigación y base ← ACTUAL
**Criterios de aceptación:**
- [x] Documentos con evidencia en docs/research/
- [x] ADRs justificados
- [x] Modelo de amenazas
- [ ] cooldown-logger corriendo y capturando datos
- [ ] Usuario aprueba para continuar

### Fase 1 — Núcleo seguro (crates/core) ✅ COMPLETADA
**Contenido implementado:**
- Crate `crates/core/`: Bóveda criptográfica de conocimiento cero.
- **KDF Argon2id:** derivación de `AuthKey` (32 bytes para servidor) y `MasterDerivationKey` (32 bytes para descifrar `VaultKey`).
- **Vault Key wrapping:** clave de 256 bits generada con `OsRng`, empaquetada dualmente con `MasterDerivationKey` y `RecoveryKey` usando XChaCha20-Poly1305.
- **Cifrado por campo:** XChaCha20-Poly1305 con nonces de 192 bits y Additional Authenticated Data (`account_id:version:field_type`) para mitigar ataques cut-and-paste.
- **Clave de respaldo:** formato legible estilo Windows/BitLocker (`AKVR-XXXX-...`), parseo, validación y soporte de recuperación de bóveda y re-cifrado de claves.
- **Zeroize en memoria:** borrado seguro de claves maestras, auth keys y vault keys al autobloquear o descartar instancias (`ZeroizeOnDrop`).
- **Tests unitarios de seguridad:**
  - `test_vault_create_lock_unlock` (creación, bloqueo, desbloqueo y recuperación)
  - `test_recovery_key_format_roundtrip` (consistencia Base32 y parsing)
  - `test_ciphertext_contains_no_cleartext` (verificación de cero fuga de texto plano en memoria serializada)
- **Calidad de código:** `cargo fmt` limpio y `cargo clippy -- -D warnings` pasando en verde.

### Fase 2 — Servidor VPS ✅ COMPLETADA
**Contenido implementado:**
- Crate `server/`: API axum con SQLite escuchando en IP directa + puerto 3010 (sin subdominio).
- **Endpoints implementados:** `GET /health`, `POST /api/devices/register`, `POST+GET /api/vault/sync`, `POST /api/quota/lease`, `POST /api/quota/readings`, `GET /api/quota/readings/:account_id`.
- **Autenticación Ed25519 de dispositivos:** registro por `public_key_hex`, verificación por SHA-256.
- **Deploy automático:** GitHub Actions compila en release y sube vía rsync + polkit al VPS.
- **Respaldo cifrado:** workflow diario (`backup.yml`) descarga la BD del VPS y hace commit a `ai_keyvault_data` usando `BACKUP_WRITE_KEY`. Retención de 30 días.
- **Tests de servidor:**
  - `test_db_schema_initialization`: esquema correcto.
  - `test_servidor_no_puede_descifrar_vault`: **conocimiento cero demostrado** — el servidor almacena blobs opacos y no tiene acceso a texto plano.
  - `test_restauracion_desde_backup`: backup y restauración verifican integridad de datos.

**Cambios manuales en VPS (documentados en `docs/operaciones.md` §4):**
- Servicio `systemd` `/etc/systemd/system/ai-keyvault.service` creado.
- `.env` actualizado con `DATABASE_URL=sqlite:./data/keyvault.db?mode=rwc`.
- Permisos `bin/`: `chown ai-keyvault-deploy:ai-keyvault`, `chmod 750`.
- Usuario `ai-keyvault` agregado al grupo `ai-keyvault-deploy` para ejecutar el binario.
- Regla polkit `/etc/polkit-1/rules.d/50-ai-keyvault.rules` para reiniciar sin sudo desde SSH sin TTY.
- Puerto 3010 abierto en UFW.

**Criterios de aceptación:**
- [x] Deploy automático funciona: push → binario en VPS (binario activo en `198.211.97.243:3010`)
- [x] El servidor no puede descifrar nada (demostrado en `test_servidor_no_puede_descifrar_vault`)
- [x] Prueba de restauración pasa (`test_restauracion_desde_backup`)
- [ ] Entrar desde una segunda PC con URL + clave maestra y ver las mismas cuentas _(requiere app desktop — Fase 5+)_

### Fase 3 — Adaptadores agy + Codex ✅ COMPLETADA
**Contenido implementado:**
- Crate `providers/agy/`: Credential Manager Windows, lectura/escritura de credencial `gemini:antigravity`, parser de `agy /usage`, verificación con `agy models`, cambio de cuenta con revertir.
- Crate `providers/codex/`: lectura/escritura atómica de `~/.codex/auth.json` (.tmp → rename + .bak), decodificación de JWT para email, cambio de cuenta con revertir.
- **10 tests unitarios en verde** con `fake_agy` y `fake_codex` (sin credenciales reales, sin procesos externos).
- **6 tests de integración real** (marcados `#[ignore]` para no correr en CI Linux):
  - `providers/agy/tests/integration.rs`: leer credencial real, `agy models`, parsear `/usage` real.
  - `providers/codex/tests/integration.rs`: leer auth.json real, verificar vigencia, roundtrip atómico.
- **Warnings eliminados** (`Path` sin uso, `std::io::Read` sin uso, `CodexError/Result` sin uso en switcher).
- **CI actualizado**: `ci.yml` sin `continue-on-error` — corre `cargo test` para core, servidor y providers.

**Commits:** `2ea05e2` (implementación), `1096f5e` (fix doctest), cierre en este commit.

**Criterios de aceptación:**
- [x] Sin tocar procesos en ejecución _(por diseño: solo Credential Manager / archivo auth.json)_
- [x] Revertir funciona si el token es rechazado _(demostrado en `test_switch_fallido_revierte`)_
- [~] Cambio real de cuenta de agy _(requiere 2 cuentas Windows; lectura y verificación sí probadas)_
- [~] Cambio real de cuenta de Codex _(requiere 2 cuentas Windows; lectura y roundtrip sí probados)_

> **Nota:** Los tests de integración real corren localmente con `-- --include-ignored`.
> El switch completo entre cuentas se validará cuando haya 2 cuentas disponibles (Fase 5+).

### Fase 4 — Cuota y planes ✅ COMPLETADA
**Contenido implementado:**
- Crate `crates/poller/`: consulta periódica de cuota con lease anti-duplicado.
  - `collector.rs`: llama a `agy --print /usage` y lee `~/.codex/auth.json`, genera `QuotaSnapshot`s.
  - `lease.rs`: adquiere/verifica lease en `/api/quota/lease` antes de consultar.
  - `uploader.rs`: POST individual a `/api/quota/readings` por cada snapshot.
  - `scheduler.rs`: loop tokio cada 10 min (configurable), agrupa por `account_id`.
  - `models.rs`: `PollerConfig`, `QuotaSnapshot`, `LeaseResponse`.
  - `main.rs`: binario ejecutable con configuración por variables de entorno.
- **6 tests unitarios** en verde. **1 test de integración** `#[ignore]`.
- Script `crates/poller/scripts/test_poller_manual.ps1`.

**Criterios de aceptación:**
- [x] Lecturas subidas al VPS con `source` y `read_at` _(verificado: `TABLET-R5MH20CH` → VPS)_
- [x] Lease funciona: solo un consultador activo por cuenta _(log: `🔒 Lease adquirido locked_until=...`)_
- [x] Lecturas con fuente y antigüedad visibles _(curl `%40` devuelve `source`, `read_at`, `percent_used`)_
- [x] agy cuota funcionando: `gemini-weekly`, `claude-gpt-weekly`, `claude-gpt-5h` subidos al VPS _(2026-11-25)_
- [x] account_id correcto desde Credential Manager de Windows _(`gemini:antigravity` → `id_token_email`)_
- [ ] Datos coherentes entre PCs _(requiere 2 PCs corriendo el poller — se valida en Fase 5+)_

**Estado final Fase 4 (2026-11-25):**
- Parser agy reescrito para formato real (TSV con local datetime)
- Pools soportados: `gemini-weekly`, `claude-gpt-weekly`, `claude-gpt-5h` (Five Hour opcional)
- account_id via Credential Manager Win32 (`gemini:antigravity` → `id_token_email`)
- 7 tests unitarios agy + 6 tests poller — todos en verde
- CI completo en verde (fmt, clippy, tests, cargo audit)

**Fix aplicado (2026-11-25):** `agy /usage` → `agy --print /usage`
  - `models.rs`: `PollerConfig`, `QuotaSnapshot`, `LeaseResponse`.
  - `main.rs`: binario ejecutable con configuración por variables de entorno (`AKYV_SERVER_URL`, `AKYV_DEVICE_ID`, `AKYV_POLL_INTERVAL`).
- **6 tests unitarios** (sin red, sin providers reales).
- **1 test de integración** `#[ignore]`: ciclo completo con servidor real y agy instalado.
- Script `crates/poller/scripts/test_poller_manual.ps1` para prueba manual en Windows.

**Pendiente para cerrar criterios:**
- [ ] Correr el poller en esta PC y verificar lecturas en el servidor
- [ ] Datos coherentes entre PCs (mismo % visto desde dos PCs)
- [ ] Lecturas con fuente y antigüedad visibles (endpoint GET `/api/quota/readings/:id`)
- [ ] Lease funciona: solo un consultador activo por cuenta

**Cómo probar el poller localmente:**
```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:AKYV_POLL_INTERVAL = "30"   # 30s para prueba rápida
$env:RUST_LOG = "ai_keyvault_poller=info"
cargo run --package ai-keyvault-poller
# Ctrl+C para detener tras el primer ciclo
```

**Verificar lecturas en el servidor (PowerShell):**
```powershell
# Reemplazar ACCOUNT_ID con tu email de agy
curl.exe http://198.211.97.243:3010/api/quota/readings/tu@gmail.com
```
  - `models.rs`: `PollerConfig`, `QuotaSnapshot`, `LeaseResponse`.
- **5 tests unitarios** (sin red, sin providers reales): config defaults, serialización JSON, payload de lease y upload, snapshot de Codex.
- **1 test de integración** `#[ignore]`: ciclo completo con servidor real y agy instalado.

**Criterios de aceptación:**
- [ ] Datos coherentes entre PCs (mismo % visto desde dos PCs)
- [ ] Lecturas con fuente y antigüedad visibles
- [ ] Lease funciona: solo un consultador activo por cuenta

**Tiempo estimado:** 3 días

### Fase 5 — Alertas
**Estado:** ✅ Completada (2026-11-25)

**Contenido:**
- `crates/alerter/` — nuevo crate: detector de umbrales + notificador
- Toast notifications en Windows (`winrt-notification`)
- ntfy.sh configurado via `AKYV_NTFY_TOPIC` (env var)
- Alerta al 80% (previa, `WARN`) y 95% (crítica, `CRIT`)
- Anti-ruido: cooldown por `(account_id, pool, level)` — no spam
- Estado persistido en `~/.ai-keyvault/alert-state.json`
- Integración con el poller: alerta tras cada ciclo de recolección

**Arquitectura:**
```
crates/alerter/
  src/
    lib.rs         — pub use, re-exports
    models.rs      — AlertConfig, AlertLevel, AlertEvent, AlertState
    detector.rs    — detect_alerts(readings, state) → Vec<AlertEvent>
    cooldown.rs    — CooldownStore: persistencia de last_alerted por (account, pool, level)
    notifier.rs    — send_toast (Windows) + send_ntfy (HTTP)
  tests/
    alerter_test.rs
```

**Criterios de aceptación:**
- [ ] Alerta WARN al superar 80% en cualquier pool
- [ ] Alerta CRIT al superar 95% en cualquier pool
- [ ] No se repite la misma alerta dentro del cooldown (1h)
- [ ] Toast de Windows aparece en la bandeja
- [ ] ntfy.sh entrega en celular
- [ ] Tópico de ntfy no aparece en logs ni en el repo (solo env var)

**Tiempo estimado:** 2 días

### Fase 6 — Agente de chats
**Estado:** ✅ Completada (2026-11-25)

**Implementado:**
- `providers/agy/src/chats.rs` — scanner de `brain/<uuid>/transcript.jsonl`
- Extrae: id, título (primer USER_INPUT ≤120 chars, sin tags XML), created_at, updated_at
- `server/src/db.rs` — tabla `chat_sessions` (PK compuesta id+account_id)
- `server/src/routes.rs` — `POST /api/chats` (upsert batch) + `GET /api/chats/:account_id`
- `crates/poller/src/chat_collector.rs` — recolecta y sube tras cada ciclo de cuota
- Solo agy en Fase 6; Codex (cloud-only) queda para Fase 9

**Criterios de aceptación:**
- [x] Chat creado en PC-A aparece en PC-B (vía servidor)
- [x] Solo chats no archivados en GET (archived=0)
- [ ] PC sin agente muestra "sin agente en esta PC" _(requiere UI — Fase 7+)_

**Tiempo estimado:** 2-3 días

### Fase 7 — Estadísticas
**Estado:** ✅ Completada (2026-11-25)

**Implementado:**
- `server/src/stats.rs` — módulo puro de estadísticas con 4 tests unitarios:
  - Burn rate (`%/hora`) ignorando caídas por reset de cuota
  - Proyección de horas restantes y timestamp estimado al 100%
  - Mapa de calor por `(día_semana, hora_UTC)`
  - Informe semanal agrupado por `(fecha, pool)`
  - Exportación CSV con escape estándar
- `server/src/routes.rs` + `server/src/main.rs` — 5 endpoints REST:
  - `GET /api/stats/summary/:account_id?days=7`
  - `GET /api/stats/heatmap/:account_id?days=30`
  - `GET /api/stats/compare?days=7`
  - `GET /api/stats/weekly/:account_id`
  - `GET /api/stats/export/:account_id?format=csv|json&days=30`

**Tiempo estimado:** 3 días

### Fase 8 — Ping mínimo
**Estado:** ✅ Completada (2026-11-25)

**Implementado:**
- `providers/agy/src/ping.rs` — `execute_ping()`, `needs_ping()`, `PingConfig` (5 unit tests)
- `crates/poller/src/pinger.rs` — `maybe_ping_auto()` integrado en el scheduler
- `server/src/db.rs` — tabla `ping_log` (id, account_id, provider, device_id, success, error, duration_ms, message_sent, is_manual, pinged_at)
- `server/src/routes.rs` — 3 endpoints REST:
  - `POST /api/pings` — registra ping (manual o automático)
  - `GET  /api/pings/:account_id?limit=20` — historial de pings
  - `GET  /api/pings/needs?max_pct=5` — cuentas que necesitan ping
- `server/src/main.rs` — 3 rutas nuevas

**Hipótesis de cooldown:**
- Se opera bajo H1 (reloj empieza con el primer uso) — preliminarmente confirmada
- Ping automático habilitado con `AKYV_PING_ENABLED=true`
- Guardas: horario configurable (`AKYV_PING_HOUR=8`), límite diario (`AKYV_PING_MAX_DAILY=2`), umbral de uso (`AKYV_PING_MAX_PCT=5`)

**Tiempo estimado:** 2 días

### Fase 9 — Proveedores adicionales
**Estado:** ⏸ Pospuesta — Segunda Etapa del proyecto

**Contenido (segunda etapa):**
- Adaptadores: Claude (Anthropic), Gemini (Google AI Studio), OpenCode, Grok
- Pestaña de chat de consumo (chat normal vs agente)
- Un adaptador nuevo sin cambios en el núcleo

> **Nota:** Los chats de ChatGPT.com y Gemini.com viven 100% en la nube de cada
> proveedor y no tienen API pública de historial. Lo que sí se trackea son las
> sesiones de los agentes de código (agy local y Codex local), no los chats web.

**Tiempo estimado (segunda etapa):** 5-7 días

---

## Riesgos y mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Cambio de endpoints de agy | Media | Alta | Adaptador reemplazable sin tocar el núcleo |
| agy no tiene `agy models` en versión del usuario | Baja | Media | Detectar instalación y versión en Fase 3 |
| Formato de auth.json de Codex cambia | Baja | Alta | Test de verificación post-cambio + rollback |
| VPS comprometido | Muy baja | Media | Conocimiento cero — datos cifrados en el servidor |
| Usuario olvida clave maestra Y clave de respaldo | Baja | Crítica | Advertencia clara al crear la bóveda + recordatorio periódico |
| SmartScreen bloquea el instalador | Alta | Baja | Documentado en README; opción de bypass manual |
| Ping automático viola ToS de proveedor | Baja | Media | Guardas, registro, apagado fácil, aviso en la UI |

---

## Criterios de calidad transversales

- Código tipado, linter y formato automáticos.
- Commits convencionales en español.
- Sin telemetría hacia terceros.
- Manejo de errores en español.
- Tests unitarios + integración + e2e básicos en cada fase.
- CI en verde antes de hacer merge.
- `PLAN.md` actualizado al terminar cada fase.

---

## ✅ PRIMERA ETAPA COMPLETADA — 2026-11-25

El backend está 100% operativo. La segunda etapa (app de escritorio Tauri 2) comienza en un chat nuevo usando `docs/desktop-app-insumos.md` como spec de entrada.

### Pendientes identificados del prompt original (§ del AI-KeyVault-prompt-claude-code.md)

Los siguientes ítems estaban en el spec original y **no se implementaron** en la primera etapa. Todos son responsabilidad de la app de escritorio o de fases avanzadas:

| Item del prompt | Sección | Estado |
|----------------|---------|--------|
| App de escritorio Tauri 2 con UI | §4, §9 | ⏳ Segunda etapa — Fase 10+ |
| Pantalla de desbloqueo con clave maestra / clave de respaldo | §9.1, §5.1 | ⏳ Segunda etapa |
| Panel con tarjetas de cuentas y barras de cuota | §9.2 | ⏳ Segunda etapa |
| Detalle de cuenta con pestañas (Resumen, Cuota, Stats, Chats, Credenciales, Historial) | §9.3 | ⏳ Segunda etapa |
| Paleta de comandos `Ctrl+K` para cambiar cuenta, copiar contraseña | §9 | ⏳ Segunda etapa |
| Portapapeles con limpieza automática (30 s) | §5.3 | ⏳ Segunda etapa |
| Autobloqueo configurable (15 min por defecto) | §2, §5.3 | ⏳ Segunda etapa — crate core listo |
| Caché local cifrada + DPAPI para desbloqueo rápido | §5.3 | ⏳ Segunda etapa |
| Pantalla "Mis dispositivos" (revocar acceso) | §7.17 | ⏳ Segunda etapa |
| Pantalla "Ajustes" (tema, autobloqueo, frecuencias, ntfy, backups) | §9.10 | ⏳ Segunda etapa |
| Tema claro/oscuro intercambiable | §2 | ⏳ Segunda etapa |
| Español 100% con archivos i18n | §2 | ⏳ Segunda etapa |
| Alertas con **sugerencia de cuenta alternativa** ("cambia a esta") | §8.1 | ⏳ Segunda etapa — detección implementada, UI falta |
| Silenciar alerta por X horas | §8.1 | ⏳ Segunda etapa |
| Alertas de **fin de suscripción** (7 días / 1 día antes) | §8.1 | ⏳ Segunda etapa |
| Alertas de **login vencido o rechazado** | §8.1 | ⏳ Segunda etapa |
| Ping desde la UI con lista "cuentas que necesitan ping" | §8.2 | ⏳ Segunda etapa — backend listo |
| Botón "Deshacer" en cambio de cuenta desde la UI | §8.3 | ⏳ Segunda etapa — revert implementado en Rust |
| Login guiado por primera vez (capturar token del login oficial) | §8.3 | ⏳ Segunda etapa |
| "Sin agente en esta PC" en vista de chats | §10 | ⏳ Segunda etapa |
| Tokens detallados por JSONL (`token_usage_record`) | §6.3 | 📄 Investigado, pendiente de implementar |
| Verificación de respaldo en `ai_keyvault_data` | §3, Fase 2 | 📄 Workflow existe, nunca se probó restauración real |
| Consulta de cuota desde el VPS 24/7 (delegación explícita) | §5.2 | ⏸ Pospuesto — riesgo documentado en ADR-003 |
| Pestaña "Chat de consumo" (chat web vs agente) | §6.5 | ⏸ Pospuesto — sin API pública de ChatGPT/Gemini.com |
| Proveedores adicionales: Claude, Gemini, OpenCode, Grok | §9, Fase 9 | ⏸ Pospuesto — Segunda Etapa |
| Instalador Windows firmado (SmartScreen) | §13 | ⏳ Fase final de segunda etapa |
| Versión portable (sin instalador) | §13 | ⏳ Fase final de segunda etapa |
| `docs/operaciones.md` §rotación de claves | §13 | 📄 Procedimiento manual documentado |

### Próximos pasos para segunda etapa

1. Leer `docs/desktop-app-insumos.md` — spec completo del frontend Tauri 2
2. Iniciar Fase 10: Scaffolding Tauri 2 (`src-tauri/` + `src/`)
3. Implementar autenticación local (Argon2id) y desbloqueo de bóveda
4. Dashboard de cuota (consumir endpoints `/api/stats/*`)
5. Switch de cuenta con 1 click (consumir `/api/switch`)
6. Pantalla de alertas y ping (consumir `/api/alerts`, `/api/pings`)
7. Firma del instalador y distribución (Fase final)

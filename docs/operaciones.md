# Operaciones â€” AI KeyVault

**Fecha:** 2026-11-22  
**Ultima actualizacion:** 2026-11-25  
**Version:** 1.3

> Este documento describe cÃ³mo administrar el VPS, hacer deploys, gestionar respaldos y restaurar datos.
> TambiÃ©n registra todos los cambios manuales que se han hecho directamente en el VPS (fuera del repositorio),
> para que al replicar en otra PC o reconstruir el VPS no falte nada.

---

## 1. Deploy del servidor (automÃ¡tico)

El deploy es automÃ¡tico al hacer push a `main` en el repo `ai_keyvault`.

**Flujo:**
1. GitHub Actions compila el binario Rust para Linux x64.
2. rsync sube el binario a `/opt/ai-keyvault/bin/`.
3. SSH ejecuta `systemctl restart ai-keyvault` (via polkit â€” sin sudo).
4. Verifica con `curl localhost:3010/health`.

**Para hacer push manual:**
```powershell
# Desde la carpeta del repo
.\.scripts\gitpush.ps1 "feat(server): descripciÃ³n del cambio"
```

---

## 2. Acceso directo al VPS

```bash
ssh root@198.211.97.243
# O con el usuario de deploy:
ssh ai-keyvault-deploy@198.211.97.243
```

---

## 3. GestiÃ³n del servicio

```bash
# Estado del servicio
systemctl status ai-keyvault

# Logs en tiempo real
journalctl -u ai-keyvault -f

# Reiniciar (como ai-keyvault-deploy, sin sudo gracias a polkit)
systemctl restart ai-keyvault

# Reiniciar (como root)
systemctl restart ai-keyvault

# Ver logs histÃ³ricos
journalctl -u ai-keyvault --since "1 hour ago"

# Si el servicio queda en estado "failed" tras muchos reinicios rÃ¡pidos:
systemctl reset-failed ai-keyvault
systemctl start ai-keyvault
```

---

## 4. ConfiguraciÃ³n manual del VPS (registro de cambios fuera del repositorio)

> âš ï¸ Esta secciÃ³n documenta todo lo que se configurÃ³ manualmente en el VPS por SSH,
> fuera de lo que maneja el repositorio. Si el VPS se recrea o se migra a otra PC,
> hay que ejecutar estos pasos antes de que el deploy automÃ¡tico funcione.

### 4.1 Estructura de directorios (Fase 0 â€” 2026-11-22)

```bash
mkdir -p /opt/ai-keyvault/{bin,config,data,logs,backups}
useradd --system --no-create-home --shell /usr/sbin/nologin ai-keyvault
chown -R ai-keyvault:ai-keyvault /opt/ai-keyvault/
```

### 4.2 Usuario de deploy (Fase 0 â€” 2026-11-22)

```bash
useradd --no-create-home --shell /bin/bash ai-keyvault-deploy
mkdir -p /home/ai-keyvault-deploy/.ssh
# Pegar la llave pÃºblica del secret VPS_SSH_KEY:
echo "ssh-ed25519 AAAA... ai-keyvault-deploy" >> /home/ai-keyvault-deploy/.ssh/authorized_keys
chmod 700 /home/ai-keyvault-deploy/.ssh
chmod 600 /home/ai-keyvault-deploy/.ssh/authorized_keys
chown -R ai-keyvault-deploy:ai-keyvault-deploy /home/ai-keyvault-deploy/
```

### 4.3 Archivo `.env` del servidor (Fase 0 â€” 2026-11-22, actualizado Fase 2 â€” 2026-11-24)

Ruta: `/opt/ai-keyvault/.env`  
Propietario: `ai-keyvault:ai-keyvault`, permisos: `600`

```env
DATABASE_URL=sqlite:./data/keyvault.db?mode=rwc
JWT_SECRET=<256 bits aleatorios â€” generar con: openssl rand -hex 32>
SERVER_MASTER_KEY=<32 bytes aleatorios â€” generar con: openssl rand -hex 32>
PORT=3010
LOG_LEVEL=info
```

> **Nota (2026-11-24):** Se agregÃ³ `?mode=rwc` a `DATABASE_URL` para que SQLite cree la BD si no existe.
> Sin este parÃ¡metro, el servidor falla al arrancar con `SqliteError { code: 14, unable to open database file }`.

CreaciÃ³n del archivo de BD:
```bash
touch /opt/ai-keyvault/data/keyvault.db
chown ai-keyvault:ai-keyvault /opt/ai-keyvault/data/keyvault.db
# permisos 644 por defecto â€” el servicio puede leer/escribir como propietario
```

### 4.4 Servicio systemd (Fase 2 â€” 2026-11-24)

Ruta: `/etc/systemd/system/ai-keyvault.service`

```ini
[Unit]
Description=AI KeyVault Server
After=network.target
StartLimitIntervalSec=60
StartLimitBurst=3

[Service]
Type=simple
User=ai-keyvault
WorkingDirectory=/opt/ai-keyvault
EnvironmentFile=/opt/ai-keyvault/.env
ExecStart=/opt/ai-keyvault/bin/ai-keyvault-server
Restart=on-failure
RestartSec=5
StandardOutput=append:/opt/ai-keyvault/logs/server.log
StandardError=append:/opt/ai-keyvault/logs/server.log
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable ai-keyvault
```

### 4.5 Permisos del directorio `bin/` (Fase 2 â€” 2026-11-24)

El usuario `ai-keyvault-deploy` necesita escribir en `bin/` (para el rsync del deploy),
y el usuario `ai-keyvault` necesita ejecutar el binario.

```bash
# bin/ pertenece al usuario de deploy, grupo del servicio
chown ai-keyvault-deploy:ai-keyvault /opt/ai-keyvault/bin
chmod 750 /opt/ai-keyvault/bin

# Agregar al usuario de servicio al grupo del deploy para que pueda entrar al directorio
usermod -aG ai-keyvault-deploy ai-keyvault
```

### 4.6 Regla polkit â€” reinicio sin sudo (Fase 2 â€” 2026-11-24)

Ruta: `/etc/polkit-1/rules.d/50-ai-keyvault.rules`

```javascript
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.systemd1.manage-units" &&
        action.lookup("unit") == "ai-keyvault.service" &&
        subject.user == "ai-keyvault-deploy") {
        return polkit.Result.YES;
    }
});
```

> **Por quÃ© polkit y no sudo:** El deploy automÃ¡tico (GitHub Actions) conecta por SSH
> sin pseudo-terminal (TTY). `sudo` requiere TTY para leer contraseÃ±a aunque tenga NOPASSWD.
> Polkit autoriza directamente a systemd sin pasar por sudo, eliminando el problema.

Pasos para crearlo:
```bash
printf 'polkit.addRule(function(action, subject) {\n    if (action.id == "org.freedesktop.systemd1.manage-units" &&\n        action.lookup("unit") == "ai-keyvault.service" &&\n        subject.user == "ai-keyvault-deploy") {\n        return polkit.Result.YES;\n    }\n});\n' > /etc/polkit-1/rules.d/50-ai-keyvault.rules
chmod 644 /etc/polkit-1/rules.d/50-ai-keyvault.rules
```

### 4.7 Sudoers de respaldo (aÃºn presente, aunque ya no se usa para el deploy)

Ruta: `/etc/sudoers.d/ai-keyvault-deploy`  
Permisos: `0440` (requerido por sudo)

```
Defaults:ai-keyvault-deploy !requiretty
ai-keyvault-deploy ALL=(ALL) NOPASSWD: /usr/bin/systemctl restart ai-keyvault, /usr/bin/systemctl status ai-keyvault, /bin/systemctl restart ai-keyvault, /bin/systemctl status ai-keyvault
```

### 4.8 Firewall UFW (Fase 2 â€” 2026-11-24)

```bash
ufw allow 3010/tcp comment 'AI KeyVault Server'
# (80 y 443 ya estaban abiertos desde Fase 0 para Caddy)
```

> **Nota:** Se usa IP directa + puerto 3010 sin subdominio ni reverse proxy (decisiÃ³n de arquitectura).

### 4.9 Caddy instalado pero no configurado

Caddy estÃ¡ instalado (`apt install caddy`) pero el Caddyfile no fue configurado porque
se decidiÃ³ no usar subdominio. No interfiere con el servidor â€” escucha en puerto 80.

---

## 4.10 Cambios de Fase 3 — Cierre completo (2026-11-24)

No hubo cambios directos al VPS en Fase 3. Los cambios fueron todos en el repositorio:

| Elemento | Cambio |
|----------|--------|
| \providers/agy/\ | Crate nuevo — Credential Manager Win32, parser \/usage\, switcher con revertir |
| \providers/codex/\ | Crate nuevo — auth.json atomico, JWT decode, switcher con revertir |
| \providers/agy/tests/integration.rs\ | Tests de integracion real (marcados \#[ignore]\, no corren en CI Linux) |
| \providers/codex/tests/integration.rs\ | Tests de integracion real (marcados \#[ignore]\) |
| \.github/workflows/ci.yml\ | Quitados \continue-on-error\ en fmt/clippy/test. \cargo audit\ usa \.cargo/audit.toml\ |
| \server/Cargo.toml\ | Eliminada dependencia \
and\ sin uso (solo se usa en \crates/core\) |
| \.cargo/audit.toml\ | Creado — 3 advisories documentados e ignorados: RUSTSEC-2023-0071 (rsa), RUSTSEC-2026-0459 (libsqlite3-sys), RUSTSEC-2026-0543 (chrono) |
| \Cargo.lock\ | Actualizado con \cargo update\ |

**Estado final del CI:**
- OK \cargo fmt --check\ — bloquea
- OK \cargo clippy -- -D warnings\ — bloquea
- OK \cargo test\ (core + server + providers) — bloquea
- INFO \cargo audit\ — informacional (\continue-on-error: true\)

---

## 5. Respaldo

### 5.1 Respaldo automÃ¡tico (GitHub Actions â€” diario)

Un workflow en `.github/workflows/backup.yml` se ejecuta diariamente a las 4:00 UTC:
1. Conecta al VPS por SSH y descarga `/opt/ai-keyvault/data/keyvault.db`.
2. Hace commit del archivo en el repo `ai_keyvault_data` usando el secret `BACKUP_WRITE_KEY`.
3. Retiene los Ãºltimos 30 snapshots (elimina los mÃ¡s viejos automÃ¡ticamente).

> **Â¿Es seguro subir la BD a GitHub?**  
> SÃ­. Los `vault_snapshots` contienen blobs ya cifrados con XChaCha20-Poly1305 (Fase 1).
> El servidor nunca tiene la clave maestra. El backup es un ciphertext opaco.

**RetenciÃ³n:** 30 dÃ­as de snapshots. Los mÃ¡s viejos se eliminan automÃ¡ticamente.

### 5.2 Respaldo manual

```bash
# En el VPS (como root)
cp /opt/ai-keyvault/data/keyvault.db /opt/ai-keyvault/backups/snapshot-manual-$(date +%Y-%m-%d).db
```

---

## 6. RestauraciÃ³n desde respaldo

```bash
# 1. Clonar el repo de datos en la PC
git clone https://github.com/MiguelGotea/ai_keyvault_data.git

# 2. Copiar el snapshot al VPS
scp ai_keyvault_data/backups/keyvault_YYYYMMDD_HHMMSS.db root@198.211.97.243:/opt/ai-keyvault/backups/

# 3. En el VPS: reemplazar la BD activa
systemctl stop ai-keyvault
cp /opt/ai-keyvault/data/keyvault.db /opt/ai-keyvault/data/keyvault.db.bak
cp /opt/ai-keyvault/backups/keyvault_YYYYMMDD_HHMMSS.db /opt/ai-keyvault/data/keyvault.db
chown ai-keyvault:ai-keyvault /opt/ai-keyvault/data/keyvault.db
systemctl start ai-keyvault
curl -sf http://localhost:3010/health && echo "OK"
```

---

## 7. RotaciÃ³n de claves del servidor

### 7.1 JWT_SECRET

```bash
# Generar nuevo secreto
openssl rand -hex 32

# Editar .env en el VPS
nano /opt/ai-keyvault/.env
# Cambiar JWT_SECRET=...

# Reiniciar (invalida todos los tokens JWT existentes â€” los clientes tendrÃ¡n que re-autenticarse)
systemctl restart ai-keyvault
```

### 7.2 SERVER_MASTER_KEY

> âš ï¸ Cambiar esta clave invalida todos los refresh tokens delegados al VPS.
> Los usuarios deberÃ¡n re-delegar sus cuentas.

```bash
openssl rand -hex 32
# Editar .env y cambiar SERVER_MASTER_KEY
systemctl restart ai-keyvault
# Notificar a los usuarios que re-deleguen sus cuentas
```

---

## 8. Monitoreo

```bash
# Uso de disco
df -h /opt/ai-keyvault/

# Uso de BD
ls -lh /opt/ai-keyvault/data/keyvault.db

# Conexiones activas
ss -tlnp | grep 3010

# fail2ban (cuando se configure en Fase 2+)
# fail2ban-client status ai-keyvault-auth
```

---

## 9. ActualizaciÃ³n del servidor

```bash
# AutomÃ¡tica: hacer push a main desde la PC de desarrollo
# El GitHub Action compila y despliega

# Manual de emergencia:
# 1. Compilar en la PC de desarrollo (workspace â€” binario en target/release/)
cargo build --release --package ai-keyvault-server
# 2. Subir binario
scp target/release/ai-keyvault-server root@198.211.97.243:/opt/ai-keyvault/bin/
# NOTA: requiere que el usuario de root tenga acceso a bin/
# (bin/ pertenece a ai-keyvault-deploy; copiar primero a /tmp y luego mover como root)
# 3. Reiniciar
ssh root@198.211.97.243 "systemctl restart ai-keyvault"
```


---

## 10. Fix parser agy — 2026-11-25

**Problema:** El parser de `providers/agy/src/quota.rs` esperaba el formato visual antiguo:
```
GEMINI MODELS
  Weekly Limit Remaining
    [████] 98.41%
    Refreshes in 167h 55m
```

**Realidad verificada** con `agy --print "/usage"` (formato real desde 2026-11-25):
```
Gemini Models   Weekly Limit Remaining  100%    2026-12-02T00:32:25Z
Claude and GPT models   Weekly Limit Remaining  62%     2026-12-02T00:29:10Z
```

**Fix aplicado:**
- `parse_usage_output()` reescrito para parsear líneas TSV: detecta pool por inicio de línea (`Gemini`/`Claude`), extrae `%` con regex y timestamp ISO 8601 directo.
- `read_account_email()` nueva función: llama a `agy --print /account` para obtener email de la cuenta activa.
- `read_quota()` actualizado: usa `agy --print /usage` con verificación de `status.success()`, enriquece readings con email.
- `collect_agy()` en `crates/poller/src/collector.rs`: importa `read_account_email` para `account_id`.
- Tests de `providers/agy/tests/agy_provider_test.rs`: 6 tests reescritos para nuevo formato TSV.
- Eliminado `tracing::debug!` temporal de diagnóstico de `collector.rs`.

**Resultado esperado en logs del poller:**
```
INFO ✅ Cuota agy recolectada account_id=batidodpitayaclaude44@gmail.com pools=2
```

---

## 11. Cierre Fase 4 — Poller completo (2026-11-25)

### Resumen de lo implementado

**Poller (`crates/poller/`):** binario `ai-keyvault-poller` que corre periódicamente y:
1. Lee cuota de agy via `agy --print /usage` → parsea 2-3 pools por ciclo
2. Lee estado del token Codex via `~/.codex/auth.json`
3. Adquiere lease exclusivo en el VPS antes de subir
4. Sube lecturas via POST `/api/quota/readings`

**Pools agy soportados:**
| Pool | Nombre en BD | Descripción |
|------|-------------|-------------|
| Gemini (weekly) | `gemini-weekly` | Límite semanal de modelos Gemini |
| Claude+GPT (weekly) | `claude-gpt-weekly` | Límite semanal de Claude y GPT |
| Claude+GPT (5h) | `claude-gpt-5h` | Límite de 5 horas de Claude+GPT (si aparece) |

**account_id de agy:** leído del Credential Manager de Windows (`gemini:antigravity` → campo `id_token_email` del blob JSON).

**Variables de entorno del poller:**
```
AKYV_SERVER_URL=http://198.211.97.243:3010   # default
AKYV_DEVICE_ID=<hostname>                    # default: nombre del equipo
AKYV_POLL_INTERVAL=600                       # default: 600s (10 min)
```

### Verificación en producción (2026-11-25)

```
INFO ✅ Cuota agy recolectada account_id=batidospitayaclaude46@gmail.com pools=2
INFO 📤 Lectura subida provider=agy pool=gemini-weekly percent_used=0.0
INFO 📤 Lectura subida provider=agy pool=claude-gpt-weekly percent_used=45.0
INFO ✅ Estado Codex recolectado account_id=batidospitayaclaude32@gmail.com valid=true
INFO 📤 Lectura subida provider=codex pool=default percent_used=0.0
INFO ✅ Ciclo completado
```

**Nota curl:** el `@` en el email requiere `%40` al hacer curl manual:
```powershell
curl.exe "http://198.211.97.243:3010/api/quota/readings/cuenta%40gmail.com"
```
El dashboard (Fase 6+) usará cliente HTTP con encoding automático.

---

## §12 — Fase 5: Alerter de cuota (2026-11-25)

### Arquitectura

```
crates/alerter/
  src/
    lib.rs         — AlertManager, re-exports públicos
    models.rs      — AlertConfig, AlertLevel, AlertEvent, AlertReading
    detector.rs    — detect_alerts(readings, config) → Vec<AlertEvent>
    cooldown.rs    — CooldownStore: persistencia en ~/.ai-keyvault/alert-state.json
    notifier.rs    — Windows toast + ntfy.sh HTTP POST
  tests/
    alerter_test.rs — 8 tests de integración
```

### Dependencia circular resuelta

El alerter **no depende del poller**. El poller convierte `QuotaSnapshot → AlertReading` antes de llamar al alerter:

```rust
// crates/poller/src/scheduler.rs
let alert_readings: Vec<AlertReading> = snapshots
    .iter()
    .map(|s| AlertReading {
        account_id: s.account_id.clone(),
        provider: s.provider.clone(),
        pool: s.pool.clone(),
        percent_used: s.percent_used,
        reset_time: s.reset_time.clone(),
    })
    .collect();
alert_manager.check_and_notify(&alert_readings);
```

### Variables de entorno del alerter

| Variable | Default | Descripción |
|----------|---------|-------------|
| `AKYV_WARN_THRESHOLD` | `80` | % para alerta WARN |
| `AKYV_CRIT_THRESHOLD` | `95` | % para alerta CRIT |
| `AKYV_COOLDOWN_MINUTES` | `60` | Minutos entre alertas repetidas del mismo tipo |
| `AKYV_TOAST_ENABLED` | `true` | `"false"` para deshabilitar toast Windows |
| `AKYV_NTFY_TOPIC` | _(vacío)_ | Tópico ntfy.sh — **nunca loguear ni commitear** |
| `AKYV_NTFY_URL` | `https://ntfy.sh` | URL base del servidor ntfy |
| `AKYV_ALERT_STATE_PATH` | `~/.ai-keyvault/alert-state.json` | Ruta del estado de cooldown |

### Prueba de alerta en desarrollo

```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:AKYV_WARN_THRESHOLD = "1"      # umbral muy bajo para forzar alerta
$env:AKYV_TOAST_ENABLED  = "true"
$env:AKYV_NTFY_TOPIC     = "tu-topico"   # opcional
$env:AKYV_POLL_INTERVAL  = "30"
cargo run --package ai-keyvault-poller 2>&1 | Select-Object -First 60
```

Debe aparecer en log: `🔔 Disparando alerta de cuota`

### Estado de cooldown

Ubicación: `%USERPROFILE%\.ai-keyvault\alert-state.json`

Formato:
```json
{
  "last_alerted": {
    "cuenta@gmail.com:claude-gpt-weekly:Warn": "2026-11-25T10:00:00Z"
  }
}
```

Para forzar re-alerta: eliminar el archivo o la clave correspondiente.

---

## §13 — Fase 6: Chat Sessions de agy (2026-11-25)

### Qué hace

El poller recolecta metadatos de chats de agy tras cada ciclo de cuota y los sube al servidor. Solo se suben metadatos — el contenido de los mensajes **nunca sale del equipo**.

### Datos que se sincronizan

| Campo | Fuente | Descripción |
|-------|--------|-------------|
| `id` | Nombre de dir UUID en `brain/` | Identificador único del chat |
| `title` | Primer `USER_INPUT` del transcript | Truncado a 120 chars, sin tags XML |
| `created_at` | `created_at` del step_index 0 | ISO 8601 UTC |
| `updated_at` | `created_at` del último step | ISO 8601 UTC |
| `archived` | Siempre `false` en Fase 6 | agy no expone archivado en FS local |

### Ruta del brain/ de agy

```
Windows: %USERPROFILE%\.gemini\antigravity\brain\<uuid>\.system_generated\logs\transcript.jsonl
Linux:   ~/.gemini/antigravity/brain/<uuid>/.system_generated/logs/transcript.jsonl
```

### Endpoints nuevos

```http
POST /api/chats
Body: { account_id, device_id, chats: [{id, provider, title, created_at, updated_at, archived}] }
→ Upsert batch de chats

GET /api/chats/:account_id
→ Chats no archivados más recientes (máx 50), ordenados por updated_at DESC
```

### Verificar en VPS

```bash
curl "http://198.211.97.243:3010/api/chats/batidospitayaclaude46%40gmail.com"
```

### Tabla nueva en BD

```sql
CREATE TABLE chat_sessions (
  id          TEXT NOT NULL,
  account_id  TEXT NOT NULL,
  provider    TEXT NOT NULL DEFAULT 'agy',
  title       TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  device_id   TEXT NOT NULL,
  archived    INTEGER NOT NULL DEFAULT 0,
  synced_at   TEXT NOT NULL,
  PRIMARY KEY (id, account_id)
);
```

---

## §14 — Fase 7: Estadísticas y Exportación (2026-11-25)

### Arquitectura

- **`server/src/stats.rs`**: cálculos puros sobre `quota_readings` (sin acceso a red).
  - **Burn rate (`%/h`)**: suma deltas positivos consecutivos por pool (`curr.percent_used - prev.percent_used >= 0`) y divide entre las horas transcurridas. Si ocurre un reset de cuota (`delta < 0`), ese salto no contamina el ritmo de consumo.
  - **Proyección**: `estimated_hours_to_100 = (100 - current_percent) / burn_rate`, más `estimated_exhaustion_at` en RFC 3339.
  - **Mapa de calor**: agrupa deltas positivos por `(weekday 0..6, hour UTC 0..23)`.
  - **Informe semanal**: mínimos, máximos y consumo acumulado por día y pool en los últimos 7 días.
  - **Exportación**: CSV estándar (`Content-Disposition: attachment`) o JSON.

### Endpoints nuevos

| Método y ruta | Descripción |
|---------------|-------------|
| `GET /api/stats/summary/:account_id?days=7` | Resumen por pool con burn rate y proyección |
| `GET /api/stats/heatmap/:account_id?days=30` | Mapa de calor por día de semana y hora UTC |
| `GET /api/stats/compare?days=7` | Comparativa de todos los pools y cuentas |
| `GET /api/stats/weekly/:account_id` | Informe semanal agrupado por día y pool |
| `GET /api/stats/export/:account_id?format=csv\|json&days=30` | Exportación de la serie temporal |

### Verificación rápida en VPS

```bash
curl "http://198.211.97.243:3010/api/stats/compare"
curl "http://198.211.97.243:3010/api/stats/summary/batidospitayaclaude49%40gmail.com"
curl "http://198.211.97.243:3010/api/stats/export/batidospitayaclaude49%40gmail.com?format=csv"
```

---

## §15 — Resumen completo de cambios: Fases 4–7 (2026-11-22 → 2026-11-25)

### Estado del proyecto al 2026-11-25

| Fase | Estado | Fecha | Descripción |
|------|--------|-------|-------------|
| 0 — Investigación | ✅ | 2026-11-22 | Exploración local, ADRs, PLAN.md |
| 1 — Core seguro | ✅ | 2026-11-22 | Argon2id, XChaCha20-Poly1305, Vault |
| 2 — Servidor VPS | ✅ | 2026-11-22 | axum, SQLite, systemd, CI/CD |
| 3 — Providers | ✅ | 2026-11-22 | agy Credential Manager + Codex auth.json |
| 4 — Poller de cuota | ✅ | 2026-11-25 | Ciclo periódico, lease anti-dup, parser agy |
| 5 — Alerter | ✅ | 2026-11-25 | Detector umbrales, cooldown, toast, ntfy.sh |
| 6 — Chat sessions agy | ✅ | 2026-11-25 | Scanner brain/, upsert REST, chat_collector |
| 7 — Estadísticas | ✅ | 2026-11-25 | Burn rate, heatmap, informe semanal, CSV |
| 8 — Ping mínimo | 🔜 | — | Próxima fase |

---

### VPS `198.211.97.243` — Cambios manuales aplicados

#### Estructura de archivos en producción
```
/opt/ai-keyvault/
  bin/
    ai-keyvault-server      ← binario compilado por CI (cross-rs linux/x86_64)
  config/
    .env                    ← DATABASE_URL, PORT, SERVER_MASTER_KEY, LOG_LEVEL
  data/
    keyvault.db             ← SQLite (tablas: devices, vault_snapshots, quota_readings,
                                       quota_leases, chat_sessions)
  logs/                     ← logs de systemd (via journald)
  backups/                  ← backups manuales del DB (si aplica)
```

#### Tablas SQLite en producción (al 2026-11-25)

| Tabla | Creada en | Propósito |
|-------|-----------|-----------|
| `devices` | Fase 2 | Dispositivos registrados |
| `vault_snapshots` | Fase 2 | Blobs cifrados de bóveda (conocimiento cero) |
| `quota_readings` | Fase 3 | Serie temporal de lecturas de cuota |
| `quota_leases` | Fase 4 | Leases anti-duplicado por cuenta |
| `chat_sessions` | Fase 6 | Metadatos de chats agy |

> [!NOTE]
> Las tablas se crean automáticamente al arrancar el servidor (`db::init_db`). No se requieren migraciones manuales.

#### Servicio systemd
```ini
# /etc/systemd/system/ai-keyvault.service
[Unit]
Description=AI KeyVault Server
After=network.target

[Service]
User=ai-keyvault-deploy
WorkingDirectory=/opt/ai-keyvault
EnvironmentFile=/opt/ai-keyvault/config/.env
ExecStart=/opt/ai-keyvault/bin/ai-keyvault-server
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Comandos de gestión:
```bash
systemctl status ai-keyvault
systemctl restart ai-keyvault
journalctl -u ai-keyvault -f
```

#### Puerto y red
- Puerto: **3010** (acceso directo IP, sin Caddy/proxy)
- IP: `198.211.97.243`
- Caddy instalado pero **no configurado** (sin subdominio asignado aún)
- CORS: `allow_origin(Any)` — para desarrollo

#### Deploy CI/CD
- GitHub Actions: `.github/workflows/deploy.yml`
- Trigger: push a `main`
- Pipeline: `cargo fmt --check` → `cargo test` → `cargo clippy` → `cross build --release --target x86_64-unknown-linux-gnu` → `scp` al VPS → `systemctl restart ai-keyvault`

---

### Cambios en código (Fases 4–7)

#### Fase 4 — Poller de cuota

**Nuevos archivos:**
- `crates/poller/src/lib.rs` — PollerConfig, QuotaSnapshot, módulos
- `crates/poller/src/collector.rs` — llama a agy y codex providers
- `crates/poller/src/scheduler.rs` — ciclo tokio con ticker
- `crates/poller/src/uploader.rs` — POST `/api/quota/readings`
- `crates/poller/src/lease.rs` — adquiere lease antes de subir
- `crates/poller/src/models.rs` — QuotaSnapshot, PollerConfig
- `crates/poller/src/main.rs` — binario `ai-keyvault-poller`

**Modificados:**
- `providers/agy/src/quota.rs` — parser reescrito para formato real de `agy --print "/usage"`:
  ```
  Gemini Models          Weekly Limit Remaining     100%  2026-12-01 19:19 CST
  Claude and GPT models  Weekly Limit Remaining      49%  2026-12-01 18:57 CST
  Claude and GPT models  Five Hour Limit Remaining   49%  2026-11-24 23:57 CST
  ```
  Pool names: `gemini-weekly`, `claude-gpt-weekly`, `claude-gpt-5h`

**Lecciones aprendidas (Fase 4):**
- `agy --print "/usage"` (no `agy /usage`)
- `AgyCredentialManager::read()` es función asociada, no método de instancia
- Pool `claude-gpt-5h` aparece solo cuando hay límite de 5h activo
- `@` en URLs de curl requiere `%40` en Windows

---

#### Fase 5 — Alerter

**Nuevos archivos:**
- `crates/alerter/src/lib.rs` — `AlertManager::check_and_notify(&[AlertReading])`
- `crates/alerter/src/models.rs` — `AlertConfig`, `AlertLevel`, `AlertEvent`, `AlertReading`
- `crates/alerter/src/detector.rs` — `detect_alerts` con 4 unit tests
- `crates/alerter/src/cooldown.rs` — `CooldownStore` (JSON en `~/.ai-keyvault/alert-state.json`)
- `crates/alerter/src/notifier.rs` — Toast via PowerShell `NotifyIcon` + ntfy.sh HTTP POST
- `crates/alerter/tests/alerter_test.rs` — 9 integration tests (con `Mutex` para env vars)

**Dependencia circular resuelta:**
```
❌ alerter → ai_keyvault_poller → alerter
✅ alerter define AlertReading propio
   poller convierte QuotaSnapshot → AlertReading antes de llamar al alerter
```

**Variables de entorno del alerter:**

| Variable | Default | Descripción |
|----------|---------|-------------|
| `AKYV_WARN_THRESHOLD` | `80` | % para alerta WARN |
| `AKYV_CRIT_THRESHOLD` | `95` | % para alerta CRIT |
| `AKYV_COOLDOWN_MINUTES` | `60` | Minutos entre alertas repetidas |
| `AKYV_TOAST_ENABLED` | `true` | `"false"` desactiva toast Windows |
| `AKYV_NTFY_TOPIC` | _(vacío)_ | **Nunca loguear ni commitear** |
| `AKYV_NTFY_URL` | `https://ntfy.sh` | URL base ntfy |
| `AKYV_ALERT_STATE_PATH` | `~/.ai-keyvault/alert-state.json` | Estado de cooldown |

**Nota sobre `winrt-notification`:**
Se descartó `winrt-notification = "0.5"` — causa `STATUS_ACCESS_VIOLATION` al inicializar COM en entornos de test. Se reemplazó por PowerShell `System.Windows.Forms.NotifyIcon`.

---

#### Fase 6 — Chat sessions agy

**Nuevos archivos:**
- `providers/agy/src/chats.rs` — `scan_chats(brain_path) → Vec<AgyChat>` con 5 unit tests
- `crates/poller/src/chat_collector.rs` — `collect_and_upload(client, config, account_id)`

**Modificados:**
- `providers/agy/Cargo.toml` — agrega `dirs-next = "2.0"`, `tempfile = "3"` (dev)
- `providers/agy/src/lib.rs` — expone `pub mod chats`
- `server/src/db.rs` — tabla `chat_sessions` (PK compuesta `id + account_id`)
- `server/src/routes.rs` — `POST /api/chats` + `GET /api/chats/:account_id`
- `server/src/main.rs` — registra las rutas de chats
- `crates/poller/src/lib.rs` — expone `pub mod chat_collector`
- `crates/poller/src/scheduler.rs` — llama al `chat_collector` **fuera del loop de leases** con el `account_id` del snapshot agy

**Formato de transcript.jsonl:**
```jsonl
{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","created_at":"2026-11-22T20:43:04Z","content":"<USER_REQUEST>\nMensaje del usuario\n</USER_REQUEST>..."}
```
- `title` = contenido dentro de `<USER_REQUEST>...</USER_REQUEST>`, truncado a 120 chars
- `created_at` = timestamp del step_index 0
- `updated_at` = timestamp del último step

**Bugs corregidos en Fase 6:**
1. `extract_title` extraía texto de TODOS los tags XML (incluyendo `<METADATA>datos</METADATA>`). Fix: extraer solo `<USER_REQUEST>`.
2. `chat_collector` estaba dentro del loop de leases → se ejecutaba con el account_id equivocado (codex). Fix: moverlo fuera del loop.
3. `for s in snapshots` tomaba ownership → `Vec<_>` incompatible con `uploader`. Fix: `for s in &snapshots` con `.clone()` al armar el mapa.

---

#### Fase 7 — Estadísticas

**Nuevos archivos:**
- `server/src/stats.rs` — módulo puro sin dependencias de red:
  - `compute_burn_rate` — `%/h` ignorando caídas por reset de cuota
  - `summarize_pools` — agrupa por `(account_id, provider, pool)`, proyección y burn rate
  - `build_heatmap` — mapa de calor `(weekday, hour_utc)` con deltas positivos
  - `build_weekly_report` — agrupado por `(fecha, pool)` con min/max/delta
  - `to_csv` — exportación con escape estándar para comas y comillas

**Modificados:**
- `server/src/main.rs` — `mod stats;` + 5 rutas nuevas
- `server/src/routes.rs` — 5 handlers + helper `fetch_readings(db, account_id, days)`
- `server/Cargo.toml` — eliminado `subtle = "2.6"` (no se usaba → warning de clippy)

**Endpoints de estadísticas:**

| Ruta | Query params | Respuesta |
|------|-------------|-----------|
| `GET /api/stats/summary/:account_id` | `?days=7` | `Vec<PoolSummary>` con burn rate y proyección |
| `GET /api/stats/heatmap/:account_id` | `?days=30` | `Vec<HeatmapCell>` por `(weekday, hour)` |
| `GET /api/stats/compare` | `?days=7` | `Vec<PoolSummary>` de todas las cuentas |
| `GET /api/stats/weekly/:account_id` | — | `WeeklyReport` (últimos 7 días) |
| `GET /api/stats/export/:account_id` | `?format=csv\|json&days=30` | CSV descargable o JSON |

---

### Estado del poller local (TABLET-R5MH20CH)

**Variables de entorno para correr el poller:**
```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:AKYV_POLL_INTERVAL  = "600"   # 10 min en producción
$env:RUST_LOG            = "ai_keyvault_poller=info"
cargo run --package ai-keyvault-poller
```

**Cuenta activa agy al 2026-11-25:** `batidospitayaclaude49@gmail.com`
- Leída automáticamente del Credential Manager de Windows (`gemini:antigravity` → campo `id_token_email` del blob JSON)
- La cuenta rota frecuentemente; el poller la detecta en cada ciclo

**Verificación completa de endpoints en VPS:**
```powershell
# Cuota
curl.exe "http://198.211.97.243:3010/api/quota/readings/batidospitayaclaude49%40gmail.com"

# Chats
curl.exe "http://198.211.97.243:3010/api/chats/batidospitayaclaude49%40gmail.com"

# Estadísticas
curl.exe "http://198.211.97.243:3010/api/stats/compare"
curl.exe "http://198.211.97.243:3010/api/stats/summary/batidospitayaclaude49%40gmail.com"
curl.exe "http://198.211.97.243:3010/api/stats/weekly/batidospitayaclaude49%40gmail.com"
curl.exe "http://198.211.97.243:3010/api/stats/heatmap/batidospitayaclaude49%40gmail.com"
curl.exe "http://198.211.97.243:3010/api/stats/export/batidospitayaclaude49%40gmail.com?format=csv"
```

---

### Commits del período (2026-11-22 → 2026-11-25)

```
chore(server): eliminar dependencia subtle no utilizada
feat(fase-7): endpoints de estadisticas, burn rate, heatmap y exportacion CSV/JSON
fix(fase-6): chat_collector fuera del loop de leases, usa account_id de agy
fix(alerter): quitar import warn no usado en notifier.rs
feat(fase-6): chat collector agy con endpoint REST y tabla chat_sessions
feat(fase-5): alerter con detector de umbrales, cooldown y notificador toast+ntfy
fix(fmt): corregir formato de quota.rs y agy_provider_test.rs para CI
feat(fase-4): poller de cuota agy+codex con lease anti-duplicado y parser real
```

---

## §16 — Fase 8: Ping mínimo (2026-11-25)

### Objetivo
Implementar el sistema de ping para agy: detectar cuentas con 0 % de uso
(que nunca arrancaron su ventana de 7 días) y enviarles el mensaje más corto
posible (`k`) para iniciar el contador de reinicio de créditos (hipótesis H1).

### Archivos nuevos
- `providers/agy/src/ping.rs` — lógica de ping + `PingConfig` + 5 unit tests
- `crates/poller/src/pinger.rs` — integración automática en el scheduler
- `server/src/db.rs` — tabla `ping_log` (Fase 8)
- `server/src/routes.rs` — 3 handlers de ping
- `server/src/main.rs` — 3 rutas nuevas

### Archivos modificados
- `providers/agy/src/lib.rs` — `pub mod ping;` + re-exportaciones
- `crates/poller/src/lib.rs` — `pub mod pinger;`
- `crates/poller/src/scheduler.rs` — llama `pinger::maybe_ping_auto()` al final del ciclo

### Endpoints nuevos

| Ruta | Método | Descripción |
|------|--------|-------------|
| `POST /api/pings` | POST | Registra un ping (manual o automático desde el poller) |
| `GET /api/pings/:account_id?limit=20` | GET | Historial de pings de una cuenta |
| `GET /api/pings/needs?max_pct=5` | GET | Cuentas con percent_used < umbral (necesitan ping) |

### Variables de entorno del pinger

| Variable | Default | Descripción |
|----------|---------|-------------|
| `AKYV_PING_ENABLED` | `false` | Habilita ping automático global |
| `AKYV_PING_HOUR` | `8` | Hora local (0-23) para disparar el ping automático |
| `AKYV_PING_MAX_DAILY` | `2` | Máximo pings automáticos por cuenta por día |
| `AKYV_PING_MAX_PCT` | `5.0` | % máximo de uso para considerar que necesita ping |
| `AKYV_PING_MESSAGE` | `k` | Mensaje mínimo a enviar |

> [!IMPORTANT]
> El ping automático está **deshabilitado por defecto** (`AKYV_PING_ENABLED=false`).
> Se activa agregando `AKYV_PING_ENABLED=true` al `.env` del poller local.
> El VPS **nunca** hace pings — solo los registra.

### Hipótesis de cooldown activa
H1 — El contador de 7 días empieza con el **primer uso** de la ventana.
Una cuenta con 0 % de uso no tiene ventana activa; el ping la activa para
que el reinicio ocurra lo antes posible dentro de la semana.

### Verificación de endpoints

```powershell
# Ver cuentas que necesitan ping (percent_used < 5%)
curl.exe "http://198.211.97.243:3010/api/pings/needs"

# Historial de pings de una cuenta
curl.exe "http://198.211.97.243:3010/api/pings/batidospitayaclaude49%40gmail.com"

# Registrar ping manual (simulado)
curl.exe -X POST "http://198.211.97.243:3010/api/pings" `
  -H "Content-Type: application/json" `
  -d '{\"account_id\":\"batidospitayaclaude49@gmail.com\",\"device_id\":\"TABLET-R5MH20CH\",\"success\":true,\"duration_ms\":1200,\"is_manual\":true}'
```

---

## §17 — Fases A, C, D, E (2026-11-25)

### C — Cooldown Experiment: H1 Confirmada

Análisis con datos reales del VPS (quota_readings):

| Cuenta | Pool | % usado | reset_time (UTC) | Interpretación |
|--------|------|---------|-----------------|----------------|
| claude49 | claude-gpt-weekly | 41% | 2026-12-02T04:40:42Z | H1: reloj arrancó con primer uso |
| claude49 | gemini-weekly | 6% | 2026-12-02T04:54:05Z | H1: pool Gemini arrancó 13min después |
| claude45 | claude_gpt | 47% | 2026-11-25T05:57:08Z | Ventana 5h activa |
| claude45 | claude_gpt | 47% | 2026-12-02T00:57:08Z | Ventana semanal |

**Conclusión:** H1 confirmada. El `reset_time` aparece solo cuando hay consumo y es
independiente por pool. El ping automático tiene sentido para arrancar ventanas inactivas.

Documento actualizado: `docs/research/04-experimento-cooldown.md`

---

### A — Codex chats

**Archivos nuevos:**
- `providers/codex/src/chats.rs` — `scan_codex_chats()` lee `~/.codex/state_5.sqlite`
  tabla `threads`, extrae primer mensaje del usuario del JSON de messages, trunca a 120 chars.
  5 unit tests. Si la BD no existe o Codex no está instalado → `Ok(vec![])` sin error.

**Archivos modificados:**
- `providers/codex/src/lib.rs` — `pub mod chats` + re-export `scan_codex_chats, CodexChat`
- `providers/codex/Cargo.toml` — agrega `rusqlite = { version = "0.32", features = ["bundled"] }`
- `crates/poller/src/chat_collector.rs` — agrega `collect_and_upload_codex()`, ambas funciones
  comparten el helper `upload_chats()` privado

**Integración en scheduler:**
- Paso 6b en `run_cycle()`: busca snapshot de Codex, usa su `account_id` para subir chats

---

### D — Alert Log en servidor

**Archivos nuevos en servidor:**
- Tabla `alert_log` en `server/src/db.rs` (Fase D)
- Endpoints `POST /api/alerts` y `GET /api/alerts?account_id=&limit=20` en `routes.rs`
- Rutas en `main.rs`

**Integración en scheduler:**
- Función `upload_alerts_to_server()` al final de `run_cycle()`: evalúa umbral y sube las
  alertas que superan WARN/CRIT como registro histórico al servidor.

```powershell
# Ver alertas registradas
curl.exe "http://198.211.97.243:3010/api/alerts?limit=10"
curl.exe "http://198.211.97.243:3010/api/alerts?account_id=batidospitayaclaude49%40gmail.com"
```

---

### E/B — Account Switcher

**Archivos nuevos en servidor:**
- Tabla `switch_log` en `server/src/db.rs`
- Endpoints `POST /api/switch` y `GET /api/switch/history?limit=20` en `routes.rs`
- Rutas en `main.rs`

**Flujo del switch (ocurre en el PC local, no en el VPS):**
1. Usuario guarda credenciales de cuenta nueva en la bóveda (cifradas)
2. Con un click: `AccountSwitcher::switch(&new_credential)` → escribe en Credential Manager
3. Si falla: revierte a la credencial anterior automáticamente
4. El resultado se sube al servidor vía `POST /api/switch` como historial

```powershell
# Ver historial de switches
curl.exe "http://198.211.97.243:3010/api/switch/history"
```

> [!IMPORTANT]
> El VPS **nunca** maneja credenciales — solo registra el historial de switches.
> La ejecución de `AccountSwitcher::switch()` ocurre en el PC local del usuario.

---

### F — Codex token renewal

**Estado: Documentado como procedimiento manual.**

Cuando el token de Codex expira (`auth.json → tokens.expiry_time` vencido),
el poller lo marca como inválido (`percent_used = 100%`). Para renovarlo:

1. Abrir Codex en la terminal: `codex` (inicia sesión automática si hay sesión activa)
2. O bien: `codex login` para forzar re-autenticación OAuth
3. El nuevo `auth.json` se escribe automáticamente y el poller lo detecta en el siguiente ciclo

La renovación automática requiere llamar al endpoint OAuth de OpenAI con el `refresh_token`,
lo que implicaría manejar credenciales OAuth en el poller — riesgo de seguridad no justificado
para esta etapa. Se pospone a la Segunda Etapa si se requiere.


---

## §18 — Segunda Etapa: App de Escritorio v0.2.0 (2026-11-25)

### Resumen del estado al cierre de la segunda etapa

| Fase | Estado | Fecha | Descripcion |
|------|--------|-------|-------------|
| 10 — Scaffold Tauri 2 | OK | 2026-11-25 | Proyecto Tauri 2 + React/TS, iconos, CI, scripts, toolchain MSVC |
| 11 — Capa comunicacion Rust<->VPS | OK | 2026-11-25 | Comandos Tauri IPC, hooks useCommand, tipos PoolSummary |
| 12 — Boveda local | OK | 2026-11-25 | Argon2id, desbloqueo, creacion, recuperacion con clave de respaldo |
| 13 — Dashboard | OK | 2026-11-25 | QuotaCard, AppLayout, switch cuentas, poller IPC |
| 14 — Autobloqueo IPC | OK | 2026-11-25 | Timer Rust, evento Tauri autolock, lock_vault con zeroize |
| 15 — Settings + Vault Sync | OK | 2026-11-25 | Configuracion persistida, sincronizacion boveda VPS |
| 16 — Build + CI verde | OK | 2026-11-25 | .exe instalado en Windows, CI en verde, v0.2.0 |

### Hallazgos criticos durante el desarrollo

| Error/Hallazgo | Causa | Solucion |
|----------------|-------|----------|
| `resource path binaries\ai-keyvault-poller-x86_64-pc-windows-gnu.exe doesn't exist` | `rust-toolchain.toml` solo en `src-tauri/`, no en `apps/desktop/` | Crear `apps/desktop/rust-toolchain.toml` (mismo contenido) |
| `ERR_PNPM_IGNORED_BUILDS` en CI | esbuild necesita aprobacion; lockfile no la tenia | `pnpm approve-builds --all` + `.npmrc` con `onlyBuiltDependencies[]=esbuild` |
| `No test files found, exiting with code 1` (Vitest) | No habia archivos `*.test.ts` | Crear `apps/desktop/src/types/api.test.ts` con tests iniciales |
| `Error HTTP: error decoding response body` en dashboard | Campos de `PoolSummary` no coincidian con VPS | Sincronizar `current_percent` -> `current_percent_used`, etc. |
| PS parse errors en `copy-sidecar.ps1` | Emojis UTF-8 leidos como Windows-1252 en PS 5.1 | Reemplazar emojis con prefijos ASCII (`[INFO]`, `[OK]`, `[ERROR]`) |
| `rustup target add x86_64-pc-windows-msvc` requerido | Target MSVC no instalado por defecto | Correr antes del primer `cargo build --target x86_64-pc-windows-msvc` |
| `cmake` no instalado | providers/agy y providers/codex usan rusqlite bundled | Excluir esos providers del `Cargo.toml` del desktop hasta instalar cmake |

### Struct PoolSummary — campos reales del VPS

```rust
// server/src/stats.rs (VPS)
pub struct PoolSummary {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub current_percent_used: f64,    // era: current_percent
    pub tokens_used: i64,             // nuevo campo
    pub tokens_limit: i64,            // nuevo campo
    pub burn_rate_pct_per_hour: f64,  // era: burn_rate_per_hour
    pub last_read_at: String,         // era: Option<String>
    // ...otros campos de proyeccion
}
```

> [!NOTE]
> Si se agrega un campo nuevo a `PoolSummary` en el servidor, debe actualizarse en:
> - `apps/desktop/src-tauri/src/api_client.rs` (struct Rust)
> - `apps/desktop/src/types/api.ts` (tipo TypeScript)
> - Cualquier componente que use el campo

---

## §19 — Autobloqueo IPC (Fase 14)

### Como funciona

```
UI (inactiva) --> timer Rust caduca --> evento Tauri "autolock"
              --> App.tsx listener --> lock_vault() --> zeroize
              --> navigate("/unlock")
              [Poller sigue corriendo independientemente]
```

**Configuracion en Settings:**
- `autolockMinutes = 0` = deshabilitado (valor por defecto — opt-in)
- Rango configurado por el usuario (ej. 15 min)
- Se guarda en `localStorage` y se pasa a `start_autolock_timer` en Rust al guardar

**Comandos Tauri involucrados:**

| Comando | Descripcion |
|---------|-------------|
| `start_autolock_timer(minutes)` | Inicia el timer Rust; `minutes=0` cancela |
| `reset_autolock_timer()` | Reinicia el timer desde 0 (en mousemove/keydown, throttled 30s) |
| `cancel_autolock_timer()` | Cancela el timer sin bloquearse |
| `lock_vault()` | Zeroiza la clave maestra en memoria |

**Evento Tauri:**
```rust
// En src-tauri/src/commands/security.rs
app_handle.emit("autolock", ()).ok();
```

```typescript
// En App.tsx
listen("autolock", async () => {
    await invoke("lock_vault");
    navigate("/unlock");
});
```

> [!IMPORTANT]
> El autobloqueo bloquea **solo la UI** (zeroiza la clave en memoria, navega a /unlock).
> El poller sidecar sigue corriendo y enviando lecturas al VPS.
> Las credenciales del poller vienen del Credential Manager de Windows, no de la boveda.

---

## §20 — Vault Sync VPS (Fase 15)

### Como funciona

La boveda local (cifrada con la clave maestra del usuario) se puede sincronizar al VPS.
Es conocimiento cero: el VPS solo almacena el blob cifrado, nunca la clave.

**Endpoints:**

| Metodo | Ruta | Descripcion |
|--------|------|-------------|
| `POST /api/vault/sync` | Sube snapshot cifrado | Body: `{device_id, ciphertext_b64, nonce_b64, kdf_params}` |
| `GET /api/vault/sync` | Descarga ultimo snapshot | Retorna el blob mas reciente por `device_id` |

**Comandos Tauri:**

| Comando | Descripcion |
|---------|-------------|
| `upload_vault_sync()` | Cifra la boveda local y la sube al VPS |
| `download_vault_sync()` | Descarga el snapshot mas reciente y lo fusiona localmente |

**Flujo de desbloqueo con sync:**
1. Usuario introduce clave maestra -> `unlock_vault()` OK
2. App llama `start_autolock_timer` (si > 0 min configurado)
3. App llama `download_vault_sync()` silenciosamente (en background)
4. Si hay datos nuevos en VPS, se fusionan en la boveda local

**Acceso manual:**
- En `/settings` hay botones "Subir boveda" y "Descargar boveda"
- Util para transferir credenciales entre PCs

> [!CAUTION]
> La sincronizacion de boveda no reemplaza la clave maestra: cada dispositivo debe
> tener la misma clave maestra para descifrar el blob descargado.
> Si se pierde la clave, la boveda (local y en VPS) es irrecuperable.

---

## §21 — Fuente de verdad: la Bóveda (corrección arquitectura)

> [!IMPORTANT]
> El Dashboard usa la **bóveda** como fuente de cuentas, NO el VPS directamente.

**Flujo real:**
```
Usuario agrega cuenta →
  import_current_agy_credential / import_current_codex_credential
        |
        v (cifrado XChaCha20)
  vault.json local + sync VPS (upload_vault_sync)
        |
        v (list_vault_accounts)
  Dashboard / AccountsPage / ChatsPage / AlertsPage
        |
        (cruce por alias)
        v
  VPS: datos de cuota / chats / alertas para esas cuentas
```

El poller sigue leyendo del OS (Credential Manager / auth.json) directamente,
SIN pasar por la bóveda. Los datos de cuota, chat y alertas se envían al VPS.
El desktop cruza: `account.alias == pool.account_id` para mostrar cuota de cada cuenta.

> [!NOTE]
> Si el Dashboard muestra "Sin datos de cuota aún", significa que el poller no ha
> corrido todavía para esa cuenta. Los datos llegan solos cuando el poller está activo.

---

## §22 — Arquitectura real corregida (Fase 17 — 2026-11-25)

### Flujo de agregar cuenta (único flujo correcto)

**Para agy (Antigravity):**
1. Usuario inicia sesión en agy en esta PC (cualquier cuenta)
2. Hace clic en "🗝️ Mi Bóveda" → "+ Agregar cuenta" → elige "Antigravity"
3. Escribe un alias (ej: `trabajo`, `personal`)
4. Rust lee el blob de `gemini:antigravity` en Credential Manager de Windows
5. Lo cifra con XChaCha20 y lo guarda en vault.json
6. Sincroniza al VPS (best-effort)

**Para Codex:**
1. Usuario inicia sesión en Codex en esta PC
2. "+ Agregar cuenta" → elige "Codex"
3. Rust lee `~/.codex/auth.json`
4. Lo cifra y guarda en vault.json + sync VPS

### Flujo de switch de cuenta

1. Usuario va al Dashboard → "⚡ Usar en esta PC"
2. Rust llama `switch_account(account_id, provider, device_id)`
3. Descifra `encrypted_tokens` de la bóveda
4. Escribe el blob en Credential Manager / auth.json
5. Registra en VPS `POST /api/switch`
6. Avisa al usuario que reinicie las ventanas del agente

> [!WARNING]
> El switch NUNCA cierra procesos. Solo escribe las credenciales en el OS.
> El usuario debe reiniciar las ventanas de agy / Codex para que surta efecto.

### VPS endpoints — campos reales (verificados en server/src/routes.rs)

| Endpoint | Campos de respuesta |
|---|---|
| `GET /api/alerts` | `id, account_id, provider, pool, level, percent_used, reset_time, notified_toast (0/1→bool), notified_ntfy (0/1→bool), detected_at` |
| `GET /api/switch/history` | `id, to_account_id, from_account_id, provider, device_id, success (0/1→bool), reason, switched_at` |
| `POST /api/switch` | body: `{to_account_id, from_account_id, provider, device_id, success, reason}` |
| `GET /api/pings/needs` | `[{account_id, provider, pool, percent_used, needs_ping, last_read_at, reset_time}]` |
| `GET /api/chats/:account_id` | `{id, account_id, provider, title, created_at, updated_at, archived (0/1→bool)}` — sin device_id |

> [!NOTE]
> SQLite guarda booleans como enteros `0/1`. El cliente Rust usa el deserializador
> personalizado `de_bool_or_int` en `api_client.rs` para convertirlos correctamente.

### providers/agy como dependencia Rust

- `providers/agy` usa solo `windows_sys` → NO requiere cmake → se puede incluir en desktop
- `providers/codex` requiere `rusqlite bundled` → SÍ requiere cmake → excluido hasta instalarlo

### Comandos Rust disponibles (v0.2.4)

| Comando | Descripción |
|---|---|
| `import_current_agy_credential(alias)` | Captura blob de Credential Manager → cifra → vault |
| `import_current_codex_credential(alias)` | Captura ~/.codex/auth.json → cifra → vault |
| `add_agy_account_oauth(alias)` | Flujo OAuth completo: limpia → abre agy → polling → captura |
| `cancel_agy_oauth()` | Cancela el flujo OAuth en curso |
| `decrypt_account_tokens(uuid)` | Descifra encrypted_tokens (uso interno del switch) |
| `switch_account(account_id, provider, device_id)` | Switch real desde bóveda |
| `list_vault_accounts()` | Lista cuentas descifradas (alias + provider, sin credenciales) |
| `upload_vault_sync / download_vault_sync` | Sync cifrado de bóveda al VPS (conocimiento cero) |

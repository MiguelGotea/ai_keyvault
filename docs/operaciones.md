# Operaciones â€” AI KeyVault

**Fecha:** 2026-11-22  
**Ãšltima actualizaciÃ³n:** 2026-11-24  
**VersiÃ³n:** 1.2

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
| \server/Cargo.toml\ | Eliminada dependencia \and\ sin uso (solo se usa en \crates/core\) |
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

# Operaciones — AI KeyVault

**Fecha:** 2026-11-22  
**Última actualización:** 2026-11-24  
**Versión:** 1.2

> Este documento describe cómo administrar el VPS, hacer deploys, gestionar respaldos y restaurar datos.
> También registra todos los cambios manuales que se han hecho directamente en el VPS (fuera del repositorio),
> para que al replicar en otra PC o reconstruir el VPS no falte nada.

---

## 1. Deploy del servidor (automático)

El deploy es automático al hacer push a `main` en el repo `ai_keyvault`.

**Flujo:**
1. GitHub Actions compila el binario Rust para Linux x64.
2. rsync sube el binario a `/opt/ai-keyvault/bin/`.
3. SSH ejecuta `systemctl restart ai-keyvault` (via polkit — sin sudo).
4. Verifica con `curl localhost:3010/health`.

**Para hacer push manual:**
```powershell
# Desde la carpeta del repo
.\.scripts\gitpush.ps1 "feat(server): descripción del cambio"
```

---

## 2. Acceso directo al VPS

```bash
ssh root@198.211.97.243
# O con el usuario de deploy:
ssh ai-keyvault-deploy@198.211.97.243
```

---

## 3. Gestión del servicio

```bash
# Estado del servicio
systemctl status ai-keyvault

# Logs en tiempo real
journalctl -u ai-keyvault -f

# Reiniciar (como ai-keyvault-deploy, sin sudo gracias a polkit)
systemctl restart ai-keyvault

# Reiniciar (como root)
systemctl restart ai-keyvault

# Ver logs históricos
journalctl -u ai-keyvault --since "1 hour ago"

# Si el servicio queda en estado "failed" tras muchos reinicios rápidos:
systemctl reset-failed ai-keyvault
systemctl start ai-keyvault
```

---

## 4. Configuración manual del VPS (registro de cambios fuera del repositorio)

> ⚠️ Esta sección documenta todo lo que se configuró manualmente en el VPS por SSH,
> fuera de lo que maneja el repositorio. Si el VPS se recrea o se migra a otra PC,
> hay que ejecutar estos pasos antes de que el deploy automático funcione.

### 4.1 Estructura de directorios (Fase 0 — 2026-11-22)

```bash
mkdir -p /opt/ai-keyvault/{bin,config,data,logs,backups}
useradd --system --no-create-home --shell /usr/sbin/nologin ai-keyvault
chown -R ai-keyvault:ai-keyvault /opt/ai-keyvault/
```

### 4.2 Usuario de deploy (Fase 0 — 2026-11-22)

```bash
useradd --no-create-home --shell /bin/bash ai-keyvault-deploy
mkdir -p /home/ai-keyvault-deploy/.ssh
# Pegar la llave pública del secret VPS_SSH_KEY:
echo "ssh-ed25519 AAAA... ai-keyvault-deploy" >> /home/ai-keyvault-deploy/.ssh/authorized_keys
chmod 700 /home/ai-keyvault-deploy/.ssh
chmod 600 /home/ai-keyvault-deploy/.ssh/authorized_keys
chown -R ai-keyvault-deploy:ai-keyvault-deploy /home/ai-keyvault-deploy/
```

### 4.3 Archivo `.env` del servidor (Fase 0 — 2026-11-22, actualizado Fase 2 — 2026-11-24)

Ruta: `/opt/ai-keyvault/.env`  
Propietario: `ai-keyvault:ai-keyvault`, permisos: `600`

```env
DATABASE_URL=sqlite:./data/keyvault.db?mode=rwc
JWT_SECRET=<256 bits aleatorios — generar con: openssl rand -hex 32>
SERVER_MASTER_KEY=<32 bytes aleatorios — generar con: openssl rand -hex 32>
PORT=3010
LOG_LEVEL=info
```

> **Nota (2026-11-24):** Se agregó `?mode=rwc` a `DATABASE_URL` para que SQLite cree la BD si no existe.
> Sin este parámetro, el servidor falla al arrancar con `SqliteError { code: 14, unable to open database file }`.

Creación del archivo de BD:
```bash
touch /opt/ai-keyvault/data/keyvault.db
chown ai-keyvault:ai-keyvault /opt/ai-keyvault/data/keyvault.db
# permisos 644 por defecto — el servicio puede leer/escribir como propietario
```

### 4.4 Servicio systemd (Fase 2 — 2026-11-24)

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

### 4.5 Permisos del directorio `bin/` (Fase 2 — 2026-11-24)

El usuario `ai-keyvault-deploy` necesita escribir en `bin/` (para el rsync del deploy),
y el usuario `ai-keyvault` necesita ejecutar el binario.

```bash
# bin/ pertenece al usuario de deploy, grupo del servicio
chown ai-keyvault-deploy:ai-keyvault /opt/ai-keyvault/bin
chmod 750 /opt/ai-keyvault/bin

# Agregar al usuario de servicio al grupo del deploy para que pueda entrar al directorio
usermod -aG ai-keyvault-deploy ai-keyvault
```

### 4.6 Regla polkit — reinicio sin sudo (Fase 2 — 2026-11-24)

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

> **Por qué polkit y no sudo:** El deploy automático (GitHub Actions) conecta por SSH
> sin pseudo-terminal (TTY). `sudo` requiere TTY para leer contraseña aunque tenga NOPASSWD.
> Polkit autoriza directamente a systemd sin pasar por sudo, eliminando el problema.

Pasos para crearlo:
```bash
printf 'polkit.addRule(function(action, subject) {\n    if (action.id == "org.freedesktop.systemd1.manage-units" &&\n        action.lookup("unit") == "ai-keyvault.service" &&\n        subject.user == "ai-keyvault-deploy") {\n        return polkit.Result.YES;\n    }\n});\n' > /etc/polkit-1/rules.d/50-ai-keyvault.rules
chmod 644 /etc/polkit-1/rules.d/50-ai-keyvault.rules
```

### 4.7 Sudoers de respaldo (aún presente, aunque ya no se usa para el deploy)

Ruta: `/etc/sudoers.d/ai-keyvault-deploy`  
Permisos: `0440` (requerido por sudo)

```
Defaults:ai-keyvault-deploy !requiretty
ai-keyvault-deploy ALL=(ALL) NOPASSWD: /usr/bin/systemctl restart ai-keyvault, /usr/bin/systemctl status ai-keyvault, /bin/systemctl restart ai-keyvault, /bin/systemctl status ai-keyvault
```

### 4.8 Firewall UFW (Fase 2 — 2026-11-24)

```bash
ufw allow 3010/tcp comment 'AI KeyVault Server'
# (80 y 443 ya estaban abiertos desde Fase 0 para Caddy)
```

> **Nota:** Se usa IP directa + puerto 3010 sin subdominio ni reverse proxy (decisión de arquitectura).

### 4.9 Caddy instalado pero no configurado

Caddy está instalado (`apt install caddy`) pero el Caddyfile no fue configurado porque
se decidió no usar subdominio. No interfiere con el servidor — escucha en puerto 80.

---

## 4.10 Cambios de Fase 3 — Cierre (2026-11-24)

No hubo cambios directos al VPS en Fase 3. Los cambios fueron todos en el repositorio:

| Elemento | Cambio |
|----------|--------|
| `providers/agy/` | Crate nuevo — Credential Manager Win32, parser `/usage`, switcher |
| `providers/codex/` | Crate nuevo — auth.json atómico, JWT decode, switcher |
| `providers/agy/tests/integration.rs` | Tests de integración real (marcados `#[ignore]`) |
| `providers/codex/tests/integration.rs` | Tests de integración real (marcados `#[ignore]`) |
| `.github/workflows/ci.yml` | Quitados `continue-on-error` en fmt/clippy/test; `cargo audit` lee `.cargo/audit.toml` |
| `server/Cargo.toml` | Eliminada dependencia `rand` sin uso |
| `.cargo/audit.toml` | Creado — ignora `RUSTSEC-2023-0071` (`rsa` transitivo) y `RUSTSEC-2026-0459` (`libsqlite3-sys` 0.30.1 fijado por `sqlx 0.8.6`) |
| `Cargo.lock` | Actualizado con `cargo update` |

**Para correr tests de integración localmente (Windows):**
```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo test --package ai-keyvault-provider-agy -- --include-ignored
cargo test --package ai-keyvault-provider-codex -- --include-ignored
```

---

## 5. Respaldo

### 5.1 Respaldo automático (GitHub Actions — diario)

Un workflow en `.github/workflows/backup.yml` se ejecuta diariamente a las 4:00 UTC:
1. Conecta al VPS por SSH y descarga `/opt/ai-keyvault/data/keyvault.db`.
2. Hace commit del archivo en el repo `ai_keyvault_data` usando el secret `BACKUP_WRITE_KEY`.
3. Retiene los últimos 30 snapshots (elimina los más viejos automáticamente).

> **¿Es seguro subir la BD a GitHub?**  
> Sí. Los `vault_snapshots` contienen blobs ya cifrados con XChaCha20-Poly1305 (Fase 1).
> El servidor nunca tiene la clave maestra. El backup es un ciphertext opaco.

**Retención:** 30 días de snapshots. Los más viejos se eliminan automáticamente.

### 5.2 Respaldo manual

```bash
# En el VPS (como root)
cp /opt/ai-keyvault/data/keyvault.db /opt/ai-keyvault/backups/snapshot-manual-$(date +%Y-%m-%d).db
```

---

## 6. Restauración desde respaldo

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

## 7. Rotación de claves del servidor

### 7.1 JWT_SECRET

```bash
# Generar nuevo secreto
openssl rand -hex 32

# Editar .env en el VPS
nano /opt/ai-keyvault/.env
# Cambiar JWT_SECRET=...

# Reiniciar (invalida todos los tokens JWT existentes — los clientes tendrán que re-autenticarse)
systemctl restart ai-keyvault
```

### 7.2 SERVER_MASTER_KEY

> ⚠️ Cambiar esta clave invalida todos los refresh tokens delegados al VPS.
> Los usuarios deberán re-delegar sus cuentas.

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

## 9. Actualización del servidor

```bash
# Automática: hacer push a main desde la PC de desarrollo
# El GitHub Action compila y despliega

# Manual de emergencia:
# 1. Compilar en la PC de desarrollo (workspace — binario en target/release/)
cargo build --release --package ai-keyvault-server
# 2. Subir binario
scp target/release/ai-keyvault-server root@198.211.97.243:/opt/ai-keyvault/bin/
# NOTA: requiere que el usuario de root tenga acceso a bin/
# (bin/ pertenece a ai-keyvault-deploy; copiar primero a /tmp y luego mover como root)
# 3. Reiniciar
ssh root@198.211.97.243 "systemctl restart ai-keyvault"
```

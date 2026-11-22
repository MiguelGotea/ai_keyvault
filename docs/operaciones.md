# Operaciones — AI KeyVault

**Fecha:** 2026-11-22
**Versión:** 1.0

> Este documento describe cómo administrar el VPS, hacer deploys, gestionar respaldos y restaurar datos.

---

## 1. Deploy del servidor (automático)

El deploy es automático al hacer push a `main` en el repo `ai_keyvault`.

**Flujo:**
1. GitHub Actions compila el binario Rust para Linux x64.
2. rsync sube el binario a `/opt/ai-keyvault/bin/`.
3. SSH ejecuta `sudo systemctl restart ai-keyvault`.
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

# Reiniciar
sudo systemctl restart ai-keyvault

# Ver logs históricos
journalctl -u ai-keyvault --since "1 hour ago"
```

---

## 4. Respaldo

### 4.1 Respaldo automático (diario)

Un systemd timer ejecuta diariamente:
1. Copia la BD SQLite a `/opt/ai-keyvault/backups/snapshot-YYYY-MM-DD.db`.
2. La cifra con la clave pública del usuario.
3. La sube al repo `ai_keyvault_data` vía deploy key de solo escritura.

**Retención:** 30 días de snapshots. Los más viejos se eliminan automáticamente.

### 4.2 Respaldo manual

```bash
# En el VPS
cp /opt/ai-keyvault/data/keyvault.db /opt/ai-keyvault/backups/snapshot-manual-$(date +%Y-%m-%d).db
```

---

## 5. Restauración desde respaldo

### 5.1 Desde ai_keyvault_data

```bash
# 1. Clonar el repo de datos en la PC
git clone https://github.com/MiguelGotea/ai_keyvault_data.git

# 2. Copiar el snapshot al VPS
scp backups/snapshot-YYYY-MM-DD.db.enc root@198.211.97.243:/opt/ai-keyvault/backups/

# 3. En el VPS: descifrar el snapshot
# (con la clave privada correspondiente)
openssl rsautl -decrypt -inkey ~/.ssh/backup_key -in snapshot.db.enc -out snapshot.db

# 4. Reemplazar la BD activa
systemctl stop ai-keyvault
cp /opt/ai-keyvault/data/keyvault.db /opt/ai-keyvault/data/keyvault.db.bak
cp snapshot.db /opt/ai-keyvault/data/keyvault.db
chown ai-keyvault:ai-keyvault /opt/ai-keyvault/data/keyvault.db
systemctl start ai-keyvault
```

---

## 6. Rotación de claves del servidor

### 6.1 JWT_SECRET

```bash
# Generar nuevo secreto
openssl rand -hex 32

# Editar .env en el VPS
nano /opt/ai-keyvault/.env
# Cambiar JWT_SECRET=...

# Reiniciar (invalida todos los tokens JWT existentes — los clientes tendrán que re-autenticarse)
systemctl restart ai-keyvault
```

### 6.2 SERVER_MASTER_KEY

> ⚠️ Cambiar esta clave invalida todos los refresh tokens delegados al VPS.
> Los usuarios deberán re-delegar sus cuentas.

```bash
openssl rand -hex 32
# Editar .env y cambiar SERVER_MASTER_KEY
systemctl restart ai-keyvault
# Notificar a los usuarios que re-deleguen sus cuentas
```

---

## 7. Monitoreo

```bash
# Uso de disco
df -h /opt/ai-keyvault/

# Uso de BD
ls -lh /opt/ai-keyvault/data/keyvault.db

# Conexiones activas
ss -tlnp | grep 3010

# fail2ban
fail2ban-client status ai-keyvault-auth
```

---

## 8. Actualización del servidor

```bash
# Automática: hacer push a main desde la PC de desarrollo
# El GitHub Action compila y despliega

# Manual de emergencia:
# 1. Compilar en la PC de desarrollo
cargo build --release --manifest-path server/Cargo.toml
# 2. Subir binario
scp target/release/ai-keyvault-server root@198.211.97.243:/opt/ai-keyvault/bin/
# 3. Reiniciar
ssh root@198.211.97.243 "systemctl restart ai-keyvault"
```

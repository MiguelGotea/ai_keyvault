# Investigación: VPS DigitalOcean — Estructura y aislamiento

**Fecha:** 2026-11-22
**IP del VPS:** 198.211.97.243
**Estado:** Exploración parcial — SSH no disponible desde el sandbox de desarrollo. Basado en proyectos existentes.

---

## 1. Proyectos existentes en el VPS

Basado en el análisis de los repositorios de referencia (`retails_nicaragua` y `GMBWorker`):

| Proyecto | Ruta en VPS | Puerto | Stack | Gestor |
|----------|-------------|--------|-------|--------|
| Retails Nicaragua | `/root/retails-nicaragua/` | Puerto no especificado | Node.js | PM2 |
| GMBWorker | `/opt/gmb-worker/` | 3009 | Node.js | PM2 |
| **AI KeyVault Server** | `/opt/ai-keyvault/` | **3010** (interno) | Rust/axum | systemd |

> **Principio de aislamiento:** AI KeyVault vive en `/opt/ai-keyvault/` — completamente separado de los otros proyectos. Si se elimina `/opt/ai-keyvault/`, no afecta a Retails Nicaragua ni a GMBWorker.

---

## 2. Decisiones de aislamiento

### 2.1 Directorio del servicio

```
/opt/ai-keyvault/
  bin/              # binario compilado del servidor
  config/           # configuración (sin secretos; los secretos van en .env)
  data/             # SQLite database (si se usa SQLite)
  logs/             # logs rotativos
  .env              # secretos reales (nunca en el repo)
  backups/          # snapshots locales antes de subirlos a ai_keyvault_data
```

### 2.2 Usuario de sistema dedicado

```bash
# Crear usuario no root para el servicio
useradd --system --no-create-home --shell /usr/sbin/nologin ai-keyvault
chown -R ai-keyvault:ai-keyvault /opt/ai-keyvault/
```

### 2.3 Servicio systemd con sandboxing

```ini
# /etc/systemd/system/ai-keyvault.service
[Unit]
Description=AI KeyVault Server
After=network.target

[Service]
Type=simple
User=ai-keyvault
Group=ai-keyvault
WorkingDirectory=/opt/ai-keyvault
EnvironmentFile=/opt/ai-keyvault/.env
ExecStart=/opt/ai-keyvault/bin/ai-keyvault-server
Restart=on-failure
RestartSec=5s

# Sandboxing
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/opt/ai-keyvault/data /opt/ai-keyvault/logs /opt/ai-keyvault/backups
ProtectHome=true
CapabilityBoundingSet=
AmbientCapabilities=

[Install]
WantedBy=multi-user.target
```

### 2.4 Firewall (ufw)

```bash
# Solo abrir el puerto de Caddy (80/443) — el puerto 3010 NO está expuesto directo
ufw allow 80/tcp
ufw allow 443/tcp
# Puerto 3010: solo accesible desde localhost (Caddy hace el proxy)
```

### 2.5 Caddy como reverse proxy con TLS automático

```
# /etc/caddy/Caddyfile (sección para AI KeyVault)
keyvault.tudominio.com {
    reverse_proxy localhost:3010
    # TLS automático via Let's Encrypt
}
```

> **Subdominio:** se configura en Spaceship apuntando a 198.211.97.243. Si no hay dominio disponible de inmediato, el servidor funciona con IP directa + certificado self-signed hasta configurar el dominio.

---

## 3. Comparación con proyectos existentes

| Aspecto | Retails Nicaragua | GMBWorker | **AI KeyVault** |
|---------|-------------------|-----------|-----------------|
| Ruta | /root/retails-nicaragua | /opt/gmb-worker | **/opt/ai-keyvault** |
| Usuario | root | root | **ai-keyvault (no root)** |
| Gestor | PM2 | PM2 | **systemd** |
| Puerto | (en config) | 3009 | **3010 (solo localhost)** |
| Deploy | git pull | rsync | **binary release** |
| TLS | — | — | **Caddy** |

---

## 4. Deploy con GitHub Actions

Basado en el patrón de GMBWorker (rsync + SSH), pero adaptado para un binario compilado:

```yaml
# .github/workflows/deploy-server.yml (esquema)
# 1. Compilar el binario en ubuntu-latest (Rust cross-compile o nativo)
# 2. rsync del binario a /opt/ai-keyvault/bin/
# 3. SSH: systemctl restart ai-keyvault
# 4. Verificar: curl localhost:3010/health
```

**Secrets requeridos en GitHub:**
| Secret | Descripción |
|--------|-------------|
| `VPS_HOST` | 198.211.97.243 |
| `VPS_USER` | ai-keyvault-deploy (usuario con solo permisos de rsync + systemctl restart) |
| `VPS_SSH_KEY` | Llave privada SSH restringida al deploy |
| `VPS_PATH` | /opt/ai-keyvault |

> La llave SSH del deploy está restringida (`command=` en `authorized_keys`) para solo ejecutar el script de reinicio del servicio. No tiene acceso de shell completo.

---

## 5. Respaldo cifrado a ai_keyvault_data

**Proceso:**
1. El VPS (cron o systemd timer, diario) exporta un snapshot de la BD SQLite.
2. Lo cifra con la clave pública de respaldo (el usuario tiene la privada en su PC).
3. Lo sube a `ai_keyvault_data` usando una **deploy key de solo escritura**.

**Ruta del snapshot:**
```
/opt/ai-keyvault/backups/snapshot-YYYY-MM-DD.db.enc
```

**GitHub Actions en ai_keyvault_data:**
```yaml
# .github/workflows/backup-upload.yml
# Triggered by push from VPS via deploy key
# Solo acepta pushes de la deploy key de respaldo
# Mantiene retención de N snapshots (elimina los más viejos)
```

**Prueba de restauración (automatizada):**
- Una vez por semana, un job descarga el último snapshot, lo descifra con una clave de test, importa la BD y verifica integridad.
- El resultado se reporta en el log del job.

---

## 6. fail2ban

```
# /etc/fail2ban/jail.local
[ai-keyvault-auth]
enabled = true
port = 443
filter = ai-keyvault-auth
logpath = /opt/ai-keyvault/logs/auth.log
maxretry = 5
bantime = 3600
findtime = 300
```

---

## 7. Secretos del servidor (en VPS, nunca en repo)

```
/opt/ai-keyvault/.env
─────────────────────
DATABASE_URL=sqlite:./data/keyvault.db
JWT_SECRET=<256 bits aleatorios>
SERVER_MASTER_KEY=<clave para cifrar delegaciones de refresh tokens>
NTFY_ADMIN_TOKEN=<si el VPS necesita enviar notificaciones de sistema>
```

> Estos secretos se generan durante el setup manual del VPS y se documentan solo en `docs/operaciones.md` (sin los valores reales, solo instrucciones de cómo generarlos).

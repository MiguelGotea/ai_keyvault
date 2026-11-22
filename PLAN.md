# PLAN.md — Plan de Desarrollo de AI KeyVault

**Versión:** 0.1 (Fase 0 en progreso)
**Última actualización:** 2026-11-22
**Estado:** ⏳ Esperando aprobación del usuario para continuar a Fase 1

---

## Resumen ejecutivo

AI KeyVault es una bóveda + tablero personal para administrar cuentas de agentes de IA de programación. Stack: **Tauri 2** (Rust + TypeScript) para escritorio, **Rust/axum** para el servidor VPS, **SQLite** como BD, **XChaCha20-Poly1305** para cifrado, **Argon2id** como KDF. Monorepo en `ai_keyvault`.

---

## Estado actual: Fase 0 — Investigación

### Completado ✅
- [x] CLAUDE.md con todas las reglas de trabajo
- [x] README.md con alcance ético y documentación
- [x] THIRD_PARTY_NOTICES.md con licencias de referencia
- [x] Estructura de directorios del monorepo
- [x] Logos copiados a `assets/branding/`
- [x] docs/research/01-agy-investigacion.md
- [x] docs/research/02-codex-investigacion.md
- [x] docs/research/03-otros-proveedores.md
- [x] docs/research/04-experimento-cooldown.md (protocolo listo, datos pendientes)
- [x] docs/research/05-vps-estructura.md
- [x] docs/adr/ADR-001-stack.md (Tauri 2 elegido y justificado)
- [x] docs/adr/ADR-002-cifrado.md (Argon2id + XChaCha20-Poly1305)
- [x] docs/adr/ADR-003-sincronizacion.md (lease, CRDT LWW, Ed25519)
- [x] docs/adr/ADR-004-ping-alertas.md (configurable por hipótesis H1/H2/H3)
- [x] docs/threat-model.md (6 escenarios cubiertos)
- [x] agent/cooldown-logger/ (script Python para el experimento)
- [x] .gitignore estricto
- [x] .github/workflows/ci.yml (CI con gitleaks)

### Pendiente (acciones manuales del usuario) ⏳
- [ ] **Experimento de cooldown:** leer `docs/research/04-experimento-cooldown.md` y ejecutar el protocolo
- [ ] **Verificar estructura real de agy:** `dir "$env:APPDATA\Gemini" -Recurse -Name | Select-Object -First 30` y `dir "$env:USERPROFILE\.gemini" -Recurse -Name | Select-Object -First 30`
- [ ] **Verificar estructura de Codex:** `dir "$env:USERPROFILE\.codex" -Recurse -Name | Select-Object -First 50`
- [ ] **Configurar secrets en GitHub Actions** (ver sección "Secrets necesarios")
- [ ] **Setup inicial del VPS** (ver sección "Setup del VPS")

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

### Fase 1 — Núcleo seguro
**Contenido:**
- Crate `crates/core/`: Vault (cifrado, descifrado, Argon2id)
- Clave maestra → Argon2id → CDM + auth_key → Vault Key wrapping
- Clave de respaldo: generación, formato legible, exportación
- Autobloqueo con zeroize
- Portapapeles con limpieza automática
- Tests de cifrado/recuperación
- CI con gitleaks

**Criterios de aceptación:**
- [ ] Tests de cifrado pasan: encrypt/decrypt de todos los tipos de campo
- [ ] Test de recuperación con clave de respaldo
- [ ] Test de que nada sensible aparece en logs ni disco en claro
- [ ] `gitleaks detect` limpio
- [ ] `cargo audit` limpio

**Tiempo estimado:** 3-4 días

### Fase 2 — Servidor VPS
**Contenido:**
- Crate `server/`: API axum con SQLite
- Endpoints: register_device, sync_vault, get_quota_readings, post_quota_reading, quota_lease
- Autenticación Ed25519 de dispositivos
- Deploy con GitHub Actions (binario compilado)
- Respaldo cifrado a ai_keyvault_data
- Prueba de restauración automática

**Criterios de aceptación:**
- [ ] Entrar desde una segunda PC con URL + clave maestra y ver las mismas cuentas
- [ ] El servidor no puede descifrar nada (demostrado en test)
- [ ] Deploy automático funciona: push → binario en VPS
- [ ] Prueba de restauración pasa

**Tiempo estimado:** 4-5 días

### Fase 3 — Adaptadores agy + Codex
**Contenido:**
- Provider `providers/agy/`: Credential Manager, leer/escribir login, verificar con `agy models`, leer cuota
- Provider `providers/codex/`: leer/escribir auth.json (atómico), verificar, leer cuota
- Login guiado (una sola vez)
- Cambio de cuenta con deshacer
- Tests con `fake_agy` y `fake_codex`

**Criterios de aceptación:**
- [ ] Cambio real de cuenta de agy probado (Credential Manager)
- [ ] Cambio real de cuenta de Codex probado (auth.json atómico)
- [ ] Sin tocar procesos en ejecución
- [ ] Revertir funciona si el token es rechazado

**Tiempo estimado:** 3-4 días

### Fase 4 — Cuota y planes
**Contenido:**
- Consulta programada (10/60 min por cuenta)
- Lease anti-duplicado en el servidor
- Panel principal: tarjetas de cuentas con % por ventana
- Detalle de cuenta: resumen, cuota/cooldown
- "Actualizado hace X por PC-Y"

**Criterios de aceptación:**
- [ ] Datos coherentes entre PCs (mismo % visto desde dos PCs)
- [ ] Lecturas con fuente y antigüedad visibles
- [ ] Lease funciona: solo un consultador activo por cuenta

**Tiempo estimado:** 3 días

### Fase 5 — Alertas
**Contenido:**
- Notificaciones de Windows (toast notifications)
- ntfy.sh con tópico aleatorio guardado en bóveda
- Alerta al 80% (previa) y 95% (crítica)
- Sugerencia de cuenta alternativa
- Alerta de fin de suscripción (7 días, 1 día)
- Anti-ruido: sin spam, enfriamiento

**Criterios de aceptación:**
- [ ] Alertas de prueba funcionan en Windows y en celular
- [ ] Sin spam (mismo alerta no se repite)
- [ ] Tópico de ntfy no aparece en logs ni repo

**Tiempo estimado:** 2 días

### Fase 6 — Agente de chats
**Contenido:**
- Agente local: leer chats activos de agy y Codex (solo metadatos)
- Subir metadatos cifrados al servidor
- Vista "Chats activos" multi-PC
- Excluir archivados

**Criterios de aceptación:**
- [ ] Chat creado en PC-A aparece en PC-B
- [ ] Chats archivados no aparecen
- [ ] PC sin agente muestra "sin agente en esta PC"

**Tiempo estimado:** 2-3 días

### Fase 7 — Estadísticas
**Contenido:**
- Serie temporal propia de lecturas
- Burn rate y proyección
- Mapa de calor por hora/día
- Comparativa entre cuentas
- Informe semanal
- Exportación CSV/JSON

**Tiempo estimado:** 3 días

### Fase 8 — Ping mínimo
**Contenido:**
- Pantalla "Cuentas que necesitan ping"
- Ping manual con botón por cuenta
- Ping automático opcional (con guardas)
- Configuración adaptada a hipótesis confirmada

**Tiempo estimado:** 2 días

### Fase 9 — Proveedores adicionales
**Contenido:**
- Adaptadores: Claude, Gemini, OpenCode, Grok
- Pestaña de chat de consumo (chat normal vs agente)
- Un adaptador nuevo sin cambios en el núcleo

**Tiempo estimado:** 5-7 días

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

## ⏸️ DETENIDO — Esperando aprobación

La Fase 0 está completada en su parte de documentación. Antes de continuar a Fase 1 necesito:

1. Tu **aprobación** del stack (ADR-001), el modelo de cifrado (ADR-002) y los demás ADRs.
2. Los resultados del **experimento de cooldown** (protocolo en `docs/research/04-experimento-cooldown.md`).
3. La verificación manual de las rutas de agy y Codex en tu PC.
4. La configuración de los **GitHub Actions Secrets** (listados arriba).
5. El **setup inicial del VPS** (comandos listados arriba — necesita tu ejecución vía SSH).

**Una vez aprobado, continúo directamente con la Fase 1 (núcleo seguro en Rust).**

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

### Fase 3 — Adaptadores agy + Codex ← ACTUAL
**Contenido implementado:**
- Crate `providers/agy/`: Credential Manager Windows, lectura/escritura de credencial `gemini:antigravity`, parser de `agy /usage`, verificación con `agy models`, cambio de cuenta con revertir.
- Crate `providers/codex/`: lectura/escritura atómica de `~/.codex/auth.json` (.tmp → rename + .bak), decodificación de JWT para email, cambio de cuenta con revertir.
- Tests con `fake_agy` y `fake_codex` (sin credenciales reales, sin procesos externos).

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

## ⏸️ FASE 0 COMPLETA — Esperando aprobación para Fase 1

### Estado del VPS (verificado 2026-11-22)
| Elemento | Estado |
|----------|--------|
| `/opt/ai-keyvault/{bin,config,data,logs,backups}` | ✅ Creados |
| `.env` del servidor | ✅ Creado |
| Usuario `ai-keyvault-deploy` con SSH | ✅ Configurado |
| Caddy instalado | ✅ Instalado (Caddyfile pendiente de configurar con subdominio) |
| Servicio `systemd` `ai-keyvault` | ⏳ Pendiente (se crea en Fase 2 cuando exista el binario) |
| `bin/ai-keyvault-server` | ⏳ Pendiente (se compila en Fase 2) |

### Qué falta antes de Fase 1 (solo tu aprobación)

1. **¿Apruebas el stack Tauri 2?** (ADR-001)
2. **¿Apruebas el modelo de cifrado** Argon2id + XChaCha20-Poly1305? (ADR-002)
3. **¿Apruebas la sincronización** LWW + Ed25519 + lease? (ADR-003)
4. **¿Apruebas el modelo de ping/alertas** configurable por hipótesis? (ADR-004)
5. **Subdominio para el VPS:** ¿cuál subdominio en Spaceship apuntará al VPS? (ej. `keyvault.tudominio.com`). Necesito el dominio para configurar el Caddyfile.

Una vez aprobado, arranco directamente con la **Fase 1: Núcleo seguro** (Argon2id, Vault Key, clave de respaldo, XChaCha20, zeroize, autobloqueo) en Rust.

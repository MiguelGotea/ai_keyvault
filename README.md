# AI KeyVault

<p align="center">
  <img src="assets/branding/AI_KeyVault_Color.png" alt="AI KeyVault Logo" width="180"/>
</p>

<p align="center">
  <strong>Bóveda y tablero personal para cuentas de agentes de IA de programación</strong><br/>
  Administra todas tus cuentas de Antigravity (agy) y Codex desde un solo lugar,<br/>
  con sincronización multi-PC, alertas inteligentes y estadísticas propias.
</p>

---

## ⚖️ Alcance ético

**AI KeyVault es una herramienta personal.** Está diseñada para:

- Administrar **cuentas propias** del autor en **PCs propias**.
- No tiene fines comerciales ni de distribución de accesos.
- No evade bloqueos ni comparte cuentas con terceros.
- La rotación de cuentas, el ping mínimo y la automatización son decisiones configurables del usuario, con registro completo de cada acción.
- No extrae, roba ni expone credenciales de nadie más.

---

## 📦 Estado del proyecto — 2026-11-26

### Primera etapa: Backend + API (✅ Completada)

| Fase | Estado | Contenido |
|------|--------|-----------|
| 0 | ✅ | Investigación, ADRs, cooldown-logger, estructura |
| 1 | ✅ | Núcleo seguro: bóveda cifrada (Argon2id + XChaCha20), clave de respaldo |
| 2 | ✅ | Servidor VPS (axum + SQLite), CI/CD, deploy automático |
| 3 | ✅ | Adaptadores agy + Codex: lectura de cuota, auth.json, Credential Manager |
| 4 | ✅ | Poller de cuota con lease anti-duplicado, multi-dispositivo |
| 5 | ✅ | Alerter: Windows toast + ntfy.sh, umbrales WARN/CRIT, cooldown |
| 6 | ✅ | Chats agy + Codex: metadatos sincronizados vía VPS |
| 7 | ✅ | Estadísticas: burn rate, heatmap, comparativa, exportación CSV |
| 8 | ✅ | Ping mínimo: detecta cuentas sin ventana activa, manual + automático |
| A | ✅ | Codex chats: lee `~/.codex/state_5.sqlite` tabla `threads` |
| C | ✅ | Cooldown H1 confirmada con datos reales |
| D | ✅ | Alert log: historial de alertas en VPS (`/api/alerts`) |
| E | ✅ | Switch log: historial de cambios de cuenta (`/api/switch`) |
| F | 📄 | Codex token renewal: documentado como procedimiento manual |
| 9 | ⏸ | **Etapa 3** — proveedores adicionales (Claude, Gemini, Grok) |

### Segunda etapa: App de escritorio ✅ v0.2.5 (Fases 18–28 completadas)

> Consulta [PLAN2.md](PLAN2.md) para el detalle completo de decisiones (D-01 a D-13) y matriz de cumplimiento.

| Fase | Estado | Descripción |
|------|--------|-------------|
| 10–17 | ✅ | Scaffold Tauri 2, bóveda local, dashboard, autobloqueo, configuración, build CI |
| 18 | ✅ | Reconexión del flujo de datos (`IdentityMap`, contrato API, errores visibles, respaldo cifrado) |
| 19 | ✅ | Sesión de usuario, primer arranque estilo Bitwarden, cierre de sesión global, inicio con Windows |
| 20 | ✅ | Credenciales de cuenta (correo/contraseña) y portapapeles con limpieza a 30 s |
| 21 | ✅ | Alertas con sugerencia de cuenta alternativa y botón «Cambiar a esa cuenta» |
| 22 | ✅ | Silenciar alertas por X horas, fin de suscripción (7 y 1 día) y consulta fallando |
| 23 | ✅ | Paleta de comandos `Ctrl+K` |
| 24 | ✅ | Detalle de cuenta con 6 pestañas |
| 25 | ✅ | Tema claro/oscuro y pantalla «Mis dispositivos» (revocar acceso) |
| 26 | ✅ | Cierre del switch («Deshacer», detección de procesos sin cerrarlos, «sin agente en esta PC») |
| 27 | ✅ | Estadísticas ampliadas (tokens por modelo, eficiencia, cuota desperdiciada) y «Experimento de cooldown» |
| 28 | ✅ | Versión portable `.zip` y soporte de firma Authenticode en CI (ver `docs/firma-y-portable.md`) |

**Instalación (Windows):**

> En GitHub Releases tienes tanto el instalador (`AI KeyVault_<versión>_x64-setup.exe` / `.msi`) como la edición portable (`AI-KeyVault-portable-v<versión>-win-x64.zip`). Si el binario no tiene firma Authenticode configurada, Windows puede mostrar SmartScreen: **"Más información" → "Ejecutar de todas formas"**.

```
1. Descargar de GitHub Releases (instalador .exe o portable .zip) o compilar con: pnpm tauri build
2. La app abre en /unlock — inicia sesión o crea tu usuario + bóveda con clave maestra (mín. 12 caracteres)
3. El poller arranca automáticamente (salvo en modo «Sin agente en esta PC») y se ve en la bandeja del sistema
```

---

## ✨ Funcionalidades del backend (operativas)

| Funcionalidad | Estado | Endpoint |
|---------------|--------|----------|
| 🔒 Bóveda cifrada sync | ✅ | `POST/GET /api/vault/sync` |
| 📊 Cuota multi-cuenta | ✅ | `GET /api/quota/readings/:id` |
| 🔔 Alert log | ✅ | `GET/POST /api/alerts` |
| 💬 Chats agy + Codex | ✅ | `GET /api/chats/:id` |
| 🏓 Ping log | ✅ | `GET/POST /api/pings` |
| 🔄 Switch log | ✅ | `GET/POST /api/switch` |
| 📈 Estadísticas | ✅ | `GET /api/stats/*` |
| 🏥 Health | ✅ | `GET /health` |

---

## 🏗️ Arquitectura

```
Cliente Windows (poller local)     VPS (198.211.97.243:3010)     GitHub
──────────────────────────────     ─────────────────────────     ──────
ai-keyvault-poller (bg)       →    API Rust/axum + SQLite    ←   Actions CI/CD
  ├─ collector (agy + codex)       /opt/ai-keyvault/
  ├─ chat_collector                  bin/ai-keyvault-server
  ├─ pinger                          data/keyvault.db
  ├─ alerter (toast + ntfy)          .env (secrets)
  └─ scheduler (cada 10min)        systemd: ai-keyvault
```

**Principio de conocimiento cero:** el servidor solo almacena texto cifrado. La clave que descifra nunca sale de tus PCs.

---

## 🖥️ App de Escritorio — v0.2.4 🟡

La app de escritorio **Tauri 2** (Rust + React/TypeScript) está operativa. Consume el backend VPS ya existente. Toda la comunicación HTTP al VPS va por comandos Rust — nunca `fetch()` desde React.

**Pantallas disponibles:**
- 🔐 `/unlock` — desbloqueo con clave maestra (Argon2id), flujo de creación de bóveda y recuperación con clave de respaldo
- 📊 `/dashboard` — tarjetas de cuota por cuenta y pool, switch de cuenta, historial
- 🔔 `/alerts` — log de alertas del VPS
- 🏓 `/ping` — cuentas que necesitan ping + ejecución manual
- 💬 `/chats` — chats activos por cuenta y dispositivo
- 📈 `/stats` — comparativa de burn rate + heatmap semanal
- ⚙️ `/settings` — URL VPS, autobloqueo (opt-in), DPAPI (opt-in), sync manual de bóveda

**Arquitectura de seguridad:**
- Autobloqueo (opt-in): timer Rust → evento `autolock` → `lock_vault` + `zeroize`. El poller **sigue corriendo**.
- DPAPI: desactivado por defecto. Con advertencia explícita.
- Vault sync: la bóveda local se sube al VPS cifrada (conocimiento cero — el servidor no puede descifrarla).

**Pendiente:** ver `PLAN2.md` (Fase 18 y «Matriz de cumplimiento del prompt original»). Incluye la firma de código para eliminar el aviso de SmartScreen.

---

## 🚀 Poller — cómo correr en una PC nueva

```powershell
# 1. Variables de entorno (.env o PowerShell)
$env:AKYV_SERVER_URL    = "http://198.211.97.243:3010"
$env:AKYV_DEVICE_ID     = "MI-PC-NOMBRE"
$env:AKYV_POLL_INTERVAL = "600"          # 10 min
$env:RUST_LOG           = "info"

# 2. Alertas opcionales
$env:AKYV_NTFY_TOPIC    = "tu-topico-secreto"  # NO al repo
$env:AKYV_TOAST_ENABLED = "true"

# 3. Ping automático (deshabilitado por defecto)
$env:AKYV_PING_ENABLED  = "false"   # activar con "true"
$env:AKYV_PING_HOUR     = "8"       # hora local (0-23)
$env:AKYV_PING_MAX_PCT  = "5.0"     # % máximo para pingear

# 4. Correr
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run --package ai-keyvault-poller
```

---

## 🔒 Seguridad

- **Argon2id** como KDF (parámetros en `docs/adr/ADR-002-cifrado.md`).
- **XChaCha20-Poly1305** para cifrado autenticado.
- **Ed25519** para autenticación de dispositivos.
- Sin telemetría hacia terceros.
- `gitleaks` en CI — ningún secreto puede entrar al repo.
- Nunca aparecen en el repo: tokens, emails reales, `auth.json`, `.env` reales, tópico ntfy.

---

## 📂 Estructura del repositorio

```
ai_keyvault/
  server/                   # API del VPS (Rust/axum + SQLite)
  crates/
    core/                   # Bóveda, cifrado, sincronización (Rust)
    poller/                 # Poller de cuota multi-dispositivo
    alerter/                # Detector de umbrales y notificador
  providers/
    agy/                    # Adaptador Antigravity (Credential Manager + cuota)
    codex/                  # Adaptador Codex (auth.json + chats SQLite)
  apps/
    desktop/                # Etapa 2: App Tauri 2
  docs/
    adr/                    # Architecture Decision Records
    research/               # Investigación con evidencia verificada
    operaciones.md          # VPS, deploy, tablas BD, variables de entorno
    desktop-app-insumos.md  # Spec completo para segunda etapa
  assets/branding/          # Logos e íconos
  .github/workflows/        # CI (fmt, clippy, test, deploy)
```

---

## 📖 Documentación adicional

- [`docs/manual-de-usuario.md`](docs/manual-de-usuario.md) — **Manual completo de usuario final** (todas las pantallas y funciones)
- [`docs/firma-y-portable.md`](docs/firma-y-portable.md) — Edición portable y firma Authenticode en CI
- [`CLAUDE.md`](CLAUDE.md) — Reglas de trabajo para el agente de desarrollo
- [`PLAN.md`](PLAN.md) — Historia del backend (etapa 1, congelado)
- [`PLAN2.md`](PLAN2.md) — App de escritorio: decisiones, estado real, Fase 18 y matriz de pendientes
- [`docs/operaciones.md`](docs/operaciones.md) — VPS, BD, deploy, variables de entorno
- [`docs/desktop-app-insumos.md`](docs/desktop-app-insumos.md) — Spec app de escritorio
- [`docs/adr/`](docs/adr/) — Decisiones técnicas justificadas
- [`docs/research/`](docs/research/) — Investigación con evidencia verificada
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) — Licencias de dependencias

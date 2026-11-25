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

## 📦 Estado del proyecto — 2026-11-25

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
| 9 | ⏸ | **Segunda etapa** — proveedores adicionales (Claude, Gemini, Grok) |

### Segunda etapa: App de escritorio (⏳ Pendiente)

Ver sección **App de Escritorio** más abajo.

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

## 🖥️ App de Escritorio — Segunda Etapa

> Ver [`docs/desktop-app-insumos.md`](docs/desktop-app-insumos.md) para el spec completo.

La segunda etapa consiste en crear la **UI de escritorio (Tauri 2)** que consume el backend ya operativo. Todo el backend está listo — la app es una capa de presentación + integración local.

**Decisiones ya tomadas (no reabrir):**
- **Framework:** Tauri 2 (Rust + TypeScript/React) — ver `docs/adr/ADR-001-stack.md`
- **Idioma UI:** 100% español, textos en `apps/desktop/src/i18n/es.json`
- **Tema:** Claro y oscuro intercambiables
- **Colores:** `#064E3B` (primario) + `#D6A84F` (secundario)
- **Sin telemetría** hacia terceros

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
    desktop/                # [SEGUNDA ETAPA] App Tauri 2
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

- [`CLAUDE.md`](CLAUDE.md) — Reglas de trabajo para el agente de desarrollo
- [`PLAN.md`](PLAN.md) — Plan detallado por fases con estado actual
- [`docs/operaciones.md`](docs/operaciones.md) — VPS, BD, deploy, variables de entorno
- [`docs/desktop-app-insumos.md`](docs/desktop-app-insumos.md) — Spec app de escritorio
- [`docs/adr/`](docs/adr/) — Decisiones técnicas justificadas
- [`docs/research/`](docs/research/) — Investigación con evidencia verificada
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) — Licencias de dependencias

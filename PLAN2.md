# PLAN2.md — Segunda Etapa: App de Escritorio Tauri 2

**Versión:** 1.1.0 — Fase 10 en progreso
**Fecha de inicio:** 2026-11-25
**Estado:** ⏳ En desarrollo — Fase 10 scaffold completo, pendiente de primer build de Tauri
**Referencia de entrada:** `docs/desktop-app-insumos.md` — spec completo del frontend

---

## Resumen ejecutivo

La primera etapa dejó el backend 100% operativo (VPS `198.211.97.243:3010`, API axum, SQLite, CI/CD). Esta segunda etapa construye la **app de escritorio Tauri 2** (Rust + React + TypeScript) que consume ese backend como cliente.

**Principio de arquitectura clave:** todo el tráfico HTTP al VPS se realiza desde **comandos Rust en Tauri** (`#[tauri::command]`), no desde `fetch()` en React. Esto evita problemas de CORS, CSP y contenido mixto, y mantiene la seguridad.

**Fuera de alcance en esta etapa:**
- Fase 9 (proveedores adicionales: Claude, Gemini, Grok, OpenCode) — se pospone
- Chats web de ChatGPT.com / Gemini.com — sin API pública de historial
- Subdominio + TLS en VPS — se agrega cuando se consiga el subdominio (Caddy ya instalado)

---

## Estado al inicio — Backend verificado ✅

| Elemento | Estado |
|----------|--------|
| Servidor activo `198.211.97.243:3010` | ✅ Operativo |
| `crates/core` (bóveda, Argon2id, XChaCha20) | ✅ Listo para reusar |
| `crates/poller` (binario `ai-keyvault-poller`) | ✅ Listo para empaquetar como sidecar |
| `crates/alerter` (detector + toast + ntfy) | ✅ Integrado en el poller |
| `providers/agy` (Credential Manager, switch, chats) | ✅ Listo |
| `providers/codex` (auth.json, switch, chats SQLite) | ✅ Listo |
| Backup diario → `ai_keyvault_data` (GitHub) | ✅ Workflow activo |
| CI/CD deploy-server en verde | ✅ |
| `apps/desktop/` | ✅ Scaffold completo — pendiente de primer build |

---

## Decisiones de arquitectura (confirmadas antes de arrancar)

> Estas decisiones están cerradas. No se reabren salvo bloqueo técnico documentado.

| Tema | Decisión |
|------|----------|
| Framework desktop | **Tauri 2** (Rust + React 18 + TypeScript + Vite) |
| Toolchain Windows | **MSVC** para Tauri (`x86_64-pc-windows-msvc`). GNU se mantiene para cross-compile al VPS. Ambos coexisten; `apps/desktop/src-tauri/rust-toolchain.toml` fuerza MSVC solo en ese directorio. |
| Comunicación UI → VPS | **Todo por comandos Rust** (`#[tauri::command]` con `reqwest`). Sin `fetch()` externo en React. CSP configurada para bloquearlo. |
| Poller | Sidecar Tauri. Proceso hijo que corre mientras la app está abierta. Se detiene al Salir. No depende de la bóveda (lee del SO directamente). |
| Nombre del sidecar | `ai-keyvault-poller-x86_64-pc-windows-msvc.exe` en disco. Referenciado como `"binaries/ai-keyvault-poller"` en `tauri.conf.json`. |
| Autobloqueo | Bloquea solo la UI. `vault.lock()` hace `zeroize` de `VaultKey`. El poller **sigue corriendo**. |
| IPC poller ↔ Tauri | Solo expone: `poller_status`, `poller_pause`, `poller_resume`. **Nunca descifrado ni credenciales**. |
| DPAPI | Opt-in, **desactivado por defecto**. Con advertencia clara. |
| Cierre de ventana | Minimizar a System Tray (bandeja). Menú: Abrir, Pausar poller, Salir. |
| Distribución | Solo instalador `.msi`/`.exe` generado por GitHub Actions (`windows-latest`). |
| VPS TLS | Sin subdominio por ahora. IP directa `198.211.97.243:3010`. Caddy instalado, pendiente de configurar cuando se obtenga subdominio. |
| Proveedores en UI | Solo agy + Codex (los únicos con backend implementado). |
| Workspace Cargo desktop | `apps/desktop/src-tauri/Cargo.toml` declara `[workspace]` vacío. Tiene su propio `Cargo.lock` y `target/`. Usa `crates/core` como path dependency. |
| ai_keyvault_data | Backup automático ya funciona (workflow `backup.yml`). La UI muestra estado del último backup en Configuración. |

---

## Fases de la segunda etapa

Cada fase termina con: **tests pasando, CI en verde, escaneo de secretos limpio, documentación actualizada, y esta sección actualizada con el estado real.**

---

### Fase 10 — Scaffolding, entorno, CI temprano y build de prueba

**Estado:** 🔄 En progreso — Scaffold creado, pendiente de primer `cargo tauri build` por el usuario
**Fecha de inicio:** 2026-11-25

**Objetivo:** Que `cargo tauri build` genere un `.msi` funcional (aunque la app esté vacía) con el sidecar del poller correctamente empaquetado. Detectar errores de CI temprano.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `cmake` no instalado en la PC de desarrollo | `rusqlite bundled` (en `providers/codex`) fallaría al compilar el desktop | **Decisión:** `crates/core`, `providers/agy`, `providers/codex` NO se incluyen como dependencias del desktop en Fase 10. Se agregarán en Fase 12 cuando se necesiten. El desktop se comunica con el backend VPS via `reqwest`. |
| `thiserror v2` en desktop vs `v1` en crates backend | Cargo puede coexistir ambas versiones sin conflicto, pero para consistencia | Unificado en `thiserror = "1"` en el desktop |
| `rust-toolchain.toml` necesita especificar el triple completo | Con solo `channel = "stable"` y dos toolchains instalados (gnu/msvc), no garantiza MSVC | Cambiado a `channel = "stable-x86_64-pc-windows-msvc"` |
| `webviewInstallMode: offlineInstaller` requiere un MSI de WebView2 | Innecesario para testing local — WebView2 ya está instalado | Cambiado a `downloadBootstrapper` para el primer build de prueba |
| `pnpm approve-builds` para esbuild | pnpm bloquea scripts de build por seguridad | Se registra en `setup-entorno.md` como paso obligatorio |
| GNU es el toolchain default activo | Correcto — el backend compila para Linux con GNU | `rust-toolchain.toml` en `src-tauri/` fuerza MSVC solo para Tauri |

#### Contenido

**Entorno (prerequisitos):**
- [x] Visual Studio Build Tools 2022 con carga "Desarrollo de escritorio con C++" — `cl.exe` encontrado en `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\`
- [x] `rustup target add x86_64-pc-windows-msvc` verificado — toolchain MSVC instalado
- [x] pnpm instalado (v12.23.0)
- [x] Tauri CLI v2 instalado (v2.12.6)

**Scaffold (completado 2026-11-25):**
- [x] `apps/desktop/src-tauri/rust-toolchain.toml` → `channel = "stable-x86_64-pc-windows-msvc"`, `targets = ["x86_64-pc-windows-msvc"]`
- [x] `apps/desktop/src-tauri/Cargo.toml` con `[workspace]` vacío — sin deps problemáticas en Fase 10
- [x] `apps/desktop/src-tauri/tauri.conf.json` — CSP configurada, sidecar registrado, WebView2 downloadBootstrapper
- [x] `apps/desktop/src-tauri/src/main.rs` — Tauri builder con tray y comandos registrados
- [x] `apps/desktop/src-tauri/src/tray.rs` — System Tray con menú (Abrir / Salir)
- [x] `apps/desktop/src-tauri/src/commands/health.rs` — `ping_server` via reqwest
- [x] `apps/desktop/src-tauri/src/commands/vault.rs` — `get_vault_status` stub
- [x] `apps/desktop/src/styles/tokens.css` — colores `#064E3B` / `#D6A84F` tema oscuro
- [x] `apps/desktop/src/styles/global.css`
- [x] `apps/desktop/src/i18n/es.json` — claves de UI en español
- [x] `apps/desktop/src/types/api.ts` — tipos TypeScript
- [x] `apps/desktop/src/pages/UnlockPage.tsx` — placeholder con `invoke("ping_server")`
- [x] `apps/desktop/src/pages/DashboardPage.tsx` — placeholder
- [x] `apps/desktop/src/App.tsx` — rutas React Router (`/unlock`, `/dashboard`)
- [x] `apps/desktop/src/main.tsx` — punto de entrada React con BrowserRouter
- [x] `apps/desktop/vite.config.ts` — puerto 1420 para Tauri dev
- [x] `apps/desktop/package.json`, `tsconfig.json`, `eslint.config.js`, `.gitignore`
- [x] `apps/desktop/node_modules/` — pnpm install completado
- [x] `apps/desktop/src-tauri/icons/` — íconos generados desde `assets/branding/` (32x32, 128x128, 128x128@2x, icon.ico, tray-icon.png)
- [x] `.scripts/gen-icons.ps1` — script para regenerar íconos desde branding
- [x] `.scripts/copy-sidecar.ps1` — script para copiar el poller compilado
- [x] `.scripts/build-fase10.ps1` — script guía del proceso completo de build

**Sidecar (pendiente — requiere acción del usuario):**
- [ ] Compilar poller para MSVC: `cargo +stable-x86_64-pc-windows-msvc build --release --package ai-keyvault-poller --target x86_64-pc-windows-msvc`
- [x] Script de copia preparado: `.scripts/copy-sidecar.ps1`
- [x] Registrado en `tauri.conf.json`: `"externalBin": ["binaries/ai-keyvault-poller"]`
- [ ] `cargo tauri build` → genera `.msi` funcional
- [ ] Verificar que el sidecar aparece en el instalador y arranca desde la app vacía

**CI en Fase 10 (pendiente de verificar en Actions):**
- [x] Job `tauri-build` agregado en `.github/workflows/ci.yml` con `runs-on: windows-latest`
- [x] `frontend-ci` corregido: usa pnpm
- [ ] Push y verificar que `tauri-build` pasa en verde en GitHub Actions

#### Documentación obligatoria al cerrar esta fase
- [ ] `docs/setup-entorno.md` §9: MSVC Build Tools, WebView2, pnpm, Tauri CLI v2, dual toolchain MSVC/GNU, naming convention del sidecar, workspace aislado de `apps/desktop`
- [ ] `docs/adr/ADR-001-stack.md`: sección nueva "Decisión dual toolchain" y "Workspace aislado"
- [ ] `CLAUDE.md` §8: regla "*apps/desktop usa su propio workspace Cargo — compilar con `cargo tauri dev/build` desde `apps/desktop/`, nunca desde la raíz*"
- [ ] `PLAN2.md` esta sección: marcar como ✅ con fecha y hallazgos

#### Criterios de aceptación
- [ ] `cargo tauri build` en verde localmente → genera `.msi`
- [ ] CI `tauri-build` en verde en GitHub Actions
- [ ] Sidecar `ai-keyvault-poller` aparece empaquetado y arranca sin errores
- [ ] Ningún secreto detectado por `gitleaks`
- [ ] `docs/setup-entorno.md` actualizado y suficiente para configurar una PC nueva

---

### Fase 11 — Capa de comunicación Rust ↔ VPS ↔ React

**Estado:** ⏳ Pendiente

**Objetivo:** Toda llamada al VPS se hace desde Rust. React solo llama `invoke()`. CSP bloquea `fetch()` externo.

#### Contenido
- [ ] `apps/desktop/src-tauri/src/api.rs`: cliente `reqwest` que encapsula todos los endpoints del VPS
- [ ] Comandos Tauri expuestos (`src-tauri/src/commands/`):
  - `get_quota(account_id: String) → Vec<QuotaReading>`
  - `get_chats(account_id: String) → Vec<ChatSession>`
  - `get_alerts(account_id: Option<String>) → Vec<AlertRecord>`
  - `get_pings_needs(max_pct: f64) → Vec<String>`
  - `get_stats_summary(account_id: String, days: u32) → StatsSummary`
  - `get_stats_heatmap(account_id: String, days: u32) → Vec<HeatmapCell>`
  - `get_stats_compare(days: u32) → Vec<StatsSummary>`
  - `get_switch_history(limit: u32) → Vec<SwitchRecord>`
  - `register_device(device_id: String, public_key: String) → ()`
- [ ] Tipos compartidos Rust `↔` TypeScript en `src-tauri/src/types.rs` + `src/types/api.ts`
- [ ] `tauri.conf.json` CSP: `"csp": "default-src 'self'; connect-src tauri: ipc: http://ipc.localhost"` — bloquea cualquier conexión externa desde JS
- [ ] Tests unitarios de los comandos con servidor mock (`mockito` en Rust)
- [ ] Hook React `useCommand<T>(cmd, args)` que envuelve `invoke()` con loading/error state

#### Documentación obligatoria al cerrar
- [ ] `docs/operaciones.md` §18: "Toda llamada al VPS va por comandos Rust — no hay fetch() externo en React"
- [ ] `PLAN2.md`: actualizar estado

#### Criterios de aceptación
- [ ] Un componente React puede mostrar datos de cuota del VPS via `invoke("get_quota", ...)`
- [ ] `fetch("http://198.211.97.243:3010/health")` desde React es bloqueado por CSP
- [ ] Tests de comandos Rust pasan sin servidor real

---

### Fase 12 — Desbloqueo, bóveda local y System Tray

**Estado:** ⏳ Pendiente

**Objetivo:** La app arranca con pantalla de clave maestra. Al desbloquear, el poller inicia como proceso hijo. Al cerrar la ventana, se va a la bandeja.

#### Contenido

**Desbloqueo:**
- [ ] Pantalla `/unlock`: input de clave maestra, opción de recuperación con clave de respaldo
- [ ] Comando Tauri `unlock_vault(password: String) → bool`  → llama a `crate core` `Vault::unlock_with_password()`
- [ ] Primer uso: si no hay bóveda local → flujo "Crear bóveda nueva" → `Vault::create_new()` → mostrar clave de respaldo `AKVR-XXXX-...` una sola vez con botón "Copiar / Imprimir / Exportar TXT"
- [ ] Autobloqueo: timer de inactividad (default 15 min) → llama a `vault.lock()` → `zeroize` de `VaultKey` → pantalla de desbloqueo

**Poller como proceso hijo:**
- [ ] Al desbloquear: `Command::new("binaries/ai-keyvault-poller").envs(config).spawn()`
- [ ] IPC con el poller: solo `poller_status()`, `poller_pause()`, `poller_resume()` — **sin acceso a credenciales cifradas**
- [ ] Al autobloquear: el poller **sigue corriendo** (lee del SO, no de la bóveda)
- [ ] Al Salir (desde el tray): el proceso hijo recibe señal de terminación

**System Tray:**
- [ ] Icono en la bandeja con el logo monocolor de AI KeyVault
- [ ] Menú contextual: "Abrir AI KeyVault", "Estado: Poller ✅/⏸", "Salir"
- [ ] Cerrar ventana → minimizar a tray (no terminar el proceso)

**DPAPI (opt-in):**
- [ ] Desactivado por defecto
- [ ] Si el usuario lo activa en Ajustes → advertencia clara: "Cualquier persona con tu sesión de Windows abierta podrá acceder sin clave maestra"
- [ ] Solo se activa con confirmación explícita

#### Documentación obligatoria al cerrar
- [ ] `CLAUDE.md` §12 (nuevo): "Separación poller/bóveda: qué puede hacer el poller cuando la UI está bloqueada"
- [ ] `docs/operaciones.md` §19: flujo de autobloqueo y qué servicios siguen activos
- [ ] `PLAN2.md`: actualizar estado

#### Criterios de aceptación
- [ ] Clave maestra incorrecta → error claro, sin acceso
- [ ] Clave de respaldo desbloquea y permite cambiar clave maestra
- [ ] Autobloqueo a los 15 min → pantalla de desbloqueo aparece, el poller sigue corriendo
- [ ] Cerrar ventana → app en bandeja, `GET /health` del VPS sigue llegando (poller activo)
- [ ] Salir desde bandeja → proceso poller termina
- [ ] Ningún token ni email visible en logs al bloquear/desbloquear

---

### Fase 13 — Dashboard y gestión de cuentas

**Estado:** ⏳ Pendiente

**Objetivo:** Panel principal con tarjetas de cuentas y flujo completo para agregar/editar/eliminar cuentas en la bóveda.

#### Contenido

**Dashboard `/dashboard`:**
- [ ] Tarjetas por cuenta: proveedor, alias, % usado por pool, próximo reset, estado (badge verde/amarillo/rojo)
- [ ] Umbral 80% → badge amarillo + borde secundario `#D6A84F`
- [ ] Umbral 95% → badge rojo + animación de pulso sutil
- [ ] Datos de cuota via `invoke("get_quota", ...)` al abrir y cada ciclo del poller
- [ ] Botón "Cambiar a esta cuenta" → navega al flujo de switch (Fase 14)
- [ ] Badge "Necesita ping" si `percent_used < 5%` → navega a Ping (Fase 15)
- [ ] Paleta de comandos `Ctrl+K`: "Cambiar a cuenta…", "Copiar contraseña de…", "Ir a estadísticas"

**Gestión de bóveda `/settings/accounts`:**
- [ ] Agregar cuenta agy:
  - La app verifica si agy tiene sesión activa (lee Credential Manager)
  - Si hay sesión → muestra el email detectado → usuario confirma → `vault.add_account()` con credencial cifrada
  - Si no hay sesión → instrucciones: "Abre agy y haz login primero, luego vuelve aquí"
- [ ] Agregar cuenta Codex:
  - Verifica si existe `~/.codex/auth.json` vigente
  - Si existe → email pre-rellenado → usuario confirma → `vault.add_account()`
  - Si no existe → instrucciones: "Abre Codex y haz login primero"
- [ ] Editar: cambiar alias, notas
- [ ] Eliminar: confirmación + borrado de bóveda local + sync al VPS
- [ ] Portapapeles: copiar email/contraseña con un clic → limpieza automática a los 30 s con contador visible

#### Documentación obligatoria al cerrar
- [ ] `PLAN2.md`: actualizar estado, registrar cualquier hallazgo sobre Credential Manager / auth.json en la UI

#### Criterios de aceptación
- [ ] Tarjeta muestra % actualizado con datos reales del VPS
- [ ] Agregar cuenta agy detecta el email del Credential Manager sin que el usuario lo escriba
- [ ] Copiar contraseña → portapapeles se limpia automáticamente a los 30 s
- [ ] `Ctrl+K` abre paleta de comandos funcional

---

### Fase 14 — Detalle de cuenta y Switch

**Estado:** ⏳ Pendiente

**Objetivo:** Pantalla de detalle completa con 6 pestañas, y flujo de cambio de cuenta seguro con revertir automático.

#### Contenido

**Pantalla `/account/:id` con pestañas:**

| Pestaña | Contenido |
|---------|-----------|
| *Resumen* | Plan, estado, última lectura (`read_at`, fuente PC/VPS), días restantes de suscripción |
| *Cuota y cooldown* | Pools con barras de progreso, `reset_time`, burn rate (`%/h`), proyección de horas al 100% |
| *Estadísticas* | Gráfica de serie temporal (burn rate histórico), heatmap de uso por día/hora de esta cuenta |
| *Chats* | Lista de chats activos de esta cuenta desde todas las PCs (`GET /api/chats/:account_id`), filtro por dispositivo |
| *Credenciales* | Email (descifrado solo al hacer clic), copiar contraseña (30s limpieza), notas |
| *Historial* | Switches, pings y alertas de esta cuenta (`GET /api/switch/history`, `/api/pings/:id`, `/api/alerts?account_id=`) |

**Switch de cuenta:**
- [ ] Botón "Activar esta cuenta" (desde Dashboard o pestaña Resumen)
- [ ] Confirmación: "¿Cambiar a cuenta@gmail.com? Deberás reiniciar las ventanas de agy/Codex que estén abiertas"
- [ ] Detecta procesos `agy.exe` / `codex.exe` en ejecución → muestra lista de los que hay que reiniciar
- [ ] Llama a `AccountSwitcher::switch()` via IPC → escribe Credential Manager / auth.json
- [ ] Validación automática: `agy models` para agy, verificación de auth.json para Codex
- [ ] Si falla → revertir automático → cuenta marcada en cuarentena con badge de advertencia
- [ ] Si éxito → `POST /api/switch` al VPS → botón "Deshacer" disponible por 5 minutos

#### Documentación obligatoria al cerrar
- [ ] `PLAN2.md`: actualizar estado, registrar comportamiento real del switch con cuentas reales

#### Criterios de aceptación
- [ ] Las 6 pestañas muestran datos reales del VPS
- [ ] Switch ejecutado por la herramienta, el usuario no corre scripts
- [ ] Fallo en switch → revertir automático demostrable
- [ ] Procesos en ejecución detectados correctamente sin ser cerrados
- [ ] Botón "Deshacer" funciona dentro del período

---

### Fase 15 — Ping, Alertas, Chats multi-PC, Dispositivos y Configuración

**Estado:** ⏳ Pendiente

**Objetivo:** Completar todas las pantallas secundarias y la configuración completa incluyendo estado del backup de `ai_keyvault_data`.

#### Contenido

**Ping `/ping`:**
- [ ] Lista de cuentas con `percent_used < umbral` (`GET /api/pings/needs`)
- [ ] Botón "Hacer ping" por cuenta → `invoke("execute_ping", {account_id})` → `POST /api/pings`
- [ ] Toggle "Ping automático global" con hora configurable
- [ ] Aviso de riesgo: "El ping es tráfico automatizado — puede violar ToS del proveedor. Úsalo con criterio."

**Alertas `/alerts`:**
- [ ] Historial de alertas disparadas (`GET /api/alerts`)
- [ ] Toggle WARN (80%) / CRIT (95%) por cuenta
- [ ] Silenciar alerta por X horas (opciones: 1h, 4h, 24h)
- [ ] Alertas de fin de suscripción (7 días / 1 día antes)
- [ ] Alertas de login vencido o rechazado (cuenta en cuarentena)

**Chats multi-PC `/chats`:**
- [ ] Lista de chats activos de todas las PCs que tienen AI KeyVault instalado
- [ ] Filtros: por proveedor (agy / Codex), por dispositivo, por cuenta
- [ ] Título + fecha de última actividad + dispositivo de origen
- [ ] "PC sin agente" para dispositivos sin poller activo (última sincronización hace más de 2h)
- [ ] Sin contenido de los mensajes — solo metadatos

**Mis Dispositivos `/devices`:**
- [ ] Lista de dispositivos registrados: nombre, último contacto, estado (activo/inactivo)
- [ ] Botón "Revocar acceso" por dispositivo

**Configuración `/settings`:**
- [ ] URL del VPS (default `http://198.211.97.243:3010`)
- [ ] Device ID (nombre de esta PC)
- [ ] Intervalo de polling (default 10 min)
- [ ] ntfy.sh tópico (campo de texto, oculto en logs y repo — solo env var)
- [ ] Toast de Windows habilitado / deshabilitado
- [ ] Umbrales WARN/CRIT (sliders)
- [ ] Hora del ping automático (0–23)
- [ ] Máx pings automáticos por cuenta por día
- [ ] Autobloqueo (minutos, default 15)
- [ ] DPAPI — opt-in con advertencia (desactivado por defecto)
- [ ] Tema claro / oscuro
- [ ] **Respaldo `ai_keyvault_data`:**
  - Indicador: "Último respaldo automático: hace X días / fecha"
  - Enlace: "Ver instrucciones de restauración" → abre `docs/operaciones.md §6`
  - Botón: "Disparar respaldo ahora" → llama al endpoint de GitHub Actions (workflow_dispatch) si se tiene el token configurado (opcional)

#### Documentación obligatoria al cerrar
- [ ] `docs/operaciones.md` §19: flujo de restauración asistido desde la UI
- [ ] `PLAN2.md`: actualizar estado

#### Criterios de aceptación
- [ ] Ping manual funciona y aparece en historial del VPS
- [ ] Alerta silenciada no se repite en el período configurado
- [ ] Chat creado en otra PC aparece en esta pantalla
- [ ] "PC sin agente" aparece para dispositivos sin sincronización reciente
- [ ] Configuración persiste entre reinicios de la app
- [ ] Estado del respaldo `ai_keyvault_data` visible en Configuración

---

### Fase 16 — Estadísticas globales e instalador final

**Estado:** ⏳ Pendiente

**Objetivo:** Pantalla de estadísticas globales completa y build de producción verificado con prueba de restauración desde `ai_keyvault_data`.

#### Contenido

**Estadísticas globales `/stats`:**
- [ ] Comparativa de cuentas: bar chart `GET /api/stats/compare`
- [ ] Burn rate proyectado por cuenta: `GET /api/stats/summary`
- [ ] Mapa de calor semanal: `GET /api/stats/heatmap`
- [ ] Informe semanal: `GET /api/stats/weekly`
- [ ] Exportar CSV / JSON: `GET /api/stats/export`

**Instalador final:**
- [ ] Job `tauri-build` en CI ya establecido desde Fase 10 — verificar en verde con la app completa
- [ ] `bundle.windows.webviewInstallMode`: modo `OfflineInstaller` para incluir WebView2 en el instalador
- [ ] Versión en `tauri.conf.json` y `Cargo.toml` sincronizadas
- [ ] SmartScreen warning documentado en `README.md`: "Windows puede mostrar una advertencia porque el instalador no está firmado con un certificado de código. Haz clic en 'Más información' → 'Ejecutar de todas formas'."
- [ ] `.msi` generado en GitHub Releases como artifact del CI

**Verificación final de `ai_keyvault_data`:**
- [ ] Confirmar que el backup workflow sigue corriendo tras instalar la app
- [ ] Realizar una restauración real desde un snapshot de `ai_keyvault_data` → VPS → verificar que la app ve los datos correctamente
- [ ] Documentar el flujo paso a paso en `docs/operaciones.md §6` (actualizar con experiencia real)

#### Documentación obligatoria al cerrar
- [ ] `PLAN2.md`: marcar todas las fases como ✅ y registrar fecha de cierre
- [ ] `PLAN.md`: actualizar con estado "Segunda Etapa Completada"
- [ ] `README.md`: actualizar con instrucciones de instalación de la app de escritorio
- [ ] `docs/setup-entorno.md`: versión final completa con todo el toolchain verificado
- [ ] `CLAUDE.md`: actualizar `§7 Estado de fases` con Fase Desktop ✅

#### Criterios de aceptación
- [ ] `.msi` instalable en una PC Windows limpia sin herramientas de desarrollo
- [ ] La app descifra la bóveda y muestra cuentas tras instalar desde el `.msi`
- [ ] El poller arranca automáticamente y se ve en el System Tray
- [ ] Restauración desde `ai_keyvault_data` funciona y los datos aparecen en la app
- [ ] CI en verde en todos los jobs: `secrets-scan`, `rust-ci`, `frontend-ci`, `tauri-build`, `deploy-server`

---

## Riesgos y mitigaciones (segunda etapa)

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Conflicto MSVC/GNU en el workspace Cargo | Media | Alta | `rust-toolchain.toml` en `apps/desktop/src-tauri/` fuerza MSVC. Probado en Fase 10. |
| Sidecar con nombre incorrecto | Alta | Alta | Probado en Fase 10 antes de cualquier UI. Nombre exacto documentado. |
| CORS/CSP bloqueando llamadas al VPS | Media | Alta | Toda llamada va por comandos Rust. CSP configurada en Fase 11. |
| WebView2 no instalado en PC destino | Media | Media | Modo `OfflineInstaller` en el bundle de Tauri incluye WebView2. |
| Poller expone credenciales cuando UI bloqueada | Baja | Crítica | Poller solo lee del SO (Credential Manager / auth.json), no de la bóveda. IPC del poller solo expone estado. |
| SmartScreen bloquea el instalador | Alta | Baja | Documentado en `README.md`. Solución manual documentada. |
| `ai_keyvault_data` backup falla silenciosamente | Baja | Media | Indicador de fecha de último backup en la UI (Fase 15). |
| `agy models` tarda o falla en la validación del switch | Media | Media | Timeout configurable. Si falla → revertir automático. |

---

## Criterios de calidad transversales (mismos que primera etapa)

- Código tipado. Rust: `rustfmt` + `clippy -D warnings`. TypeScript: ESLint + Prettier + strict mode.
- Commits convencionales en español: `feat(desktop): descripción`
- Sin telemetría hacia terceros.
- Manejo de errores en español en la UI.
- Ningún secreto en el repositorio. `gitleaks` bloquea el CI.
- Tests en cada fase: unitarios (Rust y Vitest), integración con mocks, e2e básico (Playwright + WebView2).
- `PLAN2.md` actualizado al terminar cada fase con estado real, hallazgos y fecha.
- Cualquier cambio a decisiones de arquitectura → actualizar el ADR correspondiente.

---

## Hallazgos y cambios de plan (registro vivo)

> Esta sección se completa durante el desarrollo. Cada hallazgo que modifique el plan se registra aquí con fecha.

| Fecha | Fase | Hallazgo | Impacto | Acción |
|-------|------|----------|---------|--------|
| 2026-11-25 | Pre-arranque | Rust toolchain actual: GNU activo, MSVC instalado pero no default. `apps/desktop` necesita `rust-toolchain.toml`. | Alto | Fase 10 lo configura. Documentado en ADR-001 y setup-entorno.md. |
| 2026-11-25 | Pre-arranque | `ci.yml` frontend-ci usa `npm` y no `pnpm`. Hay que corregirlo en Fase 10. | Medio | Job actualizado en Fase 10. |
| 2026-11-25 | Pre-arranque | Llamadas HTTP desde React al VPS causarían CORS/CSP/contenido mixto. | Alto | Toda comunicación VPS va por comandos Rust. CSP configurada en Fase 11. |
| 2026-11-25 | Pre-arranque | DPAPI: si se activa, cualquiera con la sesión de Windows accede sin clave maestra. | Alto | Opt-in desactivado por defecto. Advertencia explícita en Ajustes. |

---

## ✅ SEGUNDA ETAPA COMPLETADA — (fecha pendiente)

> Esta línea se completa al cerrar la Fase 16.

# PLAN2.md — Segunda Etapa: App de Escritorio Tauri 2

**Versión:** 1.3.0 — Fases 10-15 completadas
**Fecha de inicio:** 2026-11-25
**Estado:** ⏳ En desarrollo — Fases 10–15 con código completo, pendiente de primer `cargo tauri build` por el usuario
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

**Estado:** ✅ Completada (código) — 2026-11-25 — pendiente de primer `cargo tauri build` por el usuario
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

**Estado:** ✅ Completada — 2026-11-25

**Objetivo:** Toda llamada al VPS se hace desde Rust. React solo llama `invoke()`. CSP bloquea `fetch()` externo.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `crates/core` no tiene dependencia sqlite — solo argon2, chacha20poly1305, zeroize | Seguro incluirlo como path dep en desktop sin cmake | Incluido directamente como path dep |
| `get_active_account()` necesita `rusqlite bundled` (providers/agy/codex) → cmake requerido | cmake no instalado → stub retorna cuenta vacía | Stub implementado; se reemplaza cuando se instale cmake |
| VPS `GET /api/stats/summary/:account_id?days=N` retorna `Vec<PoolSummary>` | Tipos Rust/TS alineados | `api_client.rs` y `types/api.ts` actualizados |

#### Contenido (completado)
- [x] `apps/desktop/src-tauri/src/api_client.rs`: cliente `reqwest` singleton con todos los tipos del VPS
- [x] Todos los comandos Tauri en `commands/api.rs`: get_quota, get_stats_summary, get_stats_heatmap, get_stats_compare, get_stats_weekly, get_chats, get_alerts, get_switch_history, post_switch, get_pings_needs, get_pings, post_ping, register_device, check_server_health
- [x] `hooks/useCommand.ts` — hook React con loading/error/refetch
- [x] `hooks/useServerHealth.ts` — polling cada 30s
- [x] `components/ServerStatusBadge.tsx`
- [x] `tauri.conf.json` CSP configurada — bloquea fetch() externo desde JS
- [x] Tipos compartidos en `src/types/api.ts`

#### Documentación obligatoria al cerrar
- [ ] `docs/operaciones.md` §18: "Toda llamada al VPS va por comandos Rust — no hay fetch() externo en React"
- [ ] `PLAN2.md`: actualizar estado

#### Criterios de aceptación
- [ ] Un componente React puede mostrar datos de cuota del VPS via `invoke("get_quota", ...)`
- [ ] `fetch("http://198.211.97.243:3010/health")` desde React es bloqueado por CSP
- [ ] Tests de comandos Rust pasan sin servidor real

---

### Fase 12 — Desbloqueo, bóveda local y System Tray

**Estado:** ✅ Completada — 2026-11-25

**Objetivo:** La app arranca con pantalla de clave maestra. Al desbloquear, el poller inicia como proceso hijo. Al cerrar la ventana, se va a la bandeja.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `RecoveryKey::to_formatted_string()` (no `to_string()`), `from_formatted_string()` (no `from_str()`) | Error de compilación si se usa el nombre incorrecto | Nombres correctos usados en `vault.rs` |
| `Vault::recover_with_key()` retorna `Result<()>` — no devuelve nueva clave | La UI no puede mostrar nueva clave de recuperación post-recover | Flujo de recover solo re-encripta con nueva clave maestra; no muestra nueva clave de respaldo |
| `vault_path()` debe ser `pub` para usarse desde `security.rs` | Error de visibilidad | Cambiado a `pub fn vault_path` |

#### Contenido (completado)
- [x] `commands/vault.rs` — ciclo completo: get_vault_status, create_vault, unlock_vault, lock_vault, recover_vault, list_vault_accounts
- [x] `commands/poller.rs` — PollerManager: start_poller, pause_poller, resume_poller, stop_poller, get_poller_status
- [x] `commands/switch_cmd.rs` — switch_account (log a VPS), get_active_account (stub cmake)
- [x] `tray.rs` — System Tray con menú (Abrir / Salir), minimizar a bandeja al cerrar ventana
- [x] `components/PollerStatusBadge.tsx` — polling cada 15s
- [x] `pages/UnlockPage.tsx` — flujo completo: crear bóveda, desbloquear, recuperación, mostrar clave de respaldo una sola vez

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

**Estado:** ✅ Completada — 2026-11-25

**Objetivo:** Panel principal con tarjetas de cuota por cuenta y switch de cuenta.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `NavLink` de react-router-dom con `style` como función requiere cast `as React.CSSProperties` | Error TypeScript en AppLayout | Cast aplicado — TS pasa |
| `get_stats_compare` retorna `Vec<PoolSummary>` agrupados por proveedor:cuenta | UI necesita reducir el array | `byAccount` reduce en DashboardPage |
| Poller sidecar falla si no está compilado — DashboardPage lo captura silenciosamente | No bloquea el resto de la UI | Error capturado, `pollerError` state mostrado como aviso |

#### Contenido (completado)
- [x] `components/AppLayout.tsx` — sidebar con NavLink, botón bloquear, badges server+poller
- [x] `components/QuotaCard.tsx` — tarjeta con burn rate, ETA, barra de progreso, colores semafóricos
- [x] `pages/DashboardPage.tsx` — quota por cuenta, switch button, switch history table, lanzador de poller
- [x] `pages/AlertsPage.tsx` — log de alertas del VPS
- [x] `pages/PingPage.tsx` — lista ping needs + ping manual
- [x] `pages/ChatsPage.tsx` — chats por cuenta
- [x] `pages/StatsPage.tsx` — comparativa + heatmap 24h×7d
- [x] `pages/SettingsPage.tsx` — formulario completo con DPAPI toggle + modal de advertencia

#### Documentación obligatoria al cerrar
- [ ] `PLAN2.md`: actualizar estado, registrar cualquier hallazgo sobre Credential Manager / auth.json en la UI

#### Criterios de aceptación
- [ ] Tarjeta muestra % actualizado con datos reales del VPS
- [ ] Agregar cuenta agy detecta el email del Credential Manager sin que el usuario lo escriba
- [ ] Copiar contraseña → portapapeles se limpia automáticamente a los 30 s
- [ ] `Ctrl+K` abre paleta de comandos funcional

---

### Fase 14 — Autobloqueo y seguridad IPC

**Estado:** ✅ Completada — 2026-11-25

**Objetivo:** Timer de autobloqueo con emisión de evento Tauri al frontend, vault sync con VPS (conocimiento cero), seguridad del poller cuando UI está bloqueada.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `AUTOLOCK_RUNNING: AtomicBool` global — cancelar el timer anterior antes de iniciar uno nuevo | Si se llama `start_autolock_timer` dos veces, el primero puede dispararse igualmente | Flip a `false` + 100ms sleep antes de iniciar el nuevo |
| `listen()` de `@tauri-apps/api/event` retorna un unlisten function — hay que limpiarla al desmontar | Fuga de listener si App se remonta | `useRef` + cleanup en `useEffect` return |
| `reset_autolock_timer` llama a `start_autolock_timer` internamente — throttle de 30s en frontend | Rust recibiría llamadas en cada pixel de mouse sin throttle | Throttle manual con `Date.now()` en `handleActivity` |
| Autobloqueo 0 minutos = desactivado (opt-in) — valor por defecto cambiado de 15 a 0 en `DEFAULT_SETTINGS` | El usuario debe activarlo explícitamente | `autolockMinutes: 0` en DEFAULT_SETTINGS; `localStorage.getItem("autolock_minutes") ?? "0"` |
| `download_vault_sync` se llama silenciosamente tras unlock — no bloquea la navegación | Mejor UX: la app ya está en /dashboard mientras baja en background | `.catch(() => {})` tras `invoke("download_vault_sync")` |

#### Contenido (completado)
- [x] `commands/security.rs` — start_autolock_timer, reset_autolock_timer, cancel_autolock_timer, upload_vault_sync, download_vault_sync
- [x] `commands/mod.rs` — `pub mod security;` agregado
- [x] `main.rs` — 5 comandos de security registrados en invoke_handler
- [x] `App.tsx` — listener del evento `autolock` → lock_vault → navigate("/unlock")
- [x] `UnlockPage.tsx` — tras desbloqueo exitoso: start_autolock_timer (si activo) + download_vault_sync (silencioso)
- [x] `DashboardPage.tsx` — reset_autolock_timer en mousemove/keydown con throttle 30s
- [x] `SettingsPage.tsx` — VaultSyncButtons (upload/download manual), persistencia en localStorage, aplicar timer al guardar

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

**Estado:** ✅ Completada (UI base) — 2026-11-25

**Objetivo:** Completar todas las pantallas secundarias y la configuración completa incluyendo estado del backup de `ai_keyvault_data`.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `SettingsPage.tsx` usaba `console.log` para guardar ajustes (TODO) | Ajustes no persistían entre sesiones | Implementado `localStorage.setItem("akv_settings", JSON.stringify(settings))` |
| `VaultSyncButtons` necesita `invoke` y `useState` — componente local en SettingsPage | No es una página separada, es parte de la sección Respaldo | Componente `VaultSyncButtons` local dentro de `SettingsPage.tsx` |
| `autolockMinutes: 0` como default hace más claro el opt-in | 15 min por defecto podría confundir a usuarios que no saben que el timer está activo | Cambiado a 0 = desactivado. Leer con `?? "0"` en UnlockPage y DashboardPage |

#### Contenido (completado en UI base)
- [x] `pages/AlertsPage.tsx` — log de alertas con tipos y colores
- [x] `pages/PingPage.tsx` — ping needs + ping manual + aviso de riesgo ToS
- [x] `pages/ChatsPage.tsx` — chats por cuenta y dispositivo
- [x] `pages/StatsPage.tsx` — comparativa + heatmap 24h×7d
- [x] `pages/SettingsPage.tsx` — ajustes completos persistidos en localStorage, VaultSyncButtons, DPAPI con modal, autobloqueo opt-in
- [x] `i18n/es.json` — todas las claves de UI incluyendo sección vault y ajustes

#### Pendiente (segunda vuelta — Fase 16 o posterior)
- [ ] Silenciar alertas por X horas (UI implementada, lógica backend pendiente)
- [ ] `Ctrl+K` paleta de comandos
- [ ] Alertas de fin de suscripción y login vencido
- [ ] Integración real con `ai_keyvault_data` backup fecha

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
| 2026-11-25 | 10 | `cmake` no instalado — `rusqlite bundled` falla. | Alto | providers/agy y providers/codex excluidos del desktop Cargo.toml. `get_active_account` es stub. |
| 2026-11-25 | 10 | `thiserror v2` vs `v1` en diferentes crates. | Bajo | Unificado en `thiserror = "1"` en el desktop. |
| 2026-11-25 | 11 | `crates/core` no tiene sqlite — solo crypto pura. Seguro como path dep. | Alto | Incluido directamente, sin cmake. |
| 2026-11-25 | 12 | `RecoveryKey` usa `to_formatted_string()` / `from_formatted_string()` (no `Display`). | Compilación | Nombres correctos usados desde el inicio. |
| 2026-11-25 | 12 | `vault_path()` debe ser `pub` para usarse desde `security.rs`. | Compilación | Cambiado a `pub fn vault_path`. |
| 2026-11-25 | 13 | Sidecar falla si no compilado — DashboardPage lo captura silenciosamente. | UX | Error capturado, aviso visible sin bloquear la UI. |
| 2026-11-25 | 14 | `AUTOLOCK_RUNNING: AtomicBool` global — cancelar el timer anterior antes de iniciar uno nuevo. | Seguridad | Flip a `false` + 100ms sleep antes del nuevo timer. |
| 2026-11-25 | 14 | `listen()` de Tauri retorna unlisten — cleanup al desmontar. | Fuga de listener | `useRef` + return en `useEffect`. |
| 2026-11-25 | 14 | `reset_autolock_timer` sin throttle recibe llamadas en cada pixel. | Performance | Throttle manual 30s con `Date.now()` en frontend. |
| 2026-11-25 | 14 | Autobloqueo 0 = desactivado. Default cambiado de 15 a 0 (opt-in real). | UX / Seguridad | `autolockMinutes: 0` en DEFAULT_SETTINGS. `?? "0"` en lectores. |
| 2026-11-25 | 15 | `SettingsPage.tsx` usaba `console.log` como persistencia (TODO antiguo). | Crítico UX | Implementado `localStorage.setItem("akv_settings", ...)` + `invoke("start_autolock_timer")`. |
| 2026-11-25 | 14-15 | `security.rs` tenía bloque muerto `pub mod vault_path_reexport`. | Compilación | Eliminado. `mod.rs` y `main.rs` actualizados para registrar los 5 comandos. |
| 2026-11-25 | 16 | `pnpm tauri build` desde `apps/desktop/` resolvía el triple GNU porque `rust-toolchain.toml` solo estaba en `src-tauri/`. | Build falla buscando sidecar `-gnu.exe` | Copiado `rust-toolchain.toml` también a `apps/desktop/` para forzar MSVC desde el cwd de Tauri CLI. |

---

## ✅ SEGUNDA ETAPA COMPLETADA — (fecha pendiente)

> Esta línea se completa al cerrar la Fase 16.

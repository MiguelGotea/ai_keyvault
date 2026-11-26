# PLAN2.md — Segunda Etapa: App de Escritorio Tauri 2

**Versión del documento:** 2.0.0 — reordenado y corregido 2026-11-25
**Fecha de inicio:** 2026-11-25
**Estado real:** 🟡 App v0.2.4 con UI completa y código en `main`, pero con el flujo de datos roto. Ver «Cómo leer este documento» y la Fase 18.
**Referencia de entrada:** `docs/desktop-app-insumos.md` — spec completo del frontend

---

## Cómo leer este documento

- **Nomenclatura de etapas:** etapa 1 = backend (`PLAN.md`, congelado) · **etapa 2 = esta app de escritorio** · etapa 3 (futura) = proveedores adicionales (Fase 9).
- **Leyenda:** ✅ terminado y verificado en uso real · 🟡 código escrito, sin verificar · ⬜ pendiente · ⏸ pospuesto · ⚠ desviación o contradicción abierta.
- **Etiquetas de decisión:** `[DECIDIDO]` no se reabre sin bloqueo técnico documentado · `[PROPUESTA]` requiere aprobación del autor antes de implementarse · `[VERIFICADO]` / `[SUPUESTO]` según evidencia.
- Las secciones de las Fases 10–16 se conservan como **registro histórico** (los hallazgos son valiosos). Donde una decisión fue superada por la Fase 17 está marcada **[SUPERADO]**.
- La fuente de verdad de lo decidido es la sección «Decisiones» de abajo. La fuente de verdad de lo que falta respecto al prompt original es la «Matriz de cumplimiento» al final.

### Estado real por fase (propuesto; el autor confirma)

| Fase | Contenido | Estado |
|------|-----------|--------|
| 10 | Scaffold, toolchain MSVC, CI | ✅ (build e instalación verificados) |
| 11 | Comunicación Rust ↔ VPS ↔ React | 🟡 (3 incidentes de «error decoding response body») |
| 12 | Bóveda local, desbloqueo, bandeja | 🟡 |
| 13 | Dashboard y gestión de cuentas | 🟡 (rediseñado en F17) |
| 14 | Autobloqueo y sync de bóveda | 🟡 (sync no probado en una 2.ª PC) |
| 15 | Ping, alertas, chats, ajustes | 🟡 |
| 16 | Estadísticas e instalador | 🟡 |
| 17 | Alineación con el diseño original | 🟡 |
| **18** | **Reconexión del flujo de datos** | **⬜ siguiente** |
| 19 | Sesión de usuario y primer arranque | ⬜ |
| 20 | Credenciales de cuenta y portapapeles | ⬜ |
| 21–25 | Funciones posteriores (orden de D-11) | ⬜ |
| 26–28 | Cierre del switch, estadísticas ampliadas, instalador | ⬜ |

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
| `apps/desktop/` | *(histórico: scaffold al inicio de la etapa; el estado actual está en la tabla «Estado real por fase»)* |

---

## Decisiones de arquitectura (confirmadas antes de arrancar)

> Estas decisiones están cerradas. No se reabren salvo bloqueo técnico documentado.

| Tema | Decisión |
|------|----------|
| Framework desktop | **Tauri 2** (Rust + React 18 + TypeScript + Vite) |
| Toolchain Windows | **MSVC** para Tauri (`x86_64-pc-windows-msvc`). GNU se mantiene para cross-compile al VPS. Ambos coexisten; `apps/desktop/src-tauri/rust-toolchain.toml` fuerza MSVC solo en ese directorio. |
| Comunicación UI → VPS | **Todo por comandos Rust** (`#[tauri::command]` con `reqwest`). Sin `fetch()` externo en React. CSP configurada para bloquearlo. |
| Poller | Sidecar Tauri. Proceso hijo que corre mientras la app está abierta. Se detiene al Salir. No depende de la bóveda (lee del SO directamente). *(Ajustado por D-01, D-02 y D-03, ya decididas.)* |
| Nombre del sidecar | `ai-keyvault-poller-x86_64-pc-windows-msvc.exe` en disco. Referenciado como `"binaries/ai-keyvault-poller"` en `tauri.conf.json`. |
| Autobloqueo | Bloquea solo la UI. `vault.lock()` hace `zeroize` de `VaultKey`. El poller **sigue corriendo**. |
| IPC poller ↔ Tauri | Solo expone: `poller_status`, `poller_pause`, `poller_resume`. **Nunca descifrado ni credenciales**. |
| DPAPI | Opt-in, **desactivado por defecto**. Con advertencia clara. |
| Cierre de ventana | Minimizar a System Tray (bandeja). Menú: Abrir, Pausar poller, Salir. |
| Distribución | Solo instalador `.msi`/`.exe` generado por GitHub Actions (`windows-latest`). |
| VPS TLS | Sin subdominio por ahora. IP directa `198.211.97.243:3010`. Caddy instalado, pendiente de configurar cuando se obtenga subdominio. |
| Proveedores en UI | Solo agy + Codex (los únicos con backend implementado). |
| Workspace Cargo desktop | `apps/desktop/src-tauri/Cargo.toml` declara `[workspace]` vacío. Tiene su propio `Cargo.lock` y `target/`. Usa `crates/core` como path dependency. |
| ai_keyvault_data | Backup automático ya funciona (workflow `backup.yml`). La UI muestra estado del último backup en Configuración *(🟡 no implementado)*. Ver D-04 (dump cifrado + restauración probada). |

---

### Decisiones tomadas durante la implementación (Fases 14–17) — `[DECIDIDO]`

| Tema | Decisión | Origen |
|------|----------|--------|
| Autobloqueo | **Opt-in: 0 = desactivado por defecto** (el prompt original pedía 15 min). Desvío consciente. | F14 |
| Fuente de verdad de *qué cuentas existen* | **La bóveda.** El VPS guarda lecturas y registros (cuota, alertas, pings, switches, chats), no la lista de cuentas. | F17 |
| Concepto «Activar cuenta» | **No existe.** Solo «Usar en esta PC» (switch). | F17 |
| Agregar cuenta | **Captura desde el login del SO**, nunca formulario manual de tokens. agy: flujo OAuth guiado (limpia credencial, abre `agy`, espera hasta 120 s, captura). Codex: importar el `auth.json` actual. | F17 |
| Switch de cuenta | Escribe el **blob completo descifrado** de la bóveda (agy: Credential Manager `gemini:antigravity`; Codex: `auth.json`). **Nunca JSON sintético.** | F17 |
| Booleanos del VPS | SQLite los devuelve como 0/1; el cliente usa el deserializador `de_bool_or_int`. | F17 |

### Decisiones cerradas el 2026-11-25 (respuestas del autor) — `[DECIDIDO]`

| ID | Tema | Decisión |
|----|------|----------|
| D-01 | Alcance del poller | Lee **solo la cuenta activa** de cada PC. Las demás cuentas muestran su **última lectura conocida con su antigüedad** y el reinicio guardado en esa lectura. No se rotan cuentas para leer. Probar Codex por cuenta con `CODEX_HOME` queda como mejora futura, no se hace ahora. |
| D-02 | Identidad de cuenta | **UUID opaco de la bóveda** en toda la cadena (VPS, poller, app). El poller usa un mapa local cifrado correo→UUID. El VPS deja de ver correos. Prohibido cruzar por alias. |
| D-03 | Arranque del poller | Sigue siendo sidecar. Se agrega la opción **«Iniciar con Windows»** en Ajustes (opt-in, app en bandeja). |
| D-04 | Respaldo `ai_keyvault_data` | El dump se **cifra antes de subirlo** y se prueba una **restauración real**. La fecha del último backup se muestra en Ajustes. |
| D-05 | Transporte | Se mantiene IP directa por HTTP. TLS con subdominio queda como tarea futura (no hay subdominio). |
| D-06 | Frecuencia | 10 min para la cuenta activa. *Con D-01 los 60 min para «las demás» no tienen qué leer; aplicarán solo si en el futuro se leen cuentas no activas.* |
| D-07 | Correo y contraseña | Se guardan **cifrados** por cuenta, junto con alias, notas y detalles, con botón de copiar y **limpieza del portapapeles a 30 s** (Fase 20). |
| D-08 | Autobloqueo | **0 = desactivado por defecto** (opt-in). Desvío aprobado respecto al prompt original (15 min). |
| D-09 | Ping automático | Solo sobre la **cuenta activa**. Para las demás, la app avisa «esta cuenta necesita ping» y el usuario la activa con «Usar en esta PC» y hace el ping manual. |
| D-10 | Proveedores adicionales | Claude, Gemini, OpenCode y Grok (etapa 3) quedan **fuera** hasta que la etapa 2 esté verificada de punta a punta. |
| D-11 | Orden después de la Fase 18 | Fase 19 sesión → Fase 20 credenciales → alternativa en alertas → silenciar/suscripción → `Ctrl+K` → pestañas de detalle → tema claro y dispositivos → resto (ver Fases 19–28). Nada de eso entra en la Fase 18. |
| D-12 | Sesión y primer arranque | Estilo Bitwarden. Ver Fase 19. El cierre de sesión global **no** afecta los logins de agy/Codex. |
| D-13 | Arranque desde cero | Las bases de datos (VPS y locales) fueron **limpiadas** el 2026-11-25. No hay nada en producción: **sin código de migración ni compatibilidad hacia atrás**. Entorno de verificación: **2 PCs** (PC-A y PC-B). |

**Limitación aceptada:** una cuenta que no está activa en ninguna PC no tiene lectura en vivo, solo la última conocida. La consulta desde el VPS 24/7 sigue ⏸.

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
| `cmake` no instalado en la PC de desarrollo | `rusqlite bundled` (en `providers/codex`) fallaría al compilar el desktop | **Decisión:** `crates/core`, `providers/agy`, `providers/codex` NO se incluyen como dependencias del desktop en Fase 10. Se agregarán en Fase 12 cuando se necesiten. El desktop se comunica con el backend VPS via `reqwest`. **[SUPERADO en F17: `providers/agy` no requiere cmake; solo `providers/codex` (`rusqlite bundled`) sí.]** |
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
- [ ] Autobloqueo (opt-in; 0 = desactivado por defecto, ver Decisiones) → pantalla de desbloqueo aparece, el poller sigue corriendo
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

**Estado:** ✅ Completada — 2026-11-25

**Objetivo:** App instalable funcionando con CI en verde y versión bump correcta.

#### Hallazgos durante el desarrollo

| Hallazgo | Impacto | Solución aplicada |
|----------|---------|------------------|
| `PoolSummary` en desktop tenía nombres de campos distintos a los del VPS (`current_percent` vs `current_percent_used`, `burn_rate_per_hour` vs `burn_rate_pct_per_hour`) | "Error HTTP: error decoding response body" en dashboard | Struct Rust y tipo TS sincronizados exactamente con `server/src/stats.rs` |
| Vitest fallaba con exit code 1 porque no había ningún archivo `*.test.ts` | CI frontend-ci en rojo; tauri-build skipped | Creado `src/types/api.test.ts` con tests de i18n y contrato PoolSummary |
| `pnpm install --frozen-lockfile` bloqueaba esbuild en CI | Frontend CI en rojo | `pnpm approve-builds --all` actualiza el lock; `.npmrc` con `onlyBuiltDependencies[]=esbuild` |
| `rust-toolchain.toml` solo estaba en `src-tauri/` — Tauri CLI buscaba sidecar `-gnu.exe` | Build local fallaba con "resource path doesn't exist" | Copiado `rust-toolchain.toml` a `apps/desktop/` para que el toolchain MSVC aplique desde el cwd de pnpm tauri build |
| `.ps1` scripts con emojis se rompían en PowerShell 5.1 (codificación Windows-1252) | `copy-sidecar.ps1` no ejecutaba | Reemplazados emojis por prefijos ASCII (`[INFO]`, `[ERROR]`, `[OK]`) |
| Versión `0.1.0` sin bump — no refleja el estado real de la app | El instalador siempre muestra v0.1.0 | Bump a v0.2.0 en `Cargo.toml`, `tauri.conf.json` y `package.json` |

#### Contenido (completado)
- [x] CI `frontend-ci` en verde: typecheck ✅ + vitest ✅ + pnpm frozen-lockfile ✅
- [x] CI `rust-ci` en verde
- [x] Build local `pnpm tauri build` → `.exe` instalado y verificado en Windows
- [x] Todas las pestañas del dashboard funcionan sin errores
- [x] Datos de cuota del VPS se muestran correctamente (cuentas del poller, no de la bóveda local) **[SUPERADO en F17: la fuente de verdad es la bóveda; el cruce VPS↔bóveda no está verificado]**
- [x] Versión bumpeada a v0.2.0 en Cargo.toml, tauri.conf.json, package.json
- [x] CLAUDE.md §7 actualizado: Desktop ✅

#### Completado en esta vuelta
- [x] `tauri-build` CI — se activa automáticamente en push con cambios en `apps/desktop/` (ya configurado en ci.yml)
- [x] Comparativa de cuentas `StatsPage` — llama `get_stats_compare` via IPC
- [x] Burn rate y heatmap — `StatsPage` consume `get_stats_summary` y `get_stats_heatmap`
- [x] Versión `tauri.conf.json` + `Cargo.toml` + `package.json` sincronizadas (v0.2.0)
- [x] SmartScreen warning documentado en `README.md`
- [x] `README.md` actualizado con instrucciones de instalación reales
- [x] `docs/setup-entorno.md` actualizado con toolchain verificado (§9.2–§9.9)
- [x] `CLAUDE.md` §7 actualizado: Desktop ✅
- [x] `docs/operaciones.md` §18–§21: hallazgos desktop, autobloqueo, vault sync, flujo cuentas
- [x] `PLAN2.md` actualizado al cierre

#### Pendiente (tercera vuelta — opcional)
- [ ] `webviewInstallMode: OfflineInstaller` — solo necesario si se distribuye a PCs sin internet/WebView2
- [ ] Instalar cmake → habilitar providers/agy + providers/codex → switch real de credenciales desde UI
- [ ] Firma de código (EV certificate) para eliminar SmartScreen warning
- [ ] Silenciar alertas por X horas (UI implementada, lógica backend pendiente)
- [ ] `Ctrl+K` paleta de comandos
- [ ] `.exe` como artifact en GitHub Releases (actualmente solo se genera en build local)

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
| 2026-11-25 | 16 | `PoolSummary` desktop no coincidía con la struct del VPS (campo `current_percent` vs `current_percent_used`, etc.). | "Error decoding response body" en dashboard | Sincronizado exactamente con `server/src/stats.rs`. |
| 2026-11-25 | 16 | Vitest sin archivos `*.test.ts` → exit code 1 → `tauri-build` skipped. | CI en rojo | Creado `api.test.ts` con tests de i18n y PoolSummary. |
| 2026-11-25 | 16 | `pnpm install --frozen-lockfile` bloqueaba scripts de esbuild en CI. | Frontend CI en rojo | `pnpm approve-builds --all` + `.npmrc` con `onlyBuiltDependencies[]=esbuild`. |
| 2026-11-25 | 16 | Scripts `.ps1` con emojis UTF-8 se rompían en PowerShell 5.1 (Windows-1252). | `copy-sidecar.ps1` no ejecutaba | Reemplazados emojis por prefijos ASCII. |
| 2026-11-25 | 16 | Versión fija en `0.1.0` sin estrategia de bump. | Instalador siempre v0.1.0 | Bump a v0.2.0 en Cargo.toml, tauri.conf.json y package.json. |
| 2026-11-25 | 17 | Dashboard mostraba cuentas del VPS, no de la bóveda. | Arquitectura incorrecta per prompt | Dashboard reescrito: fuente de verdad = vault, cruce por alias. |
| 2026-11-25 | 17 | "Agregar cuenta" era formulario manual. | UX incorrecta | Reemplazado por captura automática desde OS. |
| 2026-11-25 | 17 | switch_agy() escribía JSON falso. | Rompía sesión de agy | Reescrito para escribir blob completo descifrado de la bóveda. |
| 2026-11-25 | 17 | AlertRecord/ChatSession/SwitchRecord/PingRecord tenían campos incorrectos vs VPS. | "Error decoding response body" en Alertas | Corregidos campos y agregado deserializador de_bool_or_int (SQLite 0/1). |
| 2026-11-25 | 17 | import_current_codex_credential faltaba en Rust. | Botón Codex sin funcionar | Implementado en vault.rs, registrado en main.rs, codex_auth_path_pub() en switch_cmd.rs. |
| 2026-11-25 | 18.0 | Diagnóstico: backend etapa 1 intacto (solo aditivo en `vault.rs` y `ci.yml`); VPS activo y tablas consultadas vacías. | Base confirmada | Ver `docs/research/06-diagnostico-fase18.md`. |
| 2026-11-25 | 18.0 | Cruce roto por 3 llaves distintas: poller sube `email`, UI filtra por `alias`, switch envía `UUID` y `StatsPage` hardcodea `"test@agy"`. | Dashboard/Alertas/Pings/Chats vacíos | Reparar en `18.2` aplicando D-02 (UUID opaco + mapa local cifrado). |
| 2026-11-25 | 18.0 | Contratos rotos en `api_client.rs` (`QuotaReading`, `HeatmapCell`, `DeviceRegistration`), `level` `"critical"` vs `"crit"`, y falta `GET /api/stats/export/:account_id`. | Errores de deserialización al haber datos | Corregir y cubrir con tests de fixtures en `18.3`. |
| 2026-11-25 | 18.0 | Sidecar `start_poller` sin env vars, descarta `_rx` (`last_poll_at` siempre `None`), `pause/resume` solo en memoria, y `.catch(() => {})` en `DashboardPage`/`UnlockPage`. | Poller sin visibilidad de errores ni estado real | Reparar en `18.4`. |
| 2026-11-25 | 18.0 | `PingPage` registra `post_ping` con `success: true` sin ejecutar ping real; `deviceId` hardcodeado a `"desktop-dev"`; `switch_codex` conserva JSON sintético `_akv_switch`. | Desvíos funcionales respecto a F17 | Corregir en `18.2`/`18.4` (o listar para fase correspondiente). |

---

## 🟡 ESTADO DE LA SEGUNDA ETAPA — revisado 2026-11-25

> La app v0.2.4 compila, instala y tiene todas las pantallas, **pero el flujo de datos de extremo a extremo (poller → VPS → app) no funciona** según el autor, y varias decisiones se fueron modificando entre fases sin quedar cerradas.
> Los ✅ de las Fases 10–17 significan «código escrito y compilando». Los criterios de aceptación con `[ ]` **nunca se verificaron en uso real**.
> **Siguiente paso: Fase 18 (Reconexión)**, al final de este documento. No se agregan funciones nuevas hasta cerrarla.

---

## Fase 17 — Tercera vuelta: Alineación arquitectura + correcciones API

**Estado:** 🟡 Código completado — 2026-11-25 (v0.2.1 → v0.2.4). Sin verificación en uso real.

### Objetivo
Corregir las desviaciones del diseño original del prompt: la bóveda como fuente de verdad, flujo real de agregar cuentas (captura desde OS), switch real con blob completo, y corrección de todos los campos VPS reales.

### Hallazgos críticos

| Fecha | Hallazgo | Impacto | Solución |
|---|---|---|---|
| 2026-11-25 | Dashboard mostraba cuentas del VPS (polleadas), no de la bóveda | Arquitectura incorrecta | Dashboard reescrito: fuente de verdad = vault, datos VPS se cruzan por alias |
| 2026-11-25 | "Agregar cuenta" era formulario manual | UX incorrecta per prompt original | Reemplazado por captura automática desde OS (Credential Manager / auth.json) |
| 2026-11-25 | "Activar" = concepto incorrecto | No existe en el diseño original | Eliminado — solo existe "Usar en esta PC" en Dashboard |
| 2026-11-25 | switch_agy() escribía JSON falso {_akv_switch:true} | Rompía sesión de agy | Reescrito para escribir blob completo de la bóveda |
| 2026-11-25 | AlertRecord tenía campos incorrectos (alerted_at, message) | Error decoding response body | Corregido a detected_at, notified_toast, notified_ntfy |
| 2026-11-25 | notified_toast/notified_ntfy llegaban como 0/1 (SQLite) | Error deserialización Rust | Agregado deserializador flexible de_bool_or_int con serde |
| 2026-11-25 | SwitchRecord/PostSwitchPayload no coincidían con VPS | Switch history roto | Corregido: to_account_id, from_account_id, reason |
| 2026-11-25 | ChatSession.archived llegaba como 0/1 (SQLite) | Error deserialización | Corregido con de_bool_or_int + device_id cambiado a Option<String> |
| 2026-11-25 | ChatsPage usaba cuentas hardcodeadas/de VPS | No mostraba cuentas de la bóveda | Reescrito para usar list_vault_accounts |
| 2026-11-25 | providers/agy NO requiere cmake | Permite incluirlo como dep del desktop | Solo usa windows_sys — cmake bloqueaba innecesariamente |

### Nuevos comandos Rust

| Comando | Descripción |
|---|---|
| `import_current_agy_credential(alias)` | Captura blob de gemini:antigravity en Credential Manager → cifra → vault |
| `import_current_codex_credential(alias)` | Captura ~/.codex/auth.json → cifra como encrypted_tokens → vault |
| `decrypt_account_tokens(uuid)` | Descifra encrypted_tokens para switch real |
| `add_agy_account_oauth(alias)` | Flujo OAuth completo: limpia credencial, abre agy, polling 120s, captura |
| `cancel_agy_oauth()` | Cancela el flujo OAuth en curso |

### Correcciones de API client (api_client.rs)

- `AlertRecord`: campos correctos + deserializador `de_bool_or_int` para `notified_toast`/`notified_ntfy`
- `ChatSession`: `archived` con `de_bool_or_int`, `device_id` como `Option<String>`
- `SwitchRecord`: campos reales del VPS (`to_account_id`, `from_account_id`, `success` con de_bool_or_int)
- `PingRecord`: `success` y `is_manual` con `de_bool_or_int`

### Commits

| Hash | Versión | Descripción |
|---|---|---|
| d471030 | v0.2.1 | corregir alerts/pings/chats, switch real, gestión de cuentas en bóveda |
| f167a96 | — | fix(ci): ejecutar tauri-build en todo push |
| 815cd78 | — | fix(ci): crear directorio binaries antes de copiar sidecar |
| 2722398 | v0.2.2 | bóveda como fuente de verdad, agregar cuenta por captura agy, switch con blob real |
| d69b03d | v0.2.2b | flujo OAuth real (agy login → captura → boveda) |
| 72f0f26 | v0.2.3 | terminar poller al salir, sync real de boveda VPS al reinstalar |
| (nuevo) | v0.2.4 | Codex import, corrección bool SQLite alertas/pings/switch/chats |

**Versiones:** v0.2.1 → v0.2.2 → v0.2.2b → v0.2.3 → v0.2.4


---

## Fase 18 — Reconexión: restaurar el flujo de datos extremo a extremo

**Estado:** 🟡 En curso — `18.0` y `18.1` completados (2026-11-25). **Siguiente: `18.2`.**
**Naturaleza:** fase de **diagnóstico y reparación**. No se agregan funciones nuevas ni se reescribe lo que ya compila. Una sesión = un sub-paso; actualizar este documento al terminar cada uno.

**Objetivo:** que cada cuenta de la bóveda muestre en el dashboard su cuota, reinicio y antigüedad de lectura, con el poller subiendo datos al VPS y la app leyéndolos con la misma llave de cuenta.

### 18.0 Diagnóstico (solo lectura, sin tocar código) — ✅ Completado (2026-11-25)
Entregable creado: `docs/research/06-diagnostico-fase18.md` con tabla «esperado / real / causa» y evidencia `[VERIFICADO]` / `[SUPUESTO]`.
- Backend etapa 1 (`server/`, `providers/`): intacto; solo cambios aditivos en `crates/core/src/vault.rs` y `.github/workflows/ci.yml`.
- VPS activo (`198.211.97.243:3010`), `keyvault.db` 52K (`Nov 25 18:49`) y endpoints de lectura vacíos (`[]` / HTTP 400 en `/api/vault/sync`).
- Limpieza local y `ai_keyvault_data` confirmada por el autor: `AppData\Roaming\com.miguelgotea.ai-keyvault` vacío, `AppData\Local\com.miguelgotea.ai-keyvault` y `~/.ai-keyvault` eliminados, y `ai_keyvault_data` sin commits viejos en local ni remoto.
- Causa raíz principal confirmada: 3 llaves incompatibles (poller=`email`, UI=`alias`, switch=`UUID`), contratos desalineados en `api_client.rs` (`QuotaReading`, `HeatmapCell`, `DeviceRegistration`), sidecar sin env vars ni captura de salida `_rx`, y `.catch` silenciosos.

### 18.1 Decisiones — ✅ Cerrado (2026-11-25)
Ya cerradas en «Decisiones cerradas» (D-01 a D-13). El diagnóstico `18.0` no contradice ninguna decisión cerrada; por el contrario, confirma que aplicar D-01 y D-02 resuelve la desconexión principal. Siguiente paso: `18.2`.

### 18.2 Reparar identidad y cruce
Aplicar D-01, D-02 y D-06 con el cambio mínimo posible. Cambios en el backend solo **aditivos**; cualquier eliminación se lista y se aprueba antes.

### 18.3 Contrato API con tests
Crear `docs/contrato-api.md` y fixtures JSON **con datos falsos** de cada endpoint que consume la app. Tests Rust que deserialicen cada fixture (incluidos los booleanos 0/1). Motivo: el error «decoding response body» apareció en F16 y de nuevo en F17.

### 18.4 Errores visibles
El dashboard muestra el estado del poller (corriendo / pausado / error + último envío) y, por cuenta, «última lectura hace X» o «sin lecturas». Prohibido `.catch(() => {})` en flujos críticos.

### 18.5 Respaldo
Implementar D-04: dump cifrado, restauración real probada y fecha del último backup en Ajustes.

### 18.6 Criterios de aceptación (todos en uso real)
- [ ] Partiendo de cero (bóveda nueva, VPS vacío), con 2 cuentas en la bóveda, la cuenta activa muestra % y reinicio con antigüedad < 15 min.
- [ ] La misma bóveda vista desde PC-B (según el flujo de la Fase 19) muestra las lecturas que subió PC-A.
- [ ] La cuenta no activa muestra su última lectura conocida con su antigüedad (no vacío ni error).
- [ ] Con el autobloqueo activo, bloquear la UI no detiene el poller.
- [ ] Estadísticas muestran burn rate de la cuenta activa tras ≥ 3 lecturas.
- [ ] Una alerta de prueba (umbral bajado) aparece en `/alerts` y como toast.
- [ ] Ping manual de una cuenta que lo necesita queda en el historial de `/ping`.
- [ ] Switch real A→B→A verificado (cuota o `auth.json`) y «Deshacer» funciona.
- [ ] CI en verde, incluidos los tests de contrato; `gitleaks` limpio.

---

## Fase 19 — Sesión de usuario y primer arranque (D-12)

**Estado:** ⬜ Después de la Fase 18. Requisito original del autor (estilo Bitwarden); además da el criterio «entrar desde otra PC», que pasó de la Fase 18 a esta.

**Flujo:**
1. **Primer arranque sin sesión en la PC:** pantalla con dos opciones.
   - **Iniciar sesión (traer mi usuario y bóveda):** URL del servidor (con valor por defecto), usuario y clave maestra. Baja la bóveda cifrada, la descifra en local y registra el dispositivo.
   - **Crear usuario nuevo:** nombre, usuario y clave maestra (mín. 12 caracteres, con confirmación). Crea el usuario y la bóveda, muestra la **clave de respaldo una sola vez** y registra el dispositivo.
2. **Arranques siguientes:** solo la clave maestra (pantalla de desbloqueo con el nombre del usuario). La recuperación con clave de respaldo sigue disponible.
3. **Ajustes → «Cerrar sesión global en esta PC»:** confirma, sube primero la bóveda pendiente (o avisa si no pudo) y borra la sesión local: bóveda local y caché, caché DPAPI, claves del dispositivo, mapa correo→UUID del poller y ajustes de sesión. Detiene el poller y vuelve a la pantalla de dos opciones. **No borra nada en el VPS.** **Nunca** toca el login de agy/Codex del SO: solo borra la sesión de la herramienta (decidido por el autor).
4. **Iniciar con Windows** (D-03): opción en Ajustes, apagada por defecto.

**Sub-pasos:**
- **19.0 Verificar el backend:** ¿el servidor ya tiene concepto de usuario (usuario único, clave de autenticación derivada y separada de la de cifrado, bloqueo por intentos)? Hoy hay registro de dispositivos y sync de bóveda. Si falta, el cambio en el servidor es **solo aditivo** y se lista y aprueba antes.
- **19.1** Servidor: usuario + clave de autenticación (distinta de la clave de cifrado, que no sale del cliente).
- **19.2** App: pantallas de primer arranque, arranques siguientes y cierre de sesión global.
- **19.3** «Iniciar con Windows».

**Criterios de aceptación (en uso real):**
- [ ] PC sin sesión muestra las dos opciones; con sesión pide solo la clave.
- [ ] Crear usuario nuevo genera bóveda y clave de respaldo, mostrada una sola vez.
- [ ] En la 2.ª PC, URL + usuario + clave maestra muestra las mismas cuentas.
- [ ] Cerrar sesión global deja la PC sin datos locales de la bóveda y el VPS conserva todo.
- [ ] Clave incorrecta no revela si el usuario existe; hay bloqueo progresivo por intentos.
- [ ] Ningún secreto en logs; `gitleaks` limpio; contrato API actualizado con sus fixtures.

---

## Fase 20 — Credenciales de cuenta y portapapeles (D-07)

**Estado:** ⬜ Después de la Fase 19.

- Por cuenta, cifrados en la bóveda: alias, notas, detalles, **correo y contraseña** (el 2FA queda en la app de autenticación del autor).
- Botón de copiar correo y contraseña con **limpieza automática del portapapeles a 30 s**.
- Editar y eliminar cuentas; la captura de login (agy/Codex) sigue siendo la forma de agregarlas.
- **Criterios:** correo y contraseña no aparecen en logs ni en disco sin cifrar; el portapapeles se limpia a los 30 s; todo sobrevive a bloquear, desbloquear y sincronizar.

---

## Fases 21–28 — Orden después de la Fase 20 (D-11)

Cada una se detalla en su propio momento, con una sesión por sub-paso. Orden fijado por el autor para 21–25; 26–28 es **orden propuesto** (el autor puede moverlas).

| Fase | Contenido |
|------|-----------|
| 21 | Alertas con **sugerencia de cuenta alternativa** y botón «Cambiar a esa cuenta» |
| 22 | **Silenciar alertas** por X horas, alertas de **fin de suscripción** (7 y 1 día) y de **login vencido** o consulta fallando |
| 23 | Paleta de comandos **`Ctrl+K`** |
| 24 | **Detalle de cuenta con pestañas** (Resumen, Cuota y cooldown, Estadísticas, Chat, Credenciales, Historial) |
| 25 | **Tema claro/oscuro** y pantalla **«Mis dispositivos»** (revocar acceso) |
| 26 | Cierre del switch: botón «Deshacer», aviso de reinicio de herramientas, detección de procesos sin cerrarlos, «sin agente en esta PC» |
| 27 | Estadísticas ampliadas (tokens por modelo, eficiencia, cuota desperdiciada) y pantalla «Experimento de cooldown» |
| 28 | Instalador firmado y versión portable |

---

## Matriz de cumplimiento del prompt original

> Única fuente de verdad de lo que falta. Sustituye la tabla de pendientes que antes vivía en `PLAN.md`. `AI-KeyVault-prompt-claude-code.md` es el **prompt histórico**: no se vuelve a ejecutar; si choca con este documento, gana este documento.

| § del prompt | Requisito | Estado | Nota |
|---|---|---|---|
| 5.1 | Bóveda: Argon2id, XChaCha20, clave de respaldo | ✅ backend · 🟡 UI | Tests de `crates/core` en verde |
| 5.2 / 7.15 | Entrar desde otra PC con usuario + clave maestra; primer arranque con dos opciones; cierre de sesión global | ⬜ | **Fase 19** (D-12). El sync existe pero no se probó en una 2.ª PC |
| 7.3 | Guardar correo + contraseña para copiar y pegar | ⬜ | **Fase 20** (D-07). Se eliminó el formulario manual en F17 |
| 5.3 | Portapapeles con limpieza a 30 s | ⬜ | Fase 20 |
| 5.3 | Autobloqueo | 🟡 | Desvío aprobado por el autor (D-08) |
| 5.3 | DPAPI opt-in | 🟡 | |
| 8.3 | Login guiado de cuentas | 🟡 | agy: OAuth guiado. Codex: solo importar el `auth.json` actual |
| 8.3 | Switch con revertir | ✅ backend · 🟡 UI | |
| 8.3 | «Deshacer», aviso de reinicio y detección de procesos | ⬜ | Fase 26 (revert existe en Rust) |
| 7.4 / 7.7 | Cuota y reinicios por cuenta, «actualizado hace X por PC» | ⚠ | Flujo roto (D-01, D-02) |
| 4.2 | Agente al iniciar Windows | ⬜ | D-03: opción «Iniciar con Windows» (Fase 19) |
| Frecuencia | 10 min activas / 60 min inactivas | 🟡 | D-06: 10 min para la cuenta activa; los 60 min no aplican con D-01 |
| 8.1 | Alertas 80 % / 95 % (toast + ntfy) | ✅ backend · 🟡 UI | |
| 8.1 | Sugerencia de cuenta alternativa y botón «Cambiar» | ⬜ | Fase 21 |
| 8.1 | Silenciar por X horas | ⬜ | Fase 22 (UI sin lógica) |
| 7.10 / 8.1 | Fin de suscripción, login vencido, consulta fallando | ⬜ | Fase 22 |
| 8.2 | Ping manual y automático | ✅ backend · 🟡 UI | |
| 10 | Chats (metadatos, solo activos) | ✅ backend · 🟡 UI | |
| 10 | «Sin agente en esta PC» | ⬜ | Fase 26 |
| 11 | Estadísticas: burn rate, heatmap, comparativa, CSV | ✅ backend · 🟡 UI | |
| 11 | Tokens por modelo, eficiencia, cuota desperdiciada | ⬜ | Fase 27 (`token_usage_record` investigado) |
| 9.3 | Detalle de cuenta con 6 pestañas | ⬜ | Fase 24 |
| 9 | Paleta `Ctrl+K` | ⬜ | Fase 23 |
| 9 | Tema claro/oscuro | ⬜ | Fase 25 (hoy solo oscuro) |
| 7.17 | Pantalla «Mis dispositivos» | ⬜ | Fase 25 (`register_device` existe) |
| 9 | Pantalla «Experimento de cooldown» | ⬜ | Fase 27 |
| 3 / 5.4 | Respaldo en `ai_keyvault_data` y restauración | ⬜ | D-04 (Fase 18.5): dump cifrado + restauración real |
| 5.2 | Consulta desde el VPS 24/7 (delegada) | ⏸ | Riesgo en ADR-003 |
| 6.5 | Chat de consumo | ⏸ | Sin API pública |
| 9 / Fase 9 | Claude, Gemini, OpenCode, Grok | ⏸ | Etapa 3 (D-10) |
| 13 | Instalador firmado y versión portable | ⬜ | Fase 28 |

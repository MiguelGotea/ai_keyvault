# CLAUDE.md — Reglas de trabajo para AI KeyVault

> Este archivo es leído automáticamente por el agente de desarrollo al abrir el proyecto.
> **No eliminar. No modificar sin actualizar la versión.**

Versión: 2.2.0 | Fecha: 2026-11-26

---

## 1. Qué es este proyecto

**AI KeyVault** es una bóveda + tablero personal para administrar cuentas de agentes de IA de programación (Antigravity/agy, Codex, y en la etapa 3 Claude, Gemini, Grok y otros). Es una **herramienta personal** del autor para sus propias cuentas en sus propias PCs. No tiene fines comerciales ni de distribución masiva.

**Estado al 2026-11-26:**
- **Etapa 1 (backend)** ✅ operativa: servidor VPS + poller + alerter + providers.
- **Etapa 2 (app de escritorio Tauri 2)** ✅ v0.2.5: Fases 18–28 completadas y verificadas (sesión multi-PC, bóveda cifrada, poller/pings/switch con deshacer, alertas inteligentes, paleta `Ctrl+K`, detalle con 6 pestañas, tema claro/oscuro, «Mis dispositivos», estadísticas ampliadas, «Experimento de cooldown» y versión portable `.zip`).
- **Etapa 3 (futura):** proveedores adicionales (Fase 9).

*(«Segunda etapa» significa siempre la app de escritorio.)*

**Alcance ético (obligatorio):**
- Uso exclusivo con cuentas propias en PCs propias.
- No evade bloqueos ni comparte cuentas con terceros.
- La rotación de cuentas y el ping son decisiones del usuario, configurables y con registro completo.
- La herramienta no roba credenciales, tokens ni claves de nadie.

### 1.1 Norte del producto (qué debe hacer la app — no reinterpretar)

0. **Sesión (estilo Bitwarden).** Si la PC no tiene sesión, la app ofrece dos opciones: **iniciar sesión** (traer mi usuario y bóveda existentes con URL, usuario y clave maestra) o **crear usuario nuevo** (nombre, usuario y clave maestra, que crea también la bóveda). En los arranques siguientes solo pide la clave maestra. En Ajustes existe **«Cerrar sesión global»**, que borra la sesión de esa PC (no los datos del VPS).
1. **Bóveda.** Guarda cuentas (agy, Codex) con alias, notas y detalles, y su correo, contraseña y tokens de sesión, todo **cifrado**. En otra PC basta iniciar sesión para ver todo (el servidor nunca descifra).
2. **Agregar cuenta = login guiado.** La app abre el login oficial del proveedor en el navegador; al terminar, el SO queda logueado y la app **captura** ese login y lo guarda como cuenta de la bóveda. Nunca un formulario manual de tokens.
3. **Cambiar de cuenta con un clic.** La app escribe el login descifrado en el SO (agy: Credential Manager `gemini:antigravity`; Codex: `~/.codex/auth.json`), verifica y revierte si falla. Sin pedir clave ni abrir navegador. Avisa que hay que reiniciar herramientas; **nunca cierra procesos**.
4. **Monitoreo (poller).** Lee periódicamente la cuota por ventana (% usado, fecha de reinicio) y la sube al VPS. La app muestra **cada cuenta de la bóveda** con su última lectura y su antigüedad. *En vivo solo se lee la cuenta activa de cada PC (el login del SO es global); las demás muestran la última lectura conocida.* De ahí salen las **estadísticas** (burn rate, heatmap, comparativa, CSV) y las **alertas** de límite (80 % / 95 %: toast de Windows + ntfy).
5. **Ping de cooldown.** El reloj de la ventana empieza con el primer uso (H1 verificada). Cuando una cuenta recupera cuota, un ping mínimo (manual por defecto, automático opcional y solo sobre la cuenta activa) reinicia su rotación.
6. **Chats.** Historial de solo lectura (metadatos) de los proyectos en curso por PC. No se reanudan desde la app ni se sube su contenido.
7. **Respaldo.** `ai_keyvault_data` guarda snapshots **cifrados** por si el VPS se pierde.

**Reparto de datos:** la **bóveda** es la fuente de verdad de *qué cuentas existen*. El **VPS** guarda *lecturas y registros* (cuota, alertas, pings, switches, chats). Ambos se unen por **una sola llave de cuenta** (ver `PLAN2.md` D-02), nunca por alias.

**Estado de los datos:** arranque desde cero. Las bases de datos (VPS y locales) se limpiaron el 2026-11-25 y no hay nada en producción: **no escribir código de migración ni de compatibilidad hacia atrás**. Entorno de pruebas: 2 PCs.

**Lo que NO es:** panel web, herramienta para compartir cuentas, evasión de bloqueos, ni algo que cierre procesos.

### 1.2 Reglas anti-deriva (obligatorias en cada sesión de trabajo)

1. **Al empezar:** leer este archivo, `PLAN2.md` (tabla «Estado real», «Decisiones» y «Fase» vigente). Si dos documentos se contradicen, **detenerse y preguntar**; no elegir en silencio.
2. **Una fuente de verdad por tema:**

| Tema | Único lugar |
|------|-------------|
| Visión del producto | `CLAUDE.md` §1.1 |
| Decisiones (D-01 a D-12, cerradas el 2026-11-25) y estado de la etapa 2 | `PLAN2.md` |
| Historia del backend (etapa 1) | `PLAN.md` (congelado) |
| Lo que falta respecto al prompt original | `PLAN2.md` → «Matriz de cumplimiento» |
| Contrato API VPS ↔ cliente | `server/src/` + `docs/contrato-api.md` |
| Operación del VPS | `docs/operaciones.md` |

3. **Propuesta ≠ decisión.** Lo marcado `[PROPUESTA]` o «decisión abierta» **no se implementa** sin aprobación del autor. Hoy no hay decisiones abiertas: D-01 a D-12 están cerradas en `PLAN2.md`. Lo `[DECIDIDO]` no se reabre sin bloqueo técnico documentado.
4. **El backend de la etapa 1 es contrato congelado** (`server/`, `crates/`, `providers/`, `.github/`). Solo cambios **aditivos**. Antes de eliminar o reescribir algo: listarlo, justificarlo, registrarlo en `PLAN2.md` y obtener aprobación.
5. **✅ significa verificado en uso real.** Si solo hay código: 🟡. Un criterio de aceptación sin marcar `[x]` no está cumplido.
6. **Contrato de datos:** al tocar una estructura compartida (VPS ↔ Rust ↔ TypeScript) se actualizan el contrato y su test con fixtures falsos en el mismo commit.
7. **Errores nunca silenciosos.** Prohibido `.catch(() => {})` o ignorar `Result` en flujos críticos (poller, sync, switch, lecturas). Todo error llega a la UI en español.
8. **No ampliar alcance.** Una sesión = un sub-paso. Terminar → actualizar `PLAN2.md` → parar.
9. **`AI-KeyVault-prompt-claude-code.md` es el prompt histórico.** No se re-ejecuta ni se toma como spec; las divergencias aprobadas están en `PLAN2.md`.

---

## 2. Reglas de seguridad (no negociables)

### 2.1 Secretos — NUNCA en el repositorio

| Categoría | Ejemplos |
|-----------|---------|
| Contraseñas reales | passwords de cuentas del autor |
| Correos reales | emails de cuentas del autor |
| Tokens de acceso | access tokens, refresh tokens, id_tokens |
| Blobs de sesión | auth.json de Codex, blobs de Credential Manager de agy |
| Claves de cifrado | clave maestra, clave de respaldo, vault key |
| Llaves SSH | llaves privadas de cualquier tipo |
| Archivos .env reales | cualquier .env con datos reales |
| Tópico de ntfy | el tópico aleatorio de notificaciones (`AKYV_NTFY_TOPIC`) |
| Snapshots sin cifrar | dumps de BD o bóveda sin cifrar |
| Capturas con datos | screenshots con datos reales visibles |

**El CI tiene `gitleaks` y bloquea el merge si detecta alguno de los anteriores.**

### 2.2 Datos de prueba

Los fixtures de tests usan **datos falsos, obviamente falsos**:
```
email: test-fake-user@example.com
password: FakePass123!
token: fake_token_do_not_use_aabbccdd
account_id: 00000000-0000-0000-0000-000000000000
```

### 2.3 Repo `ai_keyvault_data`

Solo recibe snapshots **ya cifrados** del lado del cliente. El servidor no puede descifrar nada.

---

## 3. Idioma y formato

- **Interfaz de usuario:** 100 % español. Ningún texto en inglés visible para el usuario.
- **Código (variables, funciones, módulos):** inglés (convención de la industria).
- **Comentarios en código:** español (para facilitar revisión del autor).
- **Commits:** español, formato convencional: `tipo(alcance): descripción`
- **Tipos de commit válidos:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`, `perf`, `security`
- **Documentación técnica (`docs/`):** español.

---

## 4. Colores y marca

| Token | Valor | Uso |
|-------|-------|-----|
| `--color-primary` | `#064E3B` | Fondos, barras, elementos de confianza |
| `--color-secondary` | `#D6A84F` | Acentos, iconos, estados destacados, bordes |

> ⚠️ El dorado `#D6A84F` **NO se usa como texto sobre fondo claro** (falla WCAG AA).
> Se genera una escala 50–900 de ambos colores en `apps/desktop/src/styles/tokens.css`.

Logos: `assets/branding/AI_KeyVault_Color.png` y `assets/branding/AI_KeyVault_MonoColor.png`

---

## 5. Arquitectura en resumen

```
ai_keyvault/                    ← repo principal
  apps/desktop/                 ← etapa 2 🟡 app Tauri 2 (Rust + TypeScript)
  crates/
    core/                       ← bóveda, cifrado, sincronización (Rust) ✅
    poller/                     ← poller de cuota multi-dispositivo ✅
    alerter/                    ← detector umbrales + notificador ✅
  providers/
    agy/                        ← adaptador Antigravity (Credential Manager) ✅
    codex/                      ← adaptador Codex (auth.json + SQLite) ✅
  server/                       ← API del VPS (Rust/axum + SQLite) ✅
  docs/
    research/                   ← investigación con evidencia verificada ✅
    adr/                        ← Architecture Decision Records ✅
    desktop-app-insumos.md      ← spec de la app de escritorio ✅
    operaciones.md              ← VPS, deploy, BD, variables ✅
  assets/branding/              ← logos e íconos
  .github/workflows/            ← CI, escaneo de secretos, deploy

ai_keyvault_data/               ← repo secundario (solo snapshots cifrados)
```

**Stack:** Tauri 2 (Rust + TypeScript/React). Ver `docs/adr/ADR-001-stack.md`.

---

## 6. Principios de cifrado

- **KDF:** Argon2id. Parámetros en `docs/adr/ADR-002-cifrado.md`.
- **Cifrado:** XChaCha20-Poly1305 (autenticado). Nonce por registro. AAD con id+versión.
- **Vault Key aleatoria** envuelta con clave maestra Y con clave de respaldo.
- **Clave de autenticación** (servidor) separada de **clave de cifrado** (nunca sale del cliente).
- **DPAPI** de Windows como capa adicional de caché local (ver ADR-002).

---

## 7. Estado de fases

| Fase | Estado | Contenido |
|------|--------|-----------|
| **0** | ✅ | Investigación, ADRs, cooldown-logger |
| **1** | ✅ | Núcleo seguro: bóveda y cifrado |
| **2** | ✅ | Servidor VPS: API, sync, deploy |
| **3** | ✅ | Adaptadores agy + Codex |
| **4** | ✅ | Cuota y planes con lease anti-duplicado |
| **5** | ✅ | Alertas (Windows toast + ntfy.sh) |
| **6** | ✅ | Chats agy + Codex (metadatos) |
| **7** | ✅ | Estadísticas: burn rate, heatmap, comparativa, CSV |
| **8** | ✅ | Ping mínimo (manual + automático) |
| **A** | ✅ | Codex chats: `state_5.sqlite` tabla `threads` |
| **C** | ✅ | Cooldown H1 confirmada con datos reales |
| **D** | ✅ | Alert log en servidor (`/api/alerts`) |
| **E** | ✅ | Switch log en servidor (`/api/switch`) |
| **F** | 📄 | Codex token renewal: procedimiento manual |
| **9** | ⏸ | Etapa 3 — proveedores adicionales |
| **10–17** | 🟡 | Etapa 2 — App Tauri 2 v0.2.4: código escrito, CI verde, instalable. Flujo de datos sin verificar/roto |
| **18** | ⬜ | Etapa 2 — Reconexión del flujo de datos (diagnóstico + reparación). **Siguiente** |
| **19** | ⬜ | Sesión de usuario y primer arranque (D-12) |
| **20** | ⬜ | Credenciales de cuenta y portapapeles (D-07) |
| **21–28** | ⬜ | Funciones posteriores en el orden fijado en `PLAN2.md` (D-11) |

**Leyenda:** ✅ verificado en uso real · 🟡 código escrito sin verificar · ⬜ pendiente · ⏸ pospuesto · 📄 solo documentado. Las letras A–F son etiquetas históricas; citar las fases extra **por nombre**.

**Regla:** cada fase termina con tests pasando, CI en verde y escaneo de secretos limpio.
**Regla:** `PLAN.md` se actualiza al terminar cada fase.

---

## 8. Convenciones de código

### Rust
- Edition 2021. `rustfmt` + `clippy -D warnings`. Sin `unwrap()` en código de producción.
- Errores tipados con `thiserror`. `zeroize` para borrar claves de memoria.
- Dependencias mínimas; auditar con `cargo audit` en CI.

### TypeScript (frontend Tauri — segunda etapa)
- Strict mode. ESLint + Prettier.
- Sin `any`. Tipos explícitos en la API con el backend Rust.
- i18n: todos los textos en `apps/desktop/src/i18n/es.json`.

### Tests
- Rust: `#[cfg(test)]` en cada módulo + tests de integración en `tests/`.
- TS: Vitest para unit, Playwright para e2e.
- **Fixtures siempre con datos falsos** (ver 2.2).

---

## 9. CI obligatorio (en cada push a main)

1. `cargo fmt --check`
2. `cargo clippy -D warnings`
3. `cargo test`
4. `gitleaks detect` — falla el pipeline si encuentra secretos
5. `cross build --target x86_64-unknown-linux-gnu` → deploy al VPS
6. `tauri-build` (Windows) — compila la app de escritorio y sube el instalador NSIS (`.exe`) como artifact

---

## 10. Reglas específicas de la app de escritorio (Fase 10+)

- **`apps/desktop` usa su propio workspace Cargo** — compilar con `pnpm tauri dev` o `pnpm tauri build` desde `apps/desktop/`, **nunca** con `cargo build` desde la raíz.
- **`apps/desktop/src-tauri/rust-toolchain.toml`** fuerza `stable-x86_64-pc-windows-msvc`. El toolchain GNU sigue siendo el default del sistema. **No cambiar el default a MSVC**.
- **Todo HTTP al VPS va por comandos Rust** (`#[tauri::command]` con `reqwest`). **Sin `fetch()` externo en React**. La CSP en `tauri.conf.json` lo bloquea activamente.
- **El sidecar** (`ai-keyvault-poller`) se empaqueta en el instalador. Nombre en disco: `ai-keyvault-poller-x86_64-pc-windows-msvc.exe`. En `tauri.conf.json` sin sufijo.
- **El autobloqueo** solo bloquea la UI (hace `zeroize` del `VaultKey`). El poller sigue corriendo — lee credenciales del SO, no de la bóveda.
- **IPC poller ↔ UI**: solo expone `poller_status`, `poller_pause`, `poller_resume`. Nunca credenciales descifradas.
- **DPAPI** = opt-in, desactivado por defecto, con advertencia explícita.
- **cmake** no está instalado en la PC de desarrollo. `providers/codex` usa `rusqlite bundled` y **requiere cmake**: no incluirlo como dependencia del desktop hasta instalarlo. `providers/agy` **no** requiere cmake (solo `windows_sys`, verificado en F17).
- **Cruce de datos:** la app y el poller usan la **misma llave de cuenta** que el VPS (`PLAN2.md` D-02: UUID opaco de la bóveda). Nunca cruzar por alias.

---

## 11. VPS y deploy

- **IP:** `198.211.97.243` **Puerto:** `3010` (IP directa, sin Caddy/TLS por ahora)
- **Ruta:** `/opt/ai-keyvault/`
- **Servicio:** `systemd ai-keyvault`
- **Usuario deploy:** `ai-keyvault-deploy` con SSH restringido
- **BD:** `/opt/ai-keyvault/data/keyvault.db` (SQLite)
- **Secrets:** en GitHub Actions Secrets. Ninguno en el repo.

**Reglas de trabajo con el VPS:**
- Usar MCP SSH (`run-command`) para comandos en el VPS — NO `sftp-upload` ni `privileged-command`
- El sandbox no tiene red — `cargo check/update/run` los corre el usuario
- **Nunca hacer commit manualmente** — el usuario corre `.\gitpush.ps1 "mensaje"`

---

## 12. Documentación de evidencia

Toda afirmación sobre APIs, endpoints o archivos locales no documentados oficialmente **debe tener evidencia** en `docs/research/`:

```
- Fecha de verificación
- Comando ejecutado (sin secretos)
- Salida del comando (sin secretos)
- Versión del programa
- Marcado como [VERIFICADO] o [SUPUESTO]
```

---

## 13. Hallazgos clave verificados (no investigar de nuevo)

| Hallazgo | Verificado |
|----------|-----------|
| `agy --print /usage` → 3 líneas TSV con datetime local CST | ✅ 2026-11-25 |
| `AgyCredentialManager::read()` es función asociada (no método) | ✅ |
| Pools agy reales: `gemini-weekly`, `claude-gpt-weekly`, `claude-gpt-5h` | ✅ |
| Codex: `~/.codex/state_5.sqlite` tabla `threads`, campo `messages` JSON | ✅ |
| Hipótesis H1 confirmada: reloj empieza con primer uso | ✅ 2026-11-25 |
| `@` en URLs de curl Windows: usar `%40` encoding | ✅ |
| `winrt-notification 0.5` → `STATUS_ACCESS_VIOLATION` en tests Windows | ✅ |
| Toast de Windows: usar PowerShell `NotifyIcon` (no winrt) | ✅ |
| `&&` no válido en PowerShell (usar `;` o `-eq 0`) | ✅ |
| `providers/agy` no requiere cmake (solo `windows_sys`); `providers/codex` sí | ✅ F17 |
| El VPS devuelve booleanos SQLite como 0/1 → usar `de_bool_or_int` | ✅ F17 |
| `account_id` en el VPS = correo leído del Credential Manager (`id_token_email`) | ✅ F4 (ver `PLAN2.md` D-02) |
| agy: cuota solo vía `agy --print /usage` (endpoints cloudcode 403; Codex `/wham` 404) → el poller solo ve la cuenta **activa** del SO | ✅ / deducción en D-01 |
| PATH necesario: `C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin` | ✅ |

---

## 14. Checklist antes de hacer commit

- [ ] ¿Hay algún secreto real en los archivos modificados?
- [ ] ¿Los fixtures de tests usan datos falsos?
- [ ] ¿`cargo fmt --all` pasa sin diferencias?
- [ ] ¿`cargo clippy --all-targets -- -D warnings` pasa sin errores?
- [ ] ¿Se actualizó `PLAN.md` si terminó una fase?
- [ ] ¿El commit message sigue el formato convencional en español?
- [ ] ¿Se actualizó `docs/operaciones.md` si hubo cambios en el VPS?
- [ ] ¿Se eliminó o reescribió algo en `server/`, `crates/` o `providers/`? Si sí: ¿está listado y aprobado en `PLAN2.md`?
- [ ] ¿Cambió una estructura VPS ↔ Rust ↔ TS? Si sí: ¿se actualizaron contrato y fixtures de test?
- [ ] ¿Algún error se está silenciando (`.catch(() => {})`, `let _ =`)?
- [ ] ¿Hay algo `[PROPUESTA]` implementado sin aprobación?

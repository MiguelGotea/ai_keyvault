# CLAUDE.md — Reglas de trabajo para AI KeyVault

> Este archivo es leído automáticamente por el agente de desarrollo al abrir el proyecto.
> **No eliminar. No modificar sin actualizar la versión.**

Versión: 2.0.0 | Fecha: 2026-11-25

---

## 1. Qué es este proyecto

**AI KeyVault** es una bóveda + tablero personal para administrar cuentas de agentes de IA de programación (Antigravity/agy, Codex, y en segunda etapa Claude, Gemini, Grok y otros). Es una **herramienta personal** del autor para sus propias cuentas en sus propias PCs. No tiene fines comerciales ni de distribución masiva.

**Estado al 2026-11-25:** Primera etapa **completada**. El backend (servidor VPS + poller + alerter + providers) está 100% operativo. La segunda etapa es la **app de escritorio Tauri 2**.

**Alcance ético (obligatorio):**
- Uso exclusivo con cuentas propias en PCs propias.
- No evade bloqueos ni comparte cuentas con terceros.
- La rotación de cuentas y el ping son decisiones del usuario, configurables y con registro completo.
- La herramienta no roba credenciales, tokens ni claves de nadie.

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
  apps/desktop/                 ← [SEGUNDA ETAPA] app Tauri 2 (Rust + TypeScript)
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
    desktop-app-insumos.md      ← spec completo para segunda etapa ✅
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
| **9** | ⏸ | Segunda etapa — proveedores adicionales |
| **Desktop** | 🔄 | Segunda etapa — Fase 10 scaffold completo, pendiente de primer build |

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
6. `tauri-build` (Windows) — compila la app de escritorio y sube el `.msi` como artifact

---

## 10. Reglas específicas de la app de escritorio (Fase 10+)

- **`apps/desktop` usa su propio workspace Cargo** — compilar con `pnpm tauri dev` o `pnpm tauri build` desde `apps/desktop/`, **nunca** con `cargo build` desde la raíz.
- **`apps/desktop/src-tauri/rust-toolchain.toml`** fuerza `stable-x86_64-pc-windows-msvc`. El toolchain GNU sigue siendo el default del sistema. **No cambiar el default a MSVC**.
- **Todo HTTP al VPS va por comandos Rust** (`#[tauri::command]` con `reqwest`). **Sin `fetch()` externo en React**. La CSP en `tauri.conf.json` lo bloquea activamente.
- **El sidecar** (`ai-keyvault-poller`) se empaqueta en el instalador. Nombre en disco: `ai-keyvault-poller-x86_64-pc-windows-msvc.exe`. En `tauri.conf.json` sin sufijo.
- **El autobloqueo** solo bloquea la UI (hace `zeroize` del `VaultKey`). El poller sigue corriendo — lee credenciales del SO, no de la bóveda.
- **IPC poller ↔ UI**: solo expone `poller_status`, `poller_pause`, `poller_resume`. Nunca credenciales descifradas.
- **DPAPI** = opt-in, desactivado por defecto, con advertencia explícita.
- **cmake** no está instalado en la PC de desarrollo. `rusqlite bundled` (en `providers/codex`) requiere cmake — NO incluir como dependencia directa del desktop hasta que cmake esté instalado. El desktop se comunica con el backend VPS via HTTP.

---

## 10. VPS y deploy

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

## 11. Documentación de evidencia

Toda afirmación sobre APIs, endpoints o archivos locales no documentados oficialmente **debe tener evidencia** en `docs/research/`:

```
- Fecha de verificación
- Comando ejecutado (sin secretos)
- Salida del comando (sin secretos)
- Versión del programa
- Marcado como [VERIFICADO] o [SUPUESTO]
```

---

## 12. Hallazgos clave verificados (no investigar de nuevo)

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
| PATH necesario: `C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin` | ✅ |

---

## 13. Checklist antes de hacer commit

- [ ] ¿Hay algún secreto real en los archivos modificados?
- [ ] ¿Los fixtures de tests usan datos falsos?
- [ ] ¿`cargo fmt --all` pasa sin diferencias?
- [ ] ¿`cargo clippy --all-targets -- -D warnings` pasa sin errores?
- [ ] ¿Se actualizó `PLAN.md` si terminó una fase?
- [ ] ¿El commit message sigue el formato convencional en español?
- [ ] ¿Se actualizó `docs/operaciones.md` si hubo cambios en el VPS?

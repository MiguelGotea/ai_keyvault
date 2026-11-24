# CLAUDE.md — Reglas de trabajo para AI KeyVault

> Este archivo es leído automáticamente por Claude Code al abrir el proyecto.
> **No eliminar. No modificar sin actualizar la versión.**

Versión: 1.0.0 | Fecha: 2026-11-22

---


## 1. Qué es este proyecto

**AI KeyVault** es una bóveda + tablero personal para administrar cuentas de agentes de IA de programación (Antigravity/agy, Codex, Claude, Gemini, OpenCode, Grok y otros). Es una **herramienta personal** del autor para sus propias cuentas en sus propias PCs. No tiene fines comerciales ni de distribución masiva.

**Alcance ético (obligatorio en README y aquí):**
- Uso exclusivo con cuentas propias en PCs propias.
- No evade bloqueos ni comparte cuentas con terceros.
- La rotación de cuentas y el ping son decisiones del usuario, configurables y con registro completo.
- La herramienta no roba credenciales, tokens ni claves de nadie.

---

## 2. Reglas de seguridad (no negociables)

### 2.1 Secretos — NUNCA en el repositorio

Los siguientes elementos **jamás** deben aparecer en ningún archivo del repo `ai_keyvault`:

| Categoría | Ejemplos |
|-----------|---------|
| Contraseñas reales | passwords de cuentas del autor |
| Correos reales | emails de cuentas del autor |
| Tokens de acceso | access tokens, refresh tokens, id_tokens |
| Blobs de sesión | auth.json de Codex, blobs de Credential Manager de agy |
| Claves de cifrado | clave maestra, clave de respaldo, vault key |
| Llaves SSH | llaves privadas de cualquier tipo |
| Archivos .env reales | cualquier .env con datos reales |
| Tópico de ntfy | el tópico aleatorio de notificaciones |
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

Solo recibe snapshots **ya cifrados** del lado del cliente. El servidor no puede descifrar nada. Este repo no es una bóveda — es solo un respaldo cifrado.

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
  apps/desktop/                 ← app Tauri 2 (Rust + TypeScript)
  crates/core/                  ← bóveda, cifrado, sincronización (Rust)
  providers/{agy,codex,...}/    ← adaptadores de proveedor (Rust)
  server/                       ← API del VPS (Rust/axum + SQLite)
  agent/                        ← agente local Windows (servicio background)
  docs/{research,adr}/          ← investigación y decisiones técnicas
  assets/branding/              ← logos e íconos
  .scripts/                     ← herramientas de desarrollo (sin secretos)
  .github/workflows/            ← CI, escaneo de secretos, deploy, respaldo

ai_keyvault_data/               ← repo secundario (solo snapshots cifrados)
```

**Stack decidido:** Tauri 2 (Rust + TypeScript/React). Ver `docs/adr/ADR-001-stack.md`.

---

## 6. Principios de cifrado

- **KDF:** Argon2id. Parámetros en `docs/adr/ADR-002-cifrado.md`.
- **Cifrado:** XChaCha20-Poly1305 (autenticado). Nonce por registro. AAD con id+versión.
- **Vault Key aleatoria** envuelta con clave maestra Y con clave de respaldo.
- **Clave de autenticación** (servidor) separada de **clave de cifrado** (nunca sale del cliente).
- **DPAPI** de Windows como capa adicional de caché local (ver ADR-002).

---

## 7. Orden de trabajo (fases)

| Fase | Contenido |
|------|-----------|
| **0** | Investigación, ADRs, `cooldown-logger` ← **aquí estamos** |
| **1** | Núcleo seguro: bóveda, cifrado, autobloqueo |
| **2** | Servidor VPS: API, sync, deploy |
| **3** | Adaptadores agy + Codex |
| **4** | Cuota y planes |
| **5** | Alertas (Windows + ntfy.sh) |
| **6** | Agente de chats |
| **7** | Estadísticas |
| **8** | Ping mínimo |
| **9** | Proveedores adicionales |

**Regla:** cada fase termina con tests pasando, CI en verde y escaneo de secretos limpio.
**Regla:** `PLAN.md` se actualiza al terminar cada fase.

---

## 8. Convenciones de código

### Rust
- Edition 2021. `rustfmt` + `clippy -D warnings`.
- Errores tipados con `thiserror`. Sin `unwrap()` en código de producción.
- `zeroize` para borrar claves de memoria al desbloquear/cerrar.
- Dependencias mínimas; auditar con `cargo audit` en CI.

### TypeScript (frontend Tauri)
- Strict mode. ESLint + Prettier.
- Sin `any`. Tipos explícitos en la API con el backend Rust.
- i18n: todos los textos en `apps/desktop/src/i18n/es.json`.

### Tests
- Rust: `#[cfg(test)]` en cada módulo + tests de integración en `tests/`.
- TS: Vitest para unit, Playwright para e2e.
- **Fixtures siempre con datos falsos** (ver 2.2).

---

## 9. CI obligatorio (en cada PR y push a main)

1. `cargo fmt --check`
2. `cargo clippy -D warnings`
3. `cargo test`
4. `cargo audit`
5. `gitleaks detect` — falla el pipeline si encuentra secretos
6. `eslint` + `tsc --noEmit`
7. `vitest run`
8. Build de Tauri (en release solo desde `main`)

---

## 10. VPS y deploy

- VPS: DigitalOcean (198.211.97.243). Usuario no root. Servicio `systemd` con sandboxing.
- Deploy: GitHub Actions con llave SSH restringida. Ver `.github/workflows/deploy-server.yml`.
- El servidor vive en `/opt/ai-keyvault/` — aislado de los otros proyectos (`/opt/gmb-worker/`, `/root/retails-nicaragua/`).
- **Caddy** maneja TLS. Puerto interno del servidor: 3010 (no expuesto directo).
- Secrets en GitHub Actions Secrets. Ninguno en el repo.

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

## 12. Checklist antes de hacer commit

- [ ] ¿Hay algún secreto real en los archivos modificados?
- [ ] ¿Los fixtures de tests usan datos falsos?
- [ ] ¿`gitleaks detect` pasa sin errores?
- [ ] ¿`cargo test` y `vitest run` pasan?
- [ ] ¿Se actualizó `PLAN.md` si terminó una fase?
- [ ] ¿El commit message sigue el formato convencional?

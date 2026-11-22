# ADR-001 — Elección de Stack (App de Escritorio)

**Fecha:** 2026-11-22
**Estado:** Aceptado
**Autor:** Arquitecto del proyecto
**Revisado por:** N/A (decisión unilateral del desarrollador)

---

## Contexto

AI KeyVault es una app de escritorio para Windows que administra credenciales y tokens de múltiples agentes de IA. Necesitamos elegir el stack tecnológico para:

1. La **app de escritorio** (interfaz + núcleo).
2. El **agente local** (proceso en segundo plano).
3. El **servidor del VPS** (API).

### Restricciones
- Solo Windows (todas las PCs del usuario son Windows 10/11).
- Acceso a Windows Credential Manager para agy.
- Escritura atómica de archivos (auth.json de Codex).
- Arranque rápido (< 2s), ejecutable único o instalador pequeño.
- Poco consumo de RAM en segundo plano.
- Sin telemetría hacia terceros.

---

## Opciones evaluadas

### A. Tauri 2 (Rust + TypeScript)

| Criterio | Puntuación (1-5) | Notas |
|----------|------------------|-------|
| Arranque rápido | 5 | ~200ms vs ~3s de Electron |
| Consumo RAM | 5 | ~30-50MB vs ~150MB de Electron |
| Tamaño instalador | 5 | ~10MB vs ~100MB de Electron |
| Acceso a Windows APIs | 4 | Via Rust `windows-sys` crate — robusto pero requiere conocimiento de Rust |
| Cifrado robusto | 5 | Rust tiene los mejores crates de criptografía (RustCrypto) |
| Zeroize en memoria | 5 | Crate `zeroize` — crítico para borrar claves |
| UI moderna | 4 | Frontend web completo (React, animaciones, Ctrl+K) |
| Curva de aprendizaje | 3 | Rust + IPC Tauri es más complejo que JS puro |
| **TOTAL** | **36/40** | |

### B. .NET 9 (WinUI 3 / WPF)

| Criterio | Puntuación (1-5) | Notas |
|----------|------------------|-------|
| Arranque rápido | 3 | ~1-2s con AOT; sin AOT ~3-4s |
| Consumo RAM | 3 | ~80-120MB |
| Tamaño instalador | 3 | ~40-80MB con self-contained |
| Acceso a Windows APIs | 5 | Nativo — DPAPI, Credential Manager, COM |
| Cifrado robusto | 4 | `System.Security.Cryptography` es sólido |
| Zeroize en memoria | 3 | Manual con `fixed` y `RtlSecureZeroMemory` — más frágil |
| UI moderna | 3 | WinUI 3 es moderno pero menos flexible que web |
| Curva de aprendizaje | 4 | C# es familiar para muchos devs Windows |
| **TOTAL** | **28/40** | |

### C. Electron (Node.js + Chromium)

| Criterio | Puntuación (1-5) | Notas |
|----------|------------------|-------|
| Arranque rápido | 2 | ~3-5s por cargar Chromium |
| Consumo RAM | 1 | ~150-300MB |
| Tamaño instalador | 1 | ~100-200MB |
| Acceso a Windows APIs | 2 | Via node-addon-api o powershell spawned — frágil |
| Cifrado robusto | 2 | Node.js crypto es básico; para Argon2 se necesita addon nativo |
| Zeroize en memoria | 1 | JavaScript no garantiza borrado seguro de memoria |
| UI moderna | 5 | Frontend web completo, ecosistema enorme |
| Curva de aprendizaje | 5 | TypeScript puro, familiar para casi todos |
| **TOTAL** | **19/40** | Descartado por seguridad y rendimiento |

---

## Decisión

**Elegido: Tauri 2 (Rust + TypeScript/React)**

### Justificación

1. **Seguridad primero:** el núcleo en Rust permite usar `zeroize` para borrar claves de memoria, `RustCrypto` para Argon2id y XChaCha20-Poly1305 con auditorías de seguridad, y la API `windows-sys` para Credential Manager sin capas intermedias.

2. **Rendimiento:** arranque < 500ms, RAM < 60MB en reposo. La app se sentirá "instantánea" como exige el prompt.

3. **Tamaño:** instalador ~10-15MB. Distribución por GitHub Releases sin fricciones.

4. **UI completa:** el frontend web en React permite Ctrl+K (paleta de comandos), animaciones, modo claro/oscuro, y toda la libertad del ecosistema web.

5. **Monorepo natural:** los `crates/` de Rust se comparten entre la app de escritorio (Tauri), el agente local y el servidor — sin duplicar lógica de cifrado.

### Riesgos y mitigaciones

| Riesgo | Mitigación |
|--------|------------|
| Curva de aprendizaje Rust | Usar un módulo Rust mínimo (solo bóveda, cifrado, Credential Manager) y la mayor parte de la lógica de UI en TypeScript |
| IPC Tauri compleja | Definir un contrato de comandos Tauri claro en Fase 1; usar tipos compartidos via `serde` + `ts-rs` |
| Firma de código en Windows | SmartScreen mostrará advertencia sin firma. Documentar en README. Se puede firmar con un certificado de código en el futuro. |

---

## Stack completo decidido

| Componente | Tecnología | Justificación |
|------------|-----------|---------------|
| App escritorio (UI) | Tauri 2 + React 18 + TypeScript | Ver arriba |
| Núcleo bóveda y cifrado | Rust (crate `crates/core`) | Seguridad, zeroize, RustCrypto |
| Adaptadores de proveedor | Rust (crates en `providers/`) | Acceso a Credential Manager y filesystem |
| Agente local | Rust (proceso standalone) | Mismo runtime, sin dependencias extra |
| Servidor VPS | Rust + axum + SQLite (sqlx) | Mismo lenguaje, binario pequeño, sin runtime externo |
| Base de datos servidor | SQLite (via sqlx) | Simple, sin servidor de BD separado, suficiente para ~30 cuentas |
| Reverse proxy | Caddy | TLS automático, configuración sencilla |
| CI/CD | GitHub Actions | Mismo patrón que proyectos existentes |
| i18n | JSON planos en `apps/desktop/src/i18n/` | Simple, sin librería pesada |

---

## Consecuencias

- El binario del servidor es un ejecutable Rust compilado — no necesita Node/Python en el VPS.
- El agente local también es un ejecutable Rust — se instala junto con la app de escritorio.
- El instalador de Windows se genera con `tauri build` que usa WiX o NSIS.
- Para tests e2e de UI, usar Playwright con el driver de WebView2.

---

## Alternativas rechazadas

- **Python para el agente local:** descartado por consumo de RAM y fricción de distribución (requiere Python instalado o bundled).
- **Go para el servidor:** Rust se prefiere por reutilizar el mismo ecosistema de crates de criptografía y por el monorepo.
- **PostgreSQL:** SQLite es suficiente para el volumen de datos (~30 cuentas, lecturas cada 10min). Cadena de conexión simple, sin servicio adicional.

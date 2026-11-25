# Third-Party Notices — AI KeyVault

Este archivo registra todas las dependencias y proyectos de referencia de AI KeyVault,
sus licencias y los avisos de copyright correspondientes.

**Regla:** Si se copia o adapta código de cualquier proyecto, se debe:
1. Conservar el aviso de copyright original en el código.
2. Registrar el proyecto aquí con licencia, URL y descripción del uso.
3. Verificar que la licencia sea compatible con el uso personal de este proyecto.

*Última actualización: 2026-11-25*

---

## Proyectos de referencia (código consultado / adaptado)

### antigravity-swap
- **Autor:** hrmasss
- **Licencia:** MIT
- **URL:** https://github.com/hrmasss/antigravity-swap
- **Uso:** Referencia para el adaptador `agy`: lectura/escritura de credenciales en Windows Credential Manager, verificación con `agy models`.
- **Estado:** Referencia de diseño — código no incorporado directamente.

### codex-switcher
- **Autor:** Lampese
- **Licencia:** MIT
- **URL:** https://github.com/Lampese/codex-switcher
- **Uso:** Referencia para el adaptador `codex`: cambio atómico de `auth.json`.
- **Estado:** Referencia de diseño — código no incorporado directamente.

### codex-cli-account-switcher
- **Autor:** bashar94
- **Licencia:** MIT
- **URL:** https://github.com/bashar94/codex-cli-account-switcher
- **Uso:** Referencia adicional para el cambio de cuenta de Codex.
- **Estado:** Referencia de diseño — código no incorporado directamente.

### claude-swap
- **Autor:** realiti4
- **Licencia:** Por verificar antes de incorporar (segunda etapa)
- **URL:** https://github.com/realiti4/claude-swap
- **Uso:** Referencia para el adaptador `claude` (segunda etapa).
- **Estado:** Pendiente para segunda etapa.

---

## Dependencias de Rust (versiones al 2026-11-25)

Generado con `cargo metadata`. Licencias verificadas.

### Servidor (`server/`)

| Crate | Versión | Licencia | Uso |
|-------|---------|----------|-----|
| `axum` | 0.7 | MIT | Framework HTTP |
| `tokio` | 1.40 | MIT | Runtime async |
| `tower-http` | 0.5 | MIT | Middlewares HTTP (CORS, trace) |
| `sqlx` | 0.8 | MIT/Apache-2.0 | SQLite async |
| `serde` | 1.0 | MIT/Apache-2.0 | Serialización |
| `serde_json` | 1.0 | MIT/Apache-2.0 | JSON |
| `uuid` | 1.10 | MIT/Apache-2.0 | UUIDs v4 |
| `chrono` | 0.4 | MIT/Apache-2.0 | Fechas y tiempos |
| `sha2` | 0.10 | MIT/Apache-2.0 | Hash SHA-256 |
| `hex` | 0.4 | MIT/Apache-2.0 | Encoding hexadecimal |
| `thiserror` | 1.0 | MIT/Apache-2.0 | Tipos de error |
| `dotenvy` | 0.15 | MIT/Apache-2.0 | Variables de entorno |
| `tracing` | 0.1 | MIT | Logs estructurados |
| `tracing-subscriber` | 0.3 | MIT | Salida de logs |

### Núcleo (`crates/core/`)

| Crate | Versión | Licencia | Uso |
|-------|---------|----------|-----|
| `argon2` | 0.5 | MIT/Apache-2.0 | KDF Argon2id |
| `chacha20poly1305` | 0.10 | MIT/Apache-2.0 | Cifrado XChaCha20-Poly1305 |
| `zeroize` | 1.8 | MIT/Apache-2.0 | Borrado seguro de claves |
| `rand` | 0.8 | MIT/Apache-2.0 | Generación de nonces y salts |
| `base64` | 0.22 | MIT/Apache-2.0 | Encoding Base64 |
| `ed25519-dalek` | 2.1 | BSD-3-Clause | Firma Ed25519 (auth de dispositivos) |

### Poller (`crates/poller/`)

| Crate | Versión | Licencia | Uso |
|-------|---------|----------|-----|
| `reqwest` | 0.12 | MIT/Apache-2.0 | HTTP client async |
| `tokio` | 1.40 | MIT | Runtime async |
| `chrono` | 0.4 | MIT/Apache-2.0 | Fechas y tiempos |

### Alerter (`crates/alerter/`)

| Crate | Versión | Licencia | Uso |
|-------|---------|----------|-----|
| `reqwest` | 0.12 | MIT/Apache-2.0 | HTTP a ntfy.sh (bloqueante) |
| `dirs-next` | 2.0 | MIT/Apache-2.0 | Directorio home del usuario |

### Provider agy (`providers/agy/`)

| Crate | Versión | Licencia | Uso |
|-------|---------|----------|-----|
| `regex` | 1.11 | MIT/Apache-2.0 | Parseo de salida `agy /usage` |
| `dirs-next` | 2.0 | MIT/Apache-2.0 | Ruta a brain/ de agy |
| `windows-sys` | 0.59 | MIT/Apache-2.0 | Win32 Credential Manager API |

### Provider Codex (`providers/codex/`)

| Crate | Versión | Licencia | Uso |
|-------|---------|----------|-----|
| `rusqlite` | 0.32 | MIT | Lectura de `state_5.sqlite` de Codex |

---

## Dependencias de Node/TypeScript (segunda etapa — pendientes)

> Esta sección se actualizará al configurar el frontend Tauri 2 en `apps/desktop/`.
> Usar `npm licenses` o `license-checker` para generar la lista automáticamente.

| Paquete | Versión | Licencia | Uso previsto |
|---------|---------|----------|--------------|
| `react` | TBD | MIT | UI del escritorio |
| `@tauri-apps/api` | TBD | MIT/Apache-2.0 | Bridge Tauri |
| `vitest` | TBD | MIT | Tests unitarios TS |
| `playwright` | TBD | Apache-2.0 | Tests e2e |

---

## Nota sobre `ed25519-dalek`

`ed25519-dalek` usa licencia **BSD-3-Clause**, que es compatible con uso personal y comercial, siempre que se conserve el aviso de copyright. Este aviso debe incluirse en cualquier distribución del binario de AI KeyVault.

```
Copyright (c) 2017-2019 isis agora lovecruft <isis@patternsinthevoid.net>
Copyright (c) 2017-2019 Web 3 Foundation <https://web3.foundation>
All rights reserved.
```

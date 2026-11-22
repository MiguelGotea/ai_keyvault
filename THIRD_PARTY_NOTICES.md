# Third-Party Notices — AI KeyVault

Este archivo registra todas las dependencias y proyectos de referencia de AI KeyVault,
sus licencias y los avisos de copyright correspondientes.

**Regla:** Si se copia o adapta código de cualquier proyecto, se debe:
1. Conservar el aviso de copyright original en el código.
2. Registrar el proyecto aquí con licencia, URL y descripción del uso.
3. Verificar que la licencia sea compatible con el uso personal de este proyecto.

---

## Proyectos de referencia (código consultado / adaptado)

### antigravity-swap
- **Autor:** hrmasss
- **Licencia:** MIT
- **URL:** https://github.com/hrmasss/antigravity-swap
- **Uso:** Referencia para la implementación del adaptador `agy`: lectura/escritura de credenciales en Windows Credential Manager, consulta de cuota via `cloudcode-pa.googleapis.com`, verificación con `agy models`, y estructura de tests con `fake_agy`.
- **Estado:** Referencia de diseño — código aún no incorporado.

### codex-switcher
- **Autor:** Lampese
- **Licencia:** MIT
- **URL:** https://github.com/Lampese/codex-switcher
- **Uso:** Referencia para el adaptador `codex`: cambio atómico de `auth.json`, lectura de cuota y estadísticas.
- **Estado:** Referencia de diseño — código aún no incorporado.

### codex-cli-account-switcher
- **Autor:** bashar94
- **Licencia:** MIT
- **URL:** https://github.com/bashar94/codex-cli-account-switcher
- **Uso:** Referencia adicional para el cambio de cuenta de Codex.
- **Estado:** Referencia de diseño — código aún no incorporado.

### claude-swap
- **Autor:** realiti4
- **Licencia:** Por verificar antes de incorporar
- **URL:** https://github.com/realiti4/claude-swap
- **Uso:** Referencia para el adaptador `claude`.
- **Estado:** Referencia de diseño — licencia pendiente de verificación.

### harness-swap / hswap
- **Autor:** realiti4 (por confirmar)
- **Licencia:** Por verificar antes de incorporar
- **URL:** Por confirmar
- **Uso:** Referencia para adaptadores adicionales.
- **Estado:** Pendiente de localizar y verificar licencia.

---

## Dependencias de Rust (por confirmar al agregar Cargo.toml)

> Esta sección se actualizará al agregar el archivo `Cargo.toml` con las dependencias reales.
> Usar `cargo license` para generar la lista automáticamente.

| Crate | Versión | Licencia | Uso previsto |
|-------|---------|----------|--------------|
| `argon2` | TBD | MIT/Apache-2.0 | KDF Argon2id |
| `chacha20poly1305` | TBD | MIT/Apache-2.0 | Cifrado XChaCha20-Poly1305 |
| `zeroize` | TBD | MIT/Apache-2.0 | Borrado seguro de claves en memoria |
| `axum` | TBD | MIT | Framework HTTP para el servidor |
| `sqlx` | TBD | MIT/Apache-2.0 | SQLite en el servidor |
| `tauri` | TBD | MIT/Apache-2.0 | Framework de escritorio |
| `ed25519-dalek` | TBD | BSD-3 | Autenticación de dispositivos |
| `serde` | TBD | MIT/Apache-2.0 | Serialización |
| `thiserror` | TBD | MIT/Apache-2.0 | Tipos de error |
| `tokio` | TBD | MIT | Runtime async |

---

## Dependencias de Node/TypeScript (por confirmar al agregar package.json)

> Esta sección se actualizará al configurar el frontend Tauri.

| Paquete | Versión | Licencia | Uso previsto |
|---------|---------|----------|--------------|
| `react` | TBD | MIT | UI del escritorio |
| `@tauri-apps/api` | TBD | MIT/Apache-2.0 | Bridge Tauri |
| `vitest` | TBD | MIT | Tests unitarios TS |
| `playwright` | TBD | Apache-2.0 | Tests e2e |

---

*Última actualización: 2026-11-22*

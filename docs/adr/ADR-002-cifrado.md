# ADR-002 — Modelo de Cifrado y Claves

**Fecha:** 2026-11-22
**Estado:** Aceptado
**Autor:** Arquitecto del proyecto

---

## Contexto

AI KeyVault almacena credenciales sensibles (correos, contraseñas, tokens de sesión) que deben estar cifradas en todo momento. El servidor solo recibe texto cifrado. La clave que descifra nunca sale de las PCs del usuario.

### Requisitos
1. Cifrado autenticado (detectar manipulación).
2. KDF con resistencia a ataques de fuerza bruta.
3. Clave de respaldo para recuperación si se olvida la clave maestra.
4. Desbloqueo rápido en cada sesión (sin recalcular KDF completa cada vez).
5. Zeroize al bloquear — las claves se borran de memoria.
6. Separar la clave de autenticación con el servidor de la clave de cifrado.

---

## Decisiones

### KDF: Argon2id

**Parámetros elegidos:**

| Parámetro | Valor | Justificación |
|-----------|-------|---------------|
| Variante | `argon2id` | Resistente a GPU (argon2d) Y a timing sidechannel (argon2i) |
| Memoria | 64 MB | Recomendación OWASP 2024 mínimo para contraseñas de alto valor |
| Iteraciones (t) | 3 | Balancea velocidad y seguridad; ajustar según hardware |
| Paralelismo (p) | 4 | Aprovechar 4 cores (mínimo en las PCs del usuario) |
| Longitud de salida | 64 bytes | 32 bytes para clave de cifrado + 32 bytes para clave de autenticación |
| Salt | 32 bytes aleatorios | Generados con `OsRng` al crear la bóveda |

**Justificación de 64MB:** para una laptop moderna con 16GB RAM, 64MB es un ~0.4% de la RAM total. Es el mínimo para hacer ataques de GPU impracticables. Si la PC del usuario tiene menos RAM, ajustar a 32MB con t=5.

**Referencia:** OWASP Password Storage Cheat Sheet (2024), RFC 9106 (Argon2).

**Crate:** `argon2` del proyecto RustCrypto (auditoría de seguridad pública).

### Cifrado de contenido: XChaCha20-Poly1305

**Justificación sobre AES-256-GCM:**

| Aspecto | XChaCha20-Poly1305 | AES-256-GCM |
|---------|-------------------|-------------|
| Nonce seguro | ✅ 192 bits — puede generarse aleatoriamente sin riesgo de colisión | ⚠️ 96 bits — riesgo de colisión con nonces aleatorios en volúmenes altos |
| Sin instrucciones especiales | ✅ Pura aritmética — igual de rápido en CPUs sin AES-NI | ⚠️ Lento en hardware sin AES-NI |
| Tiempo constante | ✅ Garantizado por diseño | ⚠️ Depende de la implementación |
| Auditorías | ✅ libsodium, RustCrypto | ✅ Ampliamente auditado |

**Nonce:** 24 bytes generados con `OsRng` por cada cifrado (nunca reutilizado).

**AAD (Additional Authenticated Data):** `{registro_id}:{version}:{tipo_campo}` — permite detectar si un blob cifrado se usó en un contexto incorrecto (ataque de cut-and-paste).

**Crate:** `chacha20poly1305` del proyecto RustCrypto.

### Estructura de claves

```
Clave Maestra (ingresada por el usuario)
         │
         ▼
   Argon2id + salt
         │
         ├──► 32 bytes → Clave de Autenticación (auth con el servidor)
         │                  [NUNCA se usa para cifrar datos]
         │
         └──► 32 bytes → Clave de Derivación Maestra (CDM)
                               │
                               ▼
                    XChaCha20-Poly1305 (descifrado)
                               │
                               ▼
                        Vault Key (VK) — 32 bytes aleatorios
                        [cifrada en dos copias]
                               │
                    ┌──────────┴──────────┐
                    ▼                     ▼
            Wrapped con CDM        Wrapped con Clave de Respaldo
            (en BD servidor)       (en BD servidor)
                    │
                    ▼
         VK cifra todos los registros
         (cada campo individualmente)
```

### Clave de respaldo

- Generada con `OsRng` al crear la bóveda: 32 bytes aleatorios.
- Mostrada **una sola vez** al usuario en formato legible:
  ```
  AKVR-1234-ABCD-5678-EFGH-9012-IJKL-3456
  ```
  (8 grupos de 4 caracteres Base32, estilo clave de recuperación de Windows/BitLocker).
- Permite descifrar el wrapped VK alternativo y cambiar la clave maestra.
- **No se guarda en texto claro en ningún servidor** — solo el wrapped VK con la clave de respaldo.

### Separación auth/cifrado

Siguiendo el modelo de Bitwarden:
- La **Clave de Autenticación** (derivada de Argon2id, 32 bytes) se usa para autenticarse con el servidor vía HMAC o como input a un token JWT. **Nunca cifra datos**.
- La **Clave de Derivación Maestra** (los otros 32 bytes de Argon2id) descifra la Vault Key. **Nunca sale del cliente**.
- El servidor solo ve el hash de la clave de autenticación — nunca la clave de autenticación directa.

### DPAPI — caché de sesión en Windows

- Al desbloquear la bóveda, la Vault Key se cifra con DPAPI (ligada al usuario de Windows actual) y se guarda en caché en memoria o en un archivo temporal.
- En la próxima sesión de Windows (si no se cerró la app), se puede desbloquear con DPAPI sin pedir la clave maestra.
- Al autobloqueo o cierre, el blob DPAPI se borra y la Vault Key se zeroiza en memoria.

**Crate:** `windows-sys` para llamar a `CryptProtectData` / `CryptUnprotectData`.

---

## Esquema de cifrado por registro

```rust
struct CiphertextRecord {
    id: Uuid,                  // ID opaco (el servidor lo ve)
    nonce: [u8; 24],           // nonce XChaCha20
    ciphertext: Vec<u8>,       // campo cifrado
    aad: String,               // "record_id:version:field_type" (el servidor lo ve)
    updated_at: DateTime<Utc>, // el servidor lo ve
}
```

Cada campo sensible (email, password, token, nota) se cifra **por separado**. El servidor nunca puede correlacionar campos de un mismo registro salvo por `record_id`.

---

## Zeroize

Usando el crate `zeroize`:

```rust
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
struct VaultKey([u8; 32]);

#[derive(ZeroizeOnDrop)]
struct MasterKey([u8; 64]); // 32 auth + 32 derivación
```

Al autobloqueo, el `Drop` automático borra los bytes de memoria.

---

## Consecuencias

- La Vault Key es un **punto único de acceso** — si se pierde, solo la clave de respaldo puede recuperarla.
- El servidor es completamente ciego al contenido — puede servir datos sin entender nada.
- El tamaño de los ciphertexts es ligeramente mayor que los plaintexts (+16 bytes Poly1305 tag + 24 bytes nonce).
- Para ~30 cuentas con ~10 campos cada una, el overhead es insignificante.

---

## Alternativas rechazadas

- **AES-256-GCM:** rechazado por el riesgo de colisión de nonce con nonces aleatorios de 96 bits. Con nonces de 192 bits (XChaCha20), el riesgo es despreciable.
- **bcrypt/scrypt para KDF:** rechazados. Argon2id es el estándar actual (RFC 9106) con mejor resistencia a ataques de hardware especializado.
- **PBKDF2:** muy rápido en GPU. Descartado para claves de alto valor.
- **Libsodium via bindings:** RustCrypto es preferido por auditabilidad sin FFI overhead y sin dependencia de una librería C.

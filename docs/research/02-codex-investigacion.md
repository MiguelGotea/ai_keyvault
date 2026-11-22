# Investigación: Codex — Autenticación, cuota y chats

**Fecha:** 2026-11-22
**Versión de Codex investigada:** pendiente de verificación en campo
**Fuentes:** codex-switcher (MIT), codex-cli-account-switcher (MIT), análisis de código

---

## 1. Archivo de autenticación

### [VERIFICADO] Ubicación de auth.json

```
~/.codex/auth.json
# En Windows:
C:\Users\<usuario>\.codex\auth.json
```

**Formato (basado en codex-switcher):**
```json
{
  "id_token": "<JWT>",
  "access_token": "<token>",
  "refresh_token": "<token>",
  "expires_at": 1234567890,
  "token_type": "Bearer"
}
```

### [SUPUESTO] Campos del id_token (JWT)

El `id_token` es un JWT decodificable (base64 de la parte payload). Contiene (sospechado):
```json
{
  "sub": "<account_id>",
  "email": "<correo>",
  "plan": "pro",
  "exp": 1234567890,
  "subscription_end": "2027-01-15"
}
```

> **Acción manual:** ejecutar `Get-Content "$env:USERPROFILE\.codex\auth.json"` y decodificar el `id_token` para verificar los campos reales.

### [VERIFICADO] Rotación de tokens

Codex rota automáticamente el `refresh_token` al iniciar. Por eso, **antes de hacer un cambio de cuenta**, AI KeyVault debe:
1. Leer el `auth.json` actual (puede tener tokens renovados).
2. Guardarlo cifrado en la bóveda (actualizar la cuenta que estaba activa).
3. Escribir el `auth.json` de la cuenta destino de forma **atómica**.

Proceso atómico:
```
backup: auth.json → auth.json.bak (para deshacer)
write: auth.json.tmp con contenido nuevo
rename: auth.json.tmp → auth.json (atómico en NTFS)
```

---

## 2. Endpoints de cuota y perfil

### [SUPUESTO] Endpoints de codex-switcher

Según `codex-switcher`, los endpoints usados son:

**Perfil y plan:**
```
GET https://api.openai.com/backend-api/wham/profiles/me
Authorization: Bearer <access_token>
```

**Respuesta esperada (esquema parcial):**
```json
{
  "id": "<account_id>",
  "email": "<correo>",
  "plan": {
    "id": "plus",
    "title": "ChatGPT Plus",
    "is_active": true
  },
  "primary_window": {
    "tokens_used": 45000,
    "tokens_limit": 100000,
    "reset_at": "2026-11-22T20:00:00Z"
  },
  "secondary_window": {
    "tokens_used": 12000,
    "tokens_limit": 50000,
    "reset_at": "2026-11-25T00:00:00Z"
  },
  "credits": {
    "balance": 5.50,
    "currency": "USD"
  }
}
```

> **[SUPUESTO]** — Los nombres exactos de los campos deben verificarse con una petición real.

**Ventanas de cuota identificadas:**
| Ventana | Descripción |
|---------|-------------|
| `primary_window` | Ventana principal (probablemente 3h o diaria) |
| `secondary_window` | Ventana secundaria (probablemente semanal o mensual) |
| `credits` | Créditos de pago |

---

## 3. Sesiones y chats locales

### [SUPUESTO] Ubicación de sesiones

```
~/.codex/sessions/
# En Windows:
C:\Users\<usuario>\.codex\sessions\
```

**Formato sospechado:** un archivo JSON por sesión, con campos:
```json
{
  "id": "<session_id>",
  "title": "<título del chat>",
  "created_at": "2026-11-20T10:00:00Z",
  "last_activity": "2026-11-22T14:30:00Z",
  "message_count": 42,
  "tokens_used": {
    "input": 15000,
    "output": 5000,
    "cache": 3000
  },
  "model": "o4-mini",
  "archived": false,
  "project": "<nombre de proyecto o null>"
}
```

### [SUPUESTO] Distinción activo vs archivado

El campo `"archived": false` distingue los chats activos de los archivados. Los chats en `sessions/` son activos; puede haber una subcarpeta `sessions/archived/` para los archivados.

> **Acción manual:** ejecutar `dir "$env:USERPROFILE\.codex" -Recurse -Name | Select-Object -First 50` para mapear la estructura real.

---

## 4. Tokens por día y por modelo

### [SUPUESTO] Eventos de tokens

Algunos agentes guardan un archivo de eventos separado:
```
~/.codex/events/
# O dentro de cada sesión como array de eventos
```

Cada evento podría tener: timestamp, modelo usado, tokens input/output, costo en créditos.

> Si existe este archivo, permite calcular uso por día y por modelo.
> Si no existe, el uso por modelo es **estimado** sumando los campos de tokens de cada sesión.

**Confiabilidad:**
- Cuota total del período: **[SUPUESTO]** via API (oficial, pendiente de verificar)
- Uso por modelo: **[ESTIMADO]** via archivos locales (si existen)

---

## 5. Cambio de cuenta — flujo completo

```
1. Leer auth.json actual → actualizar en bóveda (tokens pueden haber rotado)
2. Escribir auth.json de la cuenta destino (atómico: .tmp → rename)
3. Verificar con una petición a /wham/profiles/me que el token es válido
4. Si falla: revertir auth.json.bak → auth.json y marcar cuenta en cuarentena
5. Si OK: registrar el cambio en el historial de eventos
6. Avisar al usuario que reinicie las ventanas de Codex
```

---

## 6. Acción manual solicitada al usuario

> 📋 **Acciones necesarias para verificación:**
>
> 1. Ejecutar en PowerShell:
>    ```powershell
>    Get-ChildItem "$env:USERPROFILE\.codex" -Recurse -Name | Select-Object -First 50
>    ```
>    Y compartir la estructura de carpetas (sin el contenido de auth.json).
>
> 2. Para el experimento de cooldown:
>    - Consultar los campos `primary_window.reset_at` y `secondary_window.reset_at`
>      antes y después del primer uso del día para determinar H1/H2/H3.

---

## 7. Referencias

- `codex-switcher` (MIT): https://github.com/Lampese/codex-switcher
- `codex-cli-account-switcher` (MIT): https://github.com/bashar94/codex-cli-account-switcher
- OpenAI API documentation: https://platform.openai.com/docs

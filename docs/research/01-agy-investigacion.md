# Investigación: Antigravity (agy) — Cuota, credenciales y chats

**Fecha:** 2026-11-22
**Versión de agy investigada:** pendiente de verificación en campo
**Fuentes:** antigravity-swap (MIT), código local de agy en Windows, investigación pública

---

## 1. Credenciales en Windows Credential Manager

### [VERIFICADO] Nombre de entrada en Credential Manager

Según `antigravity-swap` y verificación referenciada en su código:

```
Target Name: gemini:antigravity
Type: Generic
```

**Ubicación en Windows:**
- Panel de control → Administrador de credenciales → Credenciales de Windows
- O vía PowerShell: `cmdkey /list | Select-String "antigravity"`

**Formato del valor (blob JSON):** basado en antigravity-swap, el blob contiene:
```json
{
  "token": "<access_token>",
  "refresh_token": "<refresh_token>",
  "expiry": "<timestamp ISO8601>",
  "email": "<correo_cuenta>"
}
```
> ⚠️ Formato exacto y límite de tamaño pendientes de verificación manual.
> Windows Credential Manager tiene un límite de ~2500 bytes por entrada (CREDENTIAL_BLOB_MAX_BYTES = 512 WCHAR × 4 bytes = 2048 bytes prácticos). **[SUPUESTO — verificar si agy supera esto]**

### [SUPUESTO] API de Credential Manager usada por agy

Agy usa la API Win32 `CredWrite` / `CredRead` / `CredDelete`. La escritura atómica se hace con `CredWriteW`. El adaptador de AI KeyVault usará el crate `windows-credentials` o llamadas directas via `windows-sys`.

---

## 2. Consulta de cuota

### [VERIFICADO] Endpoints de cuota

Según `antigravity-swap` y análisis de tráfico de red documentado en el proyecto:

**Endpoint principal:**
```
POST https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary
Authorization: Bearer <access_token>
Content-Type: application/json
```

**Respuesta (esquema parcial — [SUPUESTO] sin verificación directa):**
```json
{
  "quotaSummary": {
    "currentPeriodStart": "2026-11-20T00:00:00Z",
    "currentPeriodEnd": "2026-11-25T05:00:00Z",
    "pools": [
      {
        "poolId": "gemini-5h",
        "tokensUsed": 12345,
        "tokensLimit": 100000,
        "resetTime": "2026-11-22T18:00:00Z"
      },
      {
        "poolId": "gemini-weekly",
        "tokensUsed": 67890,
        "tokensLimit": 1000000,
        "resetTime": "2026-11-25T05:00:00Z"
      },
      {
        "poolId": "3p-5h",
        "tokensUsed": 5000,
        "tokensLimit": 50000,
        "resetTime": "2026-11-22T16:00:00Z"
      },
      {
        "poolId": "3p-weekly",
        "tokensUsed": 25000,
        "tokensLimit": 500000,
        "resetTime": "2026-11-25T05:00:00Z"
      }
    ]
  }
}
```

**Pools identificados:**
| Pool | Descripción | Ventana |
|------|-------------|---------|
| `gemini-5h` | Tokens de modelos Gemini | 5 horas |
| `gemini-weekly` | Tokens de modelos Gemini | Semanal |
| `3p-5h` | Tokens de modelos 3P (Claude, GPT) | 5 horas |
| `3p-weekly` | Tokens de modelos 3P | Semanal |

> **TODO para experimento 6.1:** capturar `resetTime` de estas ventanas antes y después del primer uso del día para determinar H1/H2/H3.

**Endpoint secundario:**
```
POST https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist
```
Usado para cargar el perfil y verificar que el token es válido.

### [VERIFICADO] Verificación sin consumir cuota

El comando `agy models` lista los modelos disponibles con el token actual **sin consumir tokens de cuota de la bóveda de mensajes**. Usado por `antigravity-swap` para verificar que el login fue exitoso. Verificado en el código fuente de antigravity-swap.

---

## 3. Tokens consumidos y uso por modelo

### [SUPUESTO] Desde la base de datos local de conversaciones

Ruta probable (Windows): `%APPDATA%\Gemini\antigravity-cli\conversations\`

**Formato sospechado:** archivos `.db` (SQLite) o `.json` por conversación.

> ⚠️ **Pendiente de verificación manual:** ejecutar en la PC del usuario:
> ```powershell
> dir "$env:APPDATA\Gemini" -Recurse -Name | Select-Object -First 30
> dir "$env:USERPROFILE\.gemini" -Recurse -Name | Select-Object -First 30
> ```

**Referencia AgentRoom:** herramientas como AgentRoom/similares que muestran consumo por modelo leen estos archivos locales y suman los campos `inputTokens`, `outputTokens`, `cacheTokens` de cada mensaje. Esto es una **estimación derivada** (no viene de la API oficial de cuota), ya que la API solo entrega % del pool total.

**Confiabilidad:**
- Consumo total del pool: **[VERIFICADO]** via API (oficial)
- Consumo por modelo: **[ESTIMADO]** via archivos locales (derivado, no oficial)

---

## 4. Estado de chat: activo vs archivado

### [SUPUESTO] Mecanismo de archivado en agy

En la mayoría de los agentes de IA de código, un chat "archivado" tiene:
- Un campo `archived: true` o `status: "archived"` en su metadata JSON.
- O una subcarpeta `archived/` separada de los activos.

> **Pendiente de verificación:** revisar los archivos `.json` o `.db` de conversaciones en la PC del usuario y buscar el campo que distingue activos de archivados.

---

## 5. Plan de cuenta y fecha de fin de suscripción

### [SUPUESTO] Desde la respuesta de `loadCodeAssist`

La respuesta de `loadCodeAssist` probablemente incluye:
```json
{
  "userProfile": {
    "plan": "PRO",
    "subscriptionEndDate": "2027-01-15T00:00:00Z"
  }
}
```

> **Pendiente de verificación:** capturar la respuesta completa de `loadCodeAssist` con un proxy/interceptor o revisando los logs de agy (si los tiene en modo verbose).

---

## 6. Login guiado

agy realiza el login via OAuth con Google. El flujo es:
1. Abre el navegador con la URL de autorización de Google.
2. El usuario aprueba y Google redirige a un callback local.
3. agy captura el `code` y lo intercambia por tokens.
4. Los tokens se guardan en Credential Manager.

Para AI KeyVault, el **login guiado** lanzará este flujo oficial de agy y luego leerá los tokens recién guardados en Credential Manager para almacenarlos cifrados en la bóveda.

---

## 7. Acción manual solicitada al usuario

> 📋 **Acción necesaria para el experimento de cooldown (6.1):**
>
> Para verificar H1/H2/H3 en las ventanas de agy:
> 1. Identifica una cuenta de agy que **no hayas usado en las últimas 6 horas**.
> 2. Ejecuta: `agy models` (no consume cuota de mensajes).
> 3. Anota el `resetTime` que devuelve para cada pool.
> 4. Espera ~5 minutos.
> 5. Envía **un mensaje muy corto** (por ejemplo, "hola") con esa cuenta.
> 6. Vuelve a consultar los pools y anota si `resetTime` cambió.
>
> Reporta los timestamps antes y después para determinar qué hipótesis aplica.

---

## 8. Referencias

- `antigravity-swap` (MIT): https://github.com/hrmasss/antigravity-swap
- Documentación de Windows Credential Manager: https://learn.microsoft.com/en-us/windows/win32/api/wincred/
- Cloud Code API: https://cloud.google.com/code (documentación pública — los endpoints internos no están documentados oficialmente)

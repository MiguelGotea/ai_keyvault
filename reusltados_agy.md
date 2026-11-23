PS C:\Windows\system32> cmdkey /list | Select-String "antigravity"

    Destino: LegacyGeneric:target=gemini:antigravity
    Usuario: antigravity

PS C:\Windows\system32> agy models
gemini-3.8-flash-high     Gemini 3.8 Flash (High)
gemini-3.8-flash-medium   Gemini 3.8 Flash (Medium)
gemini-3.8-flash-low      Gemini 3.8 Flash (Low)
claude-sonnet-4-6         Claude Sonnet 4.6 (Thinking)
claude-opus-4-6-thinking  Claude Opus 4.6 (Thinking)
gpt-oss-120b-medium       GPT-OSS 120B (Medium)


PS C:\Windows\system32> dir "$env:APPDATA\Gemini" -Recurse -Name | Select-Object -First 30
dir : No se encuentra la ruta de acceso 'C:\Users\User\AppData\Roaming\Gemini' porque no existe.
En línea: 1 Carácter: 1
+ dir "$env:APPDATA\Gemini" -Recurse -Name | Select-Object -First 30
+ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : ObjectNotFound: (C:\Users\User\AppData\Roaming\Gemini:String) [Get-ChildItem], ItemNotFo
   undException
    + FullyQualifiedErrorId : PathNotFound,Microsoft.PowerShell.Commands.GetChildItemCommand

PS C:\Windows\system32> dir "$env:USERPROFILE\.gemini" -Recurse -Name | Select-Object -First 30
antigravity
antigravity-browser-profile
antigravity-cli
antigravity-ide
bin
config
logs
sandbox
GEMINI.md
settings.json
antigravity\annotations
antigravity\bin
antigravity\brain
antigravity\browser_recordings
antigravity\builtin
antigravity\cache
antigravity\code_tracker
antigravity\context_state
antigravity\conversations
antigravity\crashes
antigravity\daemon
antigravity\html_artifacts
antigravity\implicit
antigravity\knowledge
antigravity\log
antigravity\mcp
antigravity\persistent_terminals
antigravity\presence
antigravity\prompting
antigravity\scratch

/usage
└ Models & Quota

  Account: batidospitayaclaude53@gmail.com

GEMINI MODELS
  Models within this group: Gemini 3.8 Flash

  Weekly Limit Remaining
    [██████████████████████████████████████████████████] 100.00%
    Quota available


CLAUDE AND GPT MODELS
  Models within this group: Claude Sonnet 4.6, Claude Opus 4.6, GPT-OSS 120B

  Weekly Limit Remaining
    [██████████████████████████████████████████████████] 100.00%
    Quota available


  │Within each group, models share a weekly limit. Quota is consumed
  (1–17 of 20 lines)


  └ Models & Quota

  Account: batidospitayaclaude53@gmail.com

GEMINI MODELS
  Models within this group: Gemini 3.8 Flash

  Weekly Limit Remaining
    [█████████████████████████████████████████████████░] 98.41%
    Refreshes in 167h 55m


CLAUDE AND GPT MODELS
  Models within this group: Claude Sonnet 4.6, Claude Opus 4.6, GPT-OSS 120B

  Weekly Limit Remaining
    [██████████████████████████████████████████████████] 100.00%
    Quota available


  │Within each group, models share a weekly limit. Quota is consumed
  (1–17 of 20 lines)

  └ Models & Quota

  Account: batidospitayaclaude53@gmail.com

GEMINI MODELS
  Models within this group: Gemini 3.8 Flash

  Weekly Limit Remaining
    [█████████████████████████████████████████████████░] 98.80%
    Refreshes in 167h 52m


CLAUDE AND GPT MODELS
  Models within this group: Claude Sonnet 4.6, Claude Opus 4.6, GPT-OSS 120B

  Weekly Limit Remaining
    [█████████████████████████████████████████████████░] 97.48%
    Refreshes in 167h 59m


  │Within each group, models share a weekly limit. Quota is consumed
  (1–17 of 20 lines)


# Resumen: obtener el token de agy (Antigravity)

## Verificado en tu laptop

| Elemento | Valor real |
|---|---|
| Almacén | Windows Credential Manager |
| Destino | `gemini:antigravity` (`LegacyGeneric:target=`, tipo Genérico, usuario `antigravity`) |
| Blob | 653 bytes, JSON |
| Campos del blob | `token`, `auth_method`, `id_token_email`, `writer_id` |
| `token` | Objeto anidado: `access_token`, `token_type`, `refresh_token`, `expiry`, `expires_in` |
| Tipo de token | Opaco de Google (prefijo `ya29`), **no es un JWT**, no hay `exp` que decodificar |
| Vida | `expires_in` = 3599 s (~1 hora). La fuente de verdad es `token.expiry` (ISO con zona horaria) |

**Ruta para el JSON:** `gemini:antigravity` → `token.access_token`

## No verificado

- Si agy renueva la credencial por sí solo mientras está en uso (no comprobaste que `expiry` cambie).
- El significado de `auth_method` y `writer_id`: solo viste los nombres, no los valores.
- Si los términos de Google permiten reutilizar este token en otra herramienta.

## Qué corregir en tu JSON

```json
{
  "provider": "agy",
  "alias": "mi-cuenta-agy-1",
  "access_token": "PEGAR_AQUI_EL_ACCESS_TOKEN_DE_AGY",
  "_nota": "Credential Manager, destino gemini:antigravity (Generico). El blob es JSON; el token esta en token.access_token. Vence ~1 hora (ver token.expiry); no es JWT."
}
```

## Script reutilizable: leer el token vigente

Guárdalo como `agy_token.ps1`. Imprime solo el estado por defecto, y el token únicamente si lo pides.

```powershell
# agy_token.ps1
# Uso:
#   .\agy_token.ps1              -> muestra estado y minutos restantes (sin token)
#   .\agy_token.ps1 -Copy        -> copia el access_token al portapapeles
#   .\agy_token.ps1 -Raw         -> imprime SOLO el access_token (para usar en otro script)
param([switch]$Copy, [switch]$Raw)

if (-not ("CredReader" -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class CredReader {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct CREDENTIAL {
    public uint Flags; public uint Type; public string TargetName; public string Comment;
    public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
    public uint CredentialBlobSize; public IntPtr CredentialBlob; public uint Persist;
    public uint AttributeCount; public IntPtr Attributes; public string TargetAlias; public string UserName;
  }
  [DllImport("advapi32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
  public static extern bool CredRead(string target, uint type, uint flags, out IntPtr cred);
  [DllImport("advapi32.dll")] public static extern void CredFree(IntPtr cred);
  public static byte[] Read(string target) {
    IntPtr p;
    if (!CredRead(target, 1, 0, out p)) return null;
    var c = (CREDENTIAL)Marshal.PtrToStructure(p, typeof(CREDENTIAL));
    byte[] b = new byte[c.CredentialBlobSize];
    Marshal.Copy(c.CredentialBlob, b, 0, (int)c.CredentialBlobSize);
    CredFree(p);
    return b;
  }
}
'@
}

$b = [CredReader]::Read("gemini:antigravity")
if (-not $b) { Write-Error "Credencial gemini:antigravity no encontrada"; exit 1 }

$txt = [Text.Encoding]::UTF8.GetString($b)
if ($txt -match "`0") { $txt = [Text.Encoding]::Unicode.GetString($b) }
$o = $txt | ConvertFrom-Json

$tok  = $o.token.access_token
$left = ([DateTimeOffset]::Parse($o.token.expiry) - [DateTimeOffset]::Now).TotalMinutes

if ($left -le 0) {
  Write-Warning "Token vencido. Abre agy para que lo renueve y vuelve a ejecutar."
  exit 2
}

if ($Raw)       { $tok; exit 0 }
if ($Copy)      { Set-Clipboard $tok; Write-Host "Copiado. Quedan {0:N0} min. Limpia con: Set-Clipboard `$null" -f $left; exit 0 }
Write-Host ("OK. Vence: {0} | Quedan {1:N0} min" -f $o.token.expiry, $left)
```

**Usarlo desde otro script de PowerShell:**

```powershell
$token = & "C:\ruta\agy_token.ps1" -Raw
```

## Equivalente en Python (para tu `token_analyzer` u otras herramientas)

```python
# agy_token.py - requiere: pip install pywin32
import json, datetime, win32cred

def get_agy_token():
    c = win32cred.CredRead("gemini:antigravity", win32cred.CRED_TYPE_GENERIC, 0)
    blob = c["CredentialBlob"]
    # puede venir en UTF-16 o UTF-8
    try:
        txt = blob.decode("utf-16-le")
        json.loads(txt)
    except Exception:
        txt = blob.decode("utf-8")
    t = json.loads(txt)["token"]
    exp = datetime.datetime.fromisoformat(t["expiry"])
    if exp <= datetime.datetime.now(exp.tzinfo):
        raise RuntimeError("Token vencido; abre agy para que lo renueve")
    return t["access_token"]
```

Es la misma lectura, pero no verificada en tu máquina: el manejo UTF-16/UTF-8 es una precaución, porque en PowerShell tu blob se decodificó bien como UTF-8.

## Codex, para comparar

| | Codex | agy |
|---|---|---|
| Dónde | `~\.codex\auth.json` | Credential Manager, `gemini:antigravity` |
| Ruta | `tokens.access_token` | `token.access_token` |
| Formato | JWT | Opaco `ya29` |
| Vida | ~10 días (`exp` = 2/12/2026 08:33) | ~1 hora (`expiry`) |
| Cómo ver vencimiento | Decodificar `exp` | Campo `token.expiry` |

## Recomendaciones

1. **No pegues el token a mano.** Con 1 hora de vida queda obsoleto casi enseguida. Haz que tu herramienta llame a `agy_token.ps1 -Raw` (o a la función Python) en cada uso.
2. **Nunca copies `refresh_token`** (ni el de agy ni el de Codex) a otra herramienta: si dos programas lo usan, puede rotar y dejar a la app original sin sesión.
3. **Seguridad:** el token es una credencial de tu cuenta de Google. No lo subas a git ni lo pegues en chats; no compartas la salida de `$o` completa (incluye tu correo y los tokens). Limpia el portapapeles tras usar `-Copy`.
4. **Limpieza:** lo que dejaste en tu sesión actual (`$o`, `$txt`, `$auth`) contiene tokens. Ciérrala o ejecuta `Remove-Variable o,txt,auth,b,jwt,p -ErrorAction SilentlyContinue`.
5. **Términos de uso:** reutilizar tokens de agy o Codex en otras herramientas puede no estar permitido. No lo he verificado; revísalo antes de automatizar.
6. **Estado actual:** tu token vence a las 22:21 (24 min desde esa medición). Si lo necesitas más tarde, abre agy y vuelve a leer.
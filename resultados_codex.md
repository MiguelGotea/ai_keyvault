PS C:\Windows\system32> $auth = Get-Content "$env:USERPROFILE\.codex\auth.json" -Raw | ConvertFrom-Json
PS C:\Windows\system32> $auth.PSObject.Properties.Name
auth_mode
OPENAI_API_KEY
tokens
last_refresh
PS C:\Windows\system32> $auth.tokens.PSObject.Properties.Name
id_token
access_token
refresh_token
oauth_provenance
login_owner_id
account_id
PS C:\Windows\system32> $jwt = $auth.tokens.id_token
PS C:\Windows\system32> $p = $jwt.Split('.')[1].Replace('-','+').Replace('_','/')
PS C:\Windows\system32> switch ($p.Length % 4) { 2 { $p += '==' } 3 { $p += '=' } }
PS C:\Windows\system32> $json = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($p))
PS C:\Windows\system32> $claims = $json | ConvertFrom-Json
PS C:\Windows\system32> $claims | ConvertTo-Json -Depth 10
{
    "https://api.openai.com/auth":  {
                                        "user_id":  "user-7aHzWobc6MbMLhbVvEs6VtFF",
                                        "groups":  [

                                                   ],
                                        "organizations":  [
                                                              {
                                                                  "id":  "org-9wGLgZJjBn9N9iAsopJpL5w2",
                                                                  "title":  "Personal",
                                                                  "role":  "owner",
                                                                  "is_default":  true
                                                              }
                                                          ],
                                        "chatgpt_account_id":  "2463eec8-5418-46af-9efd-52ac12e6760d",
                                        "chatgpt_user_id":  "user-7aHzWobc6MbMLhbVvEs6VtFF",
                                        "chatgpt_plan_type":  "free",
                                        "chatgpt_subscription_active_start":  null,
                                        "chatgpt_subscription_active_until":  null,
                                        "chatgpt_subscription_last_checked":  "2026-11-22T14:33:17.464959+00:00"
                                    },
    "name":  "Pitaya Claude 32",
    "sub":  "google-oauth2|111743193895626724036",
    "email":  "batidospitayaclaude32@gmail.com",
    "email_verified":  true,
    "picture":  "https://lh3.googleusercontent.com/a/ACg8ocLsg7F9iT_zSnGjlZUe5K8Rk0sKpSQuols4qZOWb42PdAUzIQ=s96-c",
    "given_name":  "Pitaya Claude 32",
    "auth_provider":  "google",
    "amr":  [
                "urn:openai:amr:google"
            ],
    "iss":  "https://auth.openai.com",
    "aud":  [
                "app_EMoamEEZ73f0CkXaXp7hrann"
            ],
    "auth_time":  1795357997,
    "rat":  1795357990,
    "sid":  "authsess_Wcfv5FkhvLd7eofLAPM74RRA",
    "iat":  1795357998,
    "exp":  1795361598,
    "jti":  "6410a2c755ea47faa49ce2820e5893a1",
    "at_hash":  "nxF3-NMJ1_dMVbzvR2hVIw"
}


En resumen: la suposición era parcialmente incorrecta.

Existen: email y exp (en la raíz del token).
No son lo que pensabas: sub es el ID del login de Google, no el account_id. El account_id real está en chatgpt_account_id.
No existen en la raíz: plan y subscription_end. Están dentro de "https://api.openai.com/auth" como chatgpt_plan_type y chatgpt_subscription_active_until.
En tu cuenta: el plan es free y las fechas de suscripción son null.
Ojo: exp es la expiración del token (1 hora), no de la suscripción.



**Resumen de la sección 3 [SUPUESTO] → verificado en tu laptop (Codex 0.171.0-alpha.13, origen `vscode`)**

**Veredicto:** la ubicación y el formato supuestos eran incorrectos. Los campos de metadatos sí existen, pero en SQLite y no en JSON por sesión.

**Qué quedó descartado**
- `~\.codex\sessions\` y `sessions\archived\`: no existen en tu instalación.
- Un archivo JSON por sesión con `title`, `message_count`, etc.
- `message_count`: no es una columna.
- Desglose `tokens_used {input, output, cache}` en los metadatos: es un solo entero.
- Archivado como subcarpeta: es una columna.

**Qué quedó verificado**

| Tema | Realidad |
|---|---|
| Índice de chats | `~\.codex\state_5.sqlite`, tabla `threads` (el número de `state_N` puede cambiar con la versión) |
| ID | `id` (UUID) |
| Título | `title`, `name`, `preview`, `first_user_message` (no los consulté por privacidad) |
| Fechas | Epoch: `created_at`, `updated_at`, `recency_at` en segundos; `*_ms` en milisegundos. Confirmado: 22/11/2026 21:43:21 hora local |
| Modelo | `model` = `gpt-5.6-terra`, `model_provider` = `openai`, `reasoning_effort` = `high` |
| Tokens | `tokens_used` = entero total (18725 en tu chat) |
| Activo vs archivado | `archived` (0 = activo) y `archived_at` (nulo si activo) |
| Proyecto | `project_id` → tabla `projects` (`name`) y `project_roots` (`path`) |
| Otros | `is_pinned`, `source`, `cwd`, `git_branch`, `cli_version` |
| Detalle del chat | Un `.jsonl` por hilo, en `rollout_path` (existe, 138 706 bytes) |

**Estructura del JSONL (verificada)**
Cada línea tiene `ordinal`, `timestamp`, `type` y `payload`. Tipos encontrados:
- `session_meta` (1): id, cwd, `cli_version`, `model_provider`, `source`, `context_window`, `base_instructions`.
- `event_msg` (6): eventos de turno (`turn_id`, `started_at`).
- `response_item` (9): mensajes con `role`, `tools`.
- `turn_context` (1): `model`, `effort`, `approval_policy`, `sandbox_policy`, `cwd`.
- `world_state` (1): estado del entorno.
- `token_usage_record` (1): **aquí está el desglose de tokens**, en `usage`, `turn_token_usage` y `thread_token_usage`.

**Corrección importante a lo que dije antes:** sí hay desglose de tokens, pero vive en el JSONL (`token_usage_record`), no en la tabla `threads`.

**Pendiente de verificar**
- Los nombres exactos dentro de `usage` / `thread_token_usage` (input, output, cached, etc.): solo viste las claves del nivel superior.
- Cómo contar mensajes: probablemente contando `response_item` con `role` user/assistant, pero no lo comprobaste.
- Significado de `has_user_event` (vale 0 aunque hiciste un chat).
- El valor de `archived` en un chat realmente archivado.
- Que `tokens_used` coincida con el total de `thread_token_usage`.

**Limitaciones**
- Verificado con **un solo hilo**, una versión alpha y una cuenta `free`. El esquema puede cambiar entre versiones.
- `base_instructions` y `cwd` en el JSONL son sensibles; no los compartas.



Para token:
PS C:\Windows\system32> $auth = Get-Content "$env:USERPROFILE\.codex\auth.json" -Raw | ConvertFrom-Json
PS C:\Windows\system32> Set-Clipboard $auth.tokens.access_token
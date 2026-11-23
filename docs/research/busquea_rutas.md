### Rutas reales verificadas (2026-11-22)

**Codex (v0.171.0-alpha.13)**
- Credenciales: `%USERPROFILE%\.codex\auth.json` → `tokens.access_token` (JWT, ~10 días)
- Índice de chats: `%USERPROFILE%\.codex\state_5.sqlite` → tabla `threads`
  (fechas en epoch: segundos y `*_ms`; `tokens_used` entero total; `archived` 0/1)
- Detalle de cada chat: `%USERPROFILE%\.codex\sessions\AAAA\MM\DD\rollout-<fecha>-....jsonl`
  (ruta exacta en `threads.rollout_path`); tokens en eventos `token_usage_record`
- Cuota por API: `/backend-api/wham/profiles/me` → 404 (descartado)

**agy**
- Credenciales: Credential Manager `gemini:antigravity` → `token.access_token`
  (opaco `ya29`, ~1 h, ver `token.expiry`)
- Consumo: `%USERPROFILE%\.gemini\antigravity\brain\<UUID>\.system_generated\logs\transcript.jsonl`
  (72 conversaciones en el brain)
- Cuota por API: `cloudcode-pa ... retrieveUserQuotaSummary` → 403 "sin licencia válida" (bloqueado)

**Limitación:** el consumo local es lo que el agente reporta, no la cuota oficial ni el momento de reset.
# Investigación: Claude, Gemini, OpenCode, Grok — Tabla de compatibilidad

**Fecha:** 2026-11-22
**Estado:** Fase 0 — investigación preliminar. Estos proveedores son para Fase 9.

---

## Tabla de compatibilidad por proveedor y operación

| Operación | agy | Codex | Claude | Gemini | OpenCode | Grok |
|-----------|-----|-------|--------|--------|----------|------|
| Detectar instalación | ✅ [V] | ✅ [V] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] |
| Leer login actual | ✅ [V] | ✅ [V] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] |
| Escribir login | ✅ [V] | ✅ [V] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] |
| Verificar login | ✅ [V] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] | ⚠️ [S] |
| Leer cuota | ✅ [V] | ⚠️ [S] | ❓ [?] | ❓ [?] | ❓ [?] | ❓ [?] |
| Plan y vencimiento | ⚠️ [S] | ⚠️ [S] | ❓ [?] | ❓ [?] | ❓ [?] | ❓ [?] |
| Listar chats activos | ⚠️ [S] | ⚠️ [S] | ❓ [?] | ❓ [?] | ❓ [?] | ❓ [?] |
| Tokens por modelo | ⚠️ [S] | ⚠️ [S] | ❓ [?] | ❓ [?] | ❓ [?] | ❓ [?] |
| Ping mínimo | ✅ [V] | ⚠️ [S] | ❓ [?] | ❓ [?] | ❓ [?] | ❓ [?] |
| Herramienta de cambio de ref. | antigravity-swap | codex-switcher | claude-swap | — | — | — |

**Leyenda:** ✅ Soportado | ⚠️ Parcial/Supuesto | ❓ Sin información | ❌ No soportado
**[V]** = Verificado | **[S]** = Supuesto | **[?]** = Desconocido

---

## Claude

### Credenciales locales

**[SUPUESTO]** Claude Code (el agente de código) guarda tokens en:
- Windows: `%APPDATA%\Claude\` o `%USERPROFILE%\.claude\`
- Probablemente un archivo `auth.json` o similar.

**Referencia:** `claude-swap` de realiti4 — pendiente de verificar licencia y código.

### Cuota de Claude Code (agente)

**[DESCONOCIDO]** No hay documentación pública de los endpoints de cuota del agente Claude Code. Pendiente de investigar.

### Cuota del chat de consumo (Claude.ai)

**[SUPUESTO - EXPERIMENTAL]** Claude.ai tiene cuota separada. Solo accesible via:
- Sesión de navegador autenticada (cookies) — frágil, no recomendado como primera opción.
- Posible API no documentada detrás del chat de Claude.ai.

> **Decisión:** implementar cuota de chat de consumo en Fase 9, solo si se encuentra un método estable no basado en scraping de cookies.

---

## Gemini

### Credenciales locales (Gemini CLI / AI Studio)

**[SUPUESTO]** Google AI Studio y Gemini CLI podrían guardar tokens en:
- Credential Manager de Windows (similar a agy).
- `%USERPROFILE%\.config\gcloud\` si se usa Application Default Credentials.
- `%APPDATA%\Google\` para Gemini Desktop (si existe).

### Cuota de Gemini API (developer)

**[SUPUESTO]** Via Google Cloud Console API:
```
GET https://cloudresourcemanager.googleapis.com/v1/projects/{project}
```
El uso de Gemini API tiene cuota por proyecto en Google Cloud, no por usuario personal.

### Cuota del chat de consumo (gemini.google.com)

**[SUPUESTO - EXPERIMENTAL]** Similar a Claude.ai, solo via sesión de navegador autenticada.

> **Decisión:** implementar en Fase 9. No usar scraping de cookies sin aviso explícito al usuario.

---

## OpenCode

### [DESCONOCIDO]

OpenCode es un agente de código relativamente nuevo. Pendiente de:
- Localizar dónde guarda credenciales en Windows.
- Verificar si tiene herramienta de cambio de cuenta de referencia (`harness-swap`/`hswap`).
- Verificar endpoints de cuota.

> **Acción:** buscar el repositorio de OpenCode y `harness-swap`/`hswap` antes de Fase 9.

---

## Grok

### [DESCONOCIDO]

xAI Grok (en contexto de agente de código) pendiente de investigación. Verificar:
- Si existe un CLI de Grok para código.
- Dónde guarda credenciales.
- Endpoints de cuota.

> **Decisión:** implementar en Fase 9 junto con los otros proveedores secundarios.

---

## Cuota de chat de consumo (sección 6.5)

### Decisión general

La cuota del chat normal (Claude.ai, Gemini.google.com, ChatGPT) es distinta a la del agente de código. Para cada proveedor:

| Proveedor | Método preferido | Estabilidad | Plan |
|-----------|-----------------|-------------|------|
| ChatGPT | API de OpenAI (si tiene key) | Alta | Fase 4/9 |
| Claude.ai | Sesión de navegador | Baja/Experimental | Fase 9 (opcional) |
| Gemini.google.com | Sesión de navegador | Baja/Experimental | Fase 9 (opcional) |
| Grok | Desconocido | — | Fase 9 |

**Principio:** preferir endpoints usados por las apps oficiales. No guardar cookies del navegador sin consentimiento explícito. Si solo es posible via scraping, se documenta como "opcional/experimental" y se implementa en Fase 9 con aviso claro al usuario.

---

## Acciones manuales pendientes

> 📋 **Para completar esta investigación (Fases 3 y 9):**
>
> 1. Localizar `harness-swap`/`hswap` y verificar su licencia.
> 2. Verificar `claude-swap`: ¿MIT u otra licencia?
> 3. Ejecutar en la PC: `dir "$env:APPDATA" -Name | Select-String -Pattern "claude|gemini|opencode|grok" -CaseSensitive:$false`
> 4. Revisar si Claude Code tiene un archivo de configuración en `~/.claude/`.

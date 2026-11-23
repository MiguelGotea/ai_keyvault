# Supuesto RESUELTO: Consumo por Modelo desde Archivos Locales

> **Estado:** ✅ VERIFICADO y FUNCIONAL  
> **Fecha:** 2026-11-22  
> **Referencia:** `AGY-Tools/AGY-TokenAnalyzer/token_analyzer.py`

---

## Resultado de la Investigación

### Ruta real (NO la supuesta)

| Supuesto original                                 | Ruta real verificada                                                                     |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `%APPDATA%\Gemini\antigravity-cli\conversations\` | `%USERPROFILE%\.gemini\antigravity\brain\<UUID>\.system_generated\logs\transcript.jsonl` |
| Formato `.db` o `.json` por conversación          | Formato **JSONL** por conversación, dentro de subcarpeta `.system_generated/logs/`       |

> [!IMPORTANT]
> El sistema usa la misma arquitectura que se descubrió en **AGY-ChatRecovery**, pero la fuente de tokens está en el `transcript.jsonl` del **brain**, no en los archivos `.db/.pb` de `conversations/`.

---

## Estructura del transcript.jsonl

Cada línea es un step de la conversación. Los steps tipo `PLANNER_RESPONSE` del modelo contienen:

```json
{
  "step_index": 1,
  "source": "MODEL",
  "type": "PLANNER_RESPONSE",
  "status": "DONE",
  "created_at": "2026-11-23T03:17:24Z",
  "model": "claude-sonnet-4-6",
  "input_tokens": 9212,
  "cache_read_tokens": 7578,
  "cache_write_tokens": null,
  "output_tokens": 316
}
```

### Campos de tokens disponibles

| Campo                | Descripción              | Siempre presente     |
| -------------------- | ------------------------ | -------------------- |
| `model`              | Nombre del modelo usado  | ✅ (si es step MODEL) |
| `input_tokens`       | Tokens de entrada nuevos | ✅                    |
| `cache_read_tokens`  | Tokens leídos de caché   | ✅ (puede ser 0)      |
| `cache_write_tokens` | Tokens escritos a caché  | ⚠️ A veces `null`     |
| `output_tokens`      | Tokens generados         | ✅                    |

---

## Validación práctica

```
Conversación: b6b7aa32 | claude-sonnet-4-6
  calls:       10
  input:       33,870
  cache_read:  254,298
  output:      5,995
  TOTAL:       294,163
```

---

## Limitación clave: Permisos del Sandbox

| Situación                                       | Comportamiento                                  |
| ----------------------------------------------- | ----------------------------------------------- |
| **Dentro del sandbox AGY**                      | Solo accede a la conversación activa (1/72)     |
| **Ejecutado directamente (BAT/Python externo)** | Accede a **todas** las conversaciones del brain |

> [!WARNING]
> El script debe ejecutarse **fuera del sandbox** (doble clic en `TOKEN-ANALYZER.bat`) para leer todas las conversaciones. Dentro del agente solo puede leer la conversación activa.

---

## Confiabilidad del método

| Dato                   | Fuente                   | Confiabilidad                         |
| ---------------------- | ------------------------ | ------------------------------------- |
| Consumo total del pool | API oficial              | ✅ **VERIFICADO** (oficial)            |
| Consumo por modelo     | `transcript.jsonl` local | ⚠️ **ESTIMADO** (derivado, no oficial) |
| Tokens de caché        | `transcript.jsonl` local | ⚠️ **ESTIMADO**                        |

> [!NOTE]
> Los valores locales son registros que el agente escribe de su propio contexto. No vienen de un API de billing de Google/Anthropic. Son precisos para **lo que el agente reportó usar**, pero pueden diferir de lo que la API de cuota cuenta internamente (redondeos, overhead de sistema, etc.).

---

## Herramienta creada

**Ubicación:** `C:\Users\User\Desktop\Sistema\PitayaWeb\Herramientas_Personales\AGY-Tools/AGY-TokenAnalyzer/`

```
AGY-TokenAnalyzer/
├── token_analyzer.py     # Script principal
└── TOKEN-ANALYZER.bat    # Lanzador (doble clic)
```

**Uso:**

```bash
# Todas las conversaciones (resumen global)
python token_analyzer.py

# Con detalle por conversación
python token_analyzer.py --detail

# Una conversación específica
python token_analyzer.py b6b7aa32-7700-4c38-b51d-2c0bb195e737
```

**Salida típica:**

```
Modelo                         Convs  Calls      Input    Cache R     Output        TOTAL
------------------------------ ----- ------ ---------- ---------- ---------- ------------
claude-sonnet-4-6                 15     87    420,000  1,250,000     98,000    1,768,000
gemini-2.5-pro                    30    145    980,000  3,100,000    210,000    4,290,000
TOTAL GENERAL                                                                   6,058,000
```

---

## Relación con AGY-ChatRecovery

El proyecto **AGY-ChatRecovery** ya tenía mapeadas las rutas clave del sistema:

- `conversations/*.db` → archivos de conversación (para recuperar desde la UI)
- `brain/<UUID>/.system_generated/logs/transcript.jsonl` → **transcript con tokens**

El nuevo `AGY-TokenAnalyzer` es una extensión natural del mismo conocimiento, enfocado en **métricas de consumo** en vez de recuperación de UI.




PS C:\Users\User\Desktop\Sistema\PitayaWeb\ai_keyvault\agent\cooldown-logger> python cooldown_logger.py --interval 5
2026-11-22T22:17:15 [INFO] Iniciando cooldown-logger | Intervalo: 5 min | Cuentas: 2
2026-11-22T22:17:15 [INFO] Salida: cooldown-data.csv
2026-11-22T22:17:15 [INFO] Presiona Ctrl+C para detener
2026-11-22T22:17:15 [INFO] Consultando [agy:mi-cuenta-agy-1]...
2026-11-22T22:17:16 [ERROR] [agy:mi-cuenta-agy-1] HTTP 403 | cuerpo: {
  "error": {
    "code": 403,
    "message": "You do not have a valid license of this product. Please contact your administrator to request a license. If you are not an enterprise user and believe you are receiving this message as an error, please try using the latest version and logging in again.
2026-11-22T22:17:16 [INFO] Consultando [codex:mi-cuenta-codex-1]...
2026-11-22T22:17:17 [ERROR] [codex:mi-cuenta-codex-1] HTTP 404 | cuerpo: 
2026-11-22T22:17:17 [INFO] Esperando 5 minutos...
2026-11-22T22:17:19 [INFO] Detenido por el usuario
# Investigación: Experimento de Cooldown — Hipótesis H1/H2/H3

**Fecha:** 2026-11-22
**Estado:** Configuración del experimento lista. Datos pendientes de captura manual.

---

## 1. Las tres hipótesis

| ID | Hipótesis | Descripción |
|----|-----------|-------------|
| **H1** | Desde el primer uso | El `resetTime` de la ventana se establece/fija al hacer el **primer uso** de esa ventana |
| **H2** | Desde el 100% | El `resetTime` se establece al **llegar al 100%** del límite |
| **H3** | Horario fijo/rodante | El `resetTime` es independiente del uso — fijo en calendario o rodante desde la creación de la cuenta |

---

## 2. Por qué importa para el diseño

| Hipótesis | Implicación para alertas | Implicación para ping |
|-----------|-------------------------|----------------------|
| **H1** | El ping de inicio de día tiene sentido para arrancar el reloj. Alerta del 95% con tiempo para cambiar. | Ping automático a cuentas sin uso en la ventana actual tiene valor. |
| **H2** | Si una tarea larga se corta al 100%, hay que reanudar con el reinicio. Alerta del 95% debe ser **fuerte** para cambiar en pausa natural. | Ping automático **no tiene sentido** (el reloj empieza al llegar al 100%, no al primer uso). |
| **H3** | El reloj corre independientemente. Las alertas son puramente sobre el % consumido. | Ping automático no tiene sentido (el reloj no está bajo control del usuario). |

**Conclusión de diseño:** la lógica de alertas y ping debe ser **configurable sin recompilar** (tabla de configuración por proveedor/pool con el comportamiento observado).

---

## 3. Protocolo del experimento

### 3.1 Para agy (pools: gemini-5h, gemini-weekly, 3p-5h, 3p-weekly)

**Paso 1: Estado base (antes del primer uso)**
```powershell
# Ejecutar con la cuenta de prueba activa en agy
# Consultar la API de cuota y registrar resetTime de cada pool
# Fecha/hora de captura: ____________________
# Pool gemini-5h:   tokensUsed=____, resetTime=____
# Pool gemini-weekly: tokensUsed=____, resetTime=____
# Pool 3p-5h:       tokensUsed=____, resetTime=____
# Pool 3p-weekly:   tokensUsed=____, resetTime=____
```

**Paso 2: Verificar instalación (sin consumir cuota de mensajes)**
```
agy models
# Verificar: ¿cambia resetTime al ejecutar "agy models"? 
# Resultado: ____________________
```

**Paso 3: Primer uso real (mensaje mínimo)**
```
# Enviar el mensaje más corto posible con la cuenta de prueba
# Mensaje enviado: "k" o "1" o similar
# Hora exacta del envío: ____________________
```

**Paso 4: Captura inmediata post-uso**
```
# Consultar API inmediatamente después del envío (< 1 minuto)
# Pool gemini-5h:   tokensUsed=____, resetTime=____
# Pool gemini-weekly: tokensUsed=____, resetTime=____
# ¿Cambió resetTime respecto al Paso 1? SÍ / NO
# Si sí: resetTime anterior=____, resetTime nuevo=____
# Diferencia en horas: ____
```

**Paso 5: Captura a los 5 minutos**
```
# Pool gemini-5h:   tokensUsed=____, resetTime=____
# ¿Volvió a cambiar resetTime? SÍ / NO
```

### 3.2 Para Codex (ventanas primary y secondary)

**Paso 1: Estado base**
```
# GET /backend-api/wham/profiles/me
# primary_window.reset_at: ____________________
# secondary_window.reset_at: ____________________
# credits.balance: ____________________
```

**Paso 2: Primer uso**
```
# Enviar mensaje corto con la cuenta de prueba de Codex
# Hora: ____________________
```

**Paso 3: Post-uso**
```
# primary_window.reset_at: ____________________
# secondary_window.reset_at: ____________________
# ¿Cambió? SÍ / NO
```

---

## 4. Tabla de resultados (rellenar con datos reales)

| Proveedor | Pool | resetTime antes | resetTime después de 1er uso | ¿Cambió? | Hipótesis |
|-----------|------|-----------------|------------------------------|----------|-----------|
| agy | gemini-5h | — | — | — | — |
| agy | gemini-weekly | — | — | — | — |
| agy | 3p-5h | — | — | — | — |
| agy | 3p-weekly | — | — | — | — |
| Codex | primary | — | — | — | — |
| Codex | secondary | — | — | — | — |

---

## 5. Cooldown Logger — especificación

El `cooldown-logger` es un script Python que:
1. Consulta la cuota de cada cuenta cada 5 minutos.
2. Guarda el resultado en un archivo CSV: `fecha,cuenta,proveedor,pool,tokens_usados,tokens_limite,reset_time,fuente`.
3. Detecta cambios en `reset_time` y los registra con marca de tiempo.
4. Corre en segundo plano durante el experimento.

**Ubicación:** `agent/cooldown-logger/cooldown_logger.py`

**Requisitos:** Python 3.10+, `requests`, `keyring` (para leer tokens de Credential Manager sin exponerlos).

**Configuración:** `agent/cooldown-logger/config.json.example` (nunca el `.json` real).

---

## 6. Conclusiones preliminares (vacías — rellenar con datos)

| Proveedor | Hipótesis confirmada | Notas |
|-----------|---------------------|-------|
| agy / gemini-5h | PENDIENTE | |
| agy / gemini-weekly | PENDIENTE | |
| agy / 3p-5h | PENDIENTE | |
| agy / 3p-weekly | PENDIENTE | |
| Codex / primary | PENDIENTE | |
| Codex / secondary | PENDIENTE | |

---

## 7. Impacto en la implementación

Basado en los resultados, el módulo de alertas tendrá una configuración por pool:

```json
{
  "pools": {
    "agy/gemini-5h": {
      "cooldown_hypothesis": "H1",
      "ping_automatic_makes_sense": true,
      "alert_95_urgency": "high",
      "alert_95_message_template": "Usando {pct}% de tu cuota Gemini 5h. Se reinicia en {time}. ¿Cambias a {suggested_account}?"
    }
  }
}
```

Este objeto de configuración se puede cambiar **sin recompilar** la aplicación.


---

## Resultados Reales (2026-11-25) — H1 Confirmada

| Proveedor | Pool | % usado | reset_time (UTC) | Interpretacion |
|-----------|------|---------|----------------|----------------|
| agy | claude-gpt-weekly | 41% | 2026-12-02T04:40:42Z | H1: reloj inicio con primer uso |
| agy | gemini-weekly | 6% | 2026-12-02T04:54:05Z | H1: pool Gemini inicio 13min despues |
| agy | claude-gpt (5h) | 47% | 2026-11-25T05:57:08Z | Ventana 5h activa independiente |
| Codex | default | 0% | null | No expone reset_time |

**Conclusion:** H1 confirmada con datos reales del VPS. El `reset_time` aparece solo
cuando hay consumo y es independiente por pool. El ping automatico tiene sentido para
arrancar ventanas inactivas (percent_used < 5%).

**Config activa:**
- `cooldown_hypothesis = "H1"`
- `AKYV_PING_ENABLED = false` (default) — activar con `true` en .env del poller
- `AKYV_PING_MAX_PCT = 5.0` — solo pingea cuentas con menos del 5% de uso

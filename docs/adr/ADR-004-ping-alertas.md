# ADR-004 — Modelo de Consulta de Cuota y Ping Mínimo

**Fecha:** 2026-11-22
**Estado:** Aceptado (sujeto a actualización post-experimento de cooldown)
**Autor:** Arquitecto del proyecto

---

## Contexto

La lógica de alertas y del ping mínimo depende completamente del resultado del experimento de cooldown (H1/H2/H3). Este ADR documenta las decisiones de diseño para cada escenario posible, de modo que el código sea **configurable sin recompilar**.

---

## Decisión de diseño: configuración por pool

La lógica de ping y alertas se guarda en una tabla de configuración por proveedor/pool:

```json
// config/providers.json (en la bóveda, cifrado)
{
  "agy": {
    "pools": {
      "gemini-5h": {
        "cooldown_hypothesis": "PENDING",
        "window_hours": 5,
        "alert_warn_pct": 80,
        "alert_critical_pct": 95,
        "ping_enabled_by_default": false,
        "ping_message": "k",
        "ping_min_message_tokens": 1,
        "ping_max_per_day": 2,
        "ping_schedule": "08:00",
        "query_interval_active_min": 10,
        "query_interval_inactive_min": 60
      },
      "gemini-weekly": { ... },
      "3p-5h": { ... },
      "3p-weekly": { ... }
    }
  },
  "codex": {
    "pools": {
      "primary": { ... },
      "secondary": { ... }
    }
  }
}
```

Este objeto se actualiza cuando se confirma la hipótesis del experimento, sin recompilar.

---

## Comportamiento por hipótesis

### Si H1 (reloj inicia con el primer uso)

```
cooldown_hypothesis: "H1"
ping_automatic_makes_sense: true
```

- Ping automático (si está activado): enviar el mensaje mínimo a cuentas que no han tenido uso en la ventana actual para arrancar el reloj.
- Alerta del 95%: urgencia alta, pero hay tiempo — el reloj se reinicia X horas después del primer uso.
- UI: mostrar "ventana inicia desde tu primer uso del día".

### Si H2 (reloj inicia al llegar al 100%)

```
cooldown_hypothesis: "H2"
ping_automatic_makes_sense: false
```

- Ping automático: **deshabilitado** (no tiene sentido arrancar el reloj porque el reloj empieza al llegar al 100%).
- Si el usuario activa ping automático con H2, mostrar advertencia: "Con este tipo de ventana, el ping no adelanta el reinicio".
- Alerta del 95%: urgencia MÁXIMA — hay que cambiar de cuenta ahora en una pausa natural. Texto de alerta diferente: "⚠️ Estás al 95%. Si la tarea se corta, tendrás que reanudar cuando llegues al 100% y se reinicie."
- Alerta del 100%: notificación de que la ventana se reinició, con tiempo estimado (si H2 lo permite calcular).

### Si H3 (horario fijo/rodante)

```
cooldown_hypothesis: "H3"
ping_automatic_makes_sense: false
```

- El reloj corre independientemente del uso.
- Ping: sin sentido para el cooldown; puede usarse solo para verificar que el token sigue vivo.
- Alerta del 95%: igual que H1 (hay tiempo hasta el reset fijo).

---

## Guardas del ping

Independientemente de la hipótesis, el ping tiene:

1. **Consumo mínimo real:** el mensaje más corto posible (1-3 tokens).
2. **Límite diario por cuenta:** configurable, por defecto 2 pings/día.
3. **Horario:** solo ejecutar en el horario configurado (por defecto 08:00 hora local).
4. **Parada automática:** si la API devuelve error de rate limit o aviso de abuso, deshabilitar el ping automático para esa cuenta y notificar al usuario.
5. **Registro:** cada ping se registra en el historial de eventos con cuenta, hora, tokens usados y resultado.
6. **Apagado global:** un switch global en Ajustes que deshabilita todos los pings automáticos instantáneamente.

---

## Cómo se ejecuta el ping en agy

**Problema:** agy usa el Credential Manager de Windows como login global de la PC. Cambiar la cuenta activa solo para hacer un ping es intrusivo (afecta a todas las ventanas de agy abiertas).

**Opciones evaluadas:**

| Opción | Pros | Contras |
|--------|------|---------|
| A. Llamar al endpoint directamente con el token | No interfiere con agy en uso; independiente del estado del Credential Manager | Hay que extraer el access token de la bóveda; si el token expiró, hay que renovarlo sin Credential Manager |
| B. Cambiar Credential Manager, hacer ping, restaurar | Usa el flujo oficial de agy | Intrusivo — interrumpe agy si está en uso; riesgo de quedar con cuenta incorrecta si falla |
| C. Usar `agy` con `--credential-file` (si existe la opción) | Limpio si existe | No documentado; pendiente de verificar |

**Decisión: Opción A** (llamar al endpoint directamente con el token).

El adaptador de agy extrae el access token (descifrado de la bóveda, en memoria) y hace la petición HTTP directamente a `cloudcode-pa.googleapis.com`. Si el token ha expirado, usa el refresh token para renovarlo antes del ping.

**Aviso en el ADR:** esto es tráfico automatizado en cuentas personales. El ADR de ping debe documentarse en la bóveda como riesgo aceptado por el usuario.

---

## Consecuencias

- El módulo de alertas lee la configuración por pool y adapta el texto y la urgencia.
- El módulo de ping verifica la hipótesis configurada antes de decidir si ejecutar.
- Cuando se confirme la hipótesis del experimento, se actualiza el campo `cooldown_hypothesis` en la configuración — sin recompilar.
- Si en el futuro una hipótesis cambia (por actualización del proveedor), se puede cambiar desde la UI sin actualizar la app.

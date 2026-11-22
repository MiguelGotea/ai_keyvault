# ADR-003 — Modelo de Sincronización y Consulta de Cuota

**Fecha:** 2026-11-22
**Estado:** Aceptado
**Autor:** Arquitecto del proyecto

---

## Contexto

Varias PCs del usuario tienen AI KeyVault instalado. Todas comparten las mismas cuentas vía el VPS. La cuota de cada cuenta puede cambiar desde cualquier PC. El VPS también puede consultar cuota 24/7 (si el usuario delega explícitamente). Necesitamos:

1. Evitar consultas duplicadas (varias PCs consultando la misma cuenta al mismo tiempo).
2. Reconciliar lecturas de múltiples fuentes.
3. Mantener el VPS completamente ciego al contenido cifrado.

---

## Decisiones

### Sincronización de bóveda: CRDT simplificado (Last-Write-Wins por campo)

Para simplificar sin implementar un CRDT completo:

- Cada registro tiene `updated_at` (timestamp del cliente) y `device_id` (qué PC lo modificó).
- El servidor guarda la última versión por `record_id + field_type`.
- Si dos PCs modifican el mismo campo simultáneamente, gana el `updated_at` más reciente.
- En conflicto irreconciliable: la app muestra el conflicto al usuario (Fase 2+).

**Justificación:** para un usuario single-owner con ~30 cuentas, conflictos simultáneos son rarísimos. LWW es suficiente.

### Consulta de cuota: Lease/Lock por cuenta

Para evitar que varias PCs (y el VPS) consulten la misma cuenta al mismo tiempo:

```
Tabla en BD del servidor: quota_leases
─────────────────────────────────────
account_id  │ device_id │ locked_until  │ last_read_at │ source
─────────────────────────────────────
agy-01      │ pc-home   │ 2026-11-22T14:15:00Z │ 2026-11-22T14:05:00Z │ client
agy-01      │ vps       │ NULL (no tiene lease) │ 2026-11-22T13:50:00Z │ server
```

**Flujo de consulta:**
1. El cliente pide un lease para `account_id` con duración de 10 minutos.
2. El servidor otorga el lease solo si no hay otro lease activo.
3. El cliente consulta la cuota y sube la lectura.
4. El lease expira automáticamente.
5. Si el cliente muere antes de liberar el lease, expira solo (no hay bloqueo permanente).

**Intervalos:**
- Cuenta activa (usada en las últimas 2 horas): cada 10 minutos.
- Cuenta inactiva: cada 60 minutos.
- Ambos son configurables por el usuario.

### Delegación al VPS (opcional, explícita, por cuenta)

Para que el VPS consulte cuota aunque las PCs estén apagadas:

1. El usuario activa la delegación por cuenta desde la UI con un botón y un aviso de riesgo.
2. El refresh token de esa cuenta se cifra con la `SERVER_MASTER_KEY` (en `.env` del VPS, nunca en el repo).
3. El VPS puede renovar el access token y consultar la cuota.
4. El usuario puede **revocar la delegación** en cualquier momento — el VPS borra el token cifrado.
5. Si el VPS se compromete, solo quedan expuestas las cuentas con delegación activa.

**Aviso de riesgo (texto en la UI):**
> "Al activar la consulta remota, el VPS podrá renovar tokens de esta cuenta mientras tus PCs estén apagadas. Si el servidor se compromete, esta cuenta podría verse afectada. Las demás cuentas sin delegación no corren este riesgo."

### Reconciliación de lecturas

Cada lectura de cuota sube al servidor con:
```json
{
  "account_id": "agy-01",
  "pool": "gemini-5h",
  "tokens_used": 45000,
  "tokens_limit": 100000,
  "percent_used": 45.0,
  "reset_time": "2026-11-22T18:00:00Z",
  "read_at": "2026-11-22T14:05:00Z",
  "source": "pc-home"
}
```

La UI muestra: **"Actualizado hace 5 min por PC-Home"** o **"Actualizado hace 10 min por VPS"**.

Si hay lecturas de múltiples fuentes, se muestra la más reciente.

### Autenticación de dispositivos: Ed25519

- Cada PC genera un par de claves Ed25519 al registrarse.
- Las peticiones van firmadas con la clave privada.
- El servidor verifica la firma con la clave pública registrada.
- Para registrar una PC nueva: URL del servidor + usuario + clave maestra + (opcionalmente) aprobación desde otra PC ya registrada.

**Crate:** `ed25519-dalek`.

---

## Consecuencias

- El servidor necesita una tabla `quota_leases` y lógica de expiración.
- El cliente necesita renovar el lease antes de que expire si la consulta tarda.
- La delegación al VPS es un riesgo explícitamente aceptado por el usuario, documentado en el modelo de amenazas.
- La autenticación Ed25519 requiere que cada PC guarde su clave privada de forma segura (en el Credential Manager de Windows o en la caché DPAPI).

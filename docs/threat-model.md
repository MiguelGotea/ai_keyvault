# Modelo de Amenazas — AI KeyVault

**Fecha:** 2026-11-22
**Versión:** 1.0
**Metodología:** STRIDE simplificada

---

## Activos a proteger

| Activo | Criticidad | Descripción |
|--------|-----------|-------------|
| Clave maestra | 🔴 Crítica | La raíz de todo el cifrado |
| Vault Key | 🔴 Crítica | Cifra todos los registros |
| Clave de respaldo | 🔴 Crítica | Permite recuperar si se olvida la clave maestra |
| Tokens de sesión (access + refresh) | 🔴 Crítica | Acceso a las cuentas de los proveedores |
| Correos y contraseñas | 🟠 Alta | Credenciales de las cuentas |
| Tópico de ntfy | 🟡 Media | Permite suplantar alertas o silenciarlas |
| Claves Ed25519 de dispositivos | 🟡 Media | Permite suplantar una PC registrada |
| Metadatos de chats | 🟢 Baja | Privacidad, pero no dan acceso a nada |

---

## Escenario 1: VPS comprometido

**Qué puede hacer el atacante:**
- Leer todos los blobs cifrados en la BD SQLite.
- Leer las lecturas de cuota (% usado, fechas de reinicio).
- Leer los metadatos de chats (cifrados).
- Si hay delegaciones activas: renovar tokens de las cuentas delegadas y usarlas.

**Qué NO puede hacer:**
- Descifrar los blobs (no tiene la Vault Key ni la clave maestra).
- Ver correos, contraseñas o tokens no delegados.
- Escribir registros válidos (no tiene la clave privada Ed25519 de ningún dispositivo).

**Mitigaciones implementadas:**
1. Cifrado de conocimiento cero — el servidor solo ve IDs opacos y blobs.
2. Las delegaciones son explícitas y revocables.
3. La `SERVER_MASTER_KEY` (en `.env` del VPS) solo descifra refresh tokens delegados — y solo los de cuentas con delegación activa.
4. fail2ban + firewall + usuario no root + sandboxing systemd reducen la probabilidad de compromiso.

**Respuesta:**
1. Revocar todas las delegaciones desde cualquier PC con la clave maestra.
2. Cambiar la `SERVER_MASTER_KEY` y rotar todos los tokens delegados.
3. Los datos cifrados no delegados están seguros — no hace falta cambiar contraseñas.

---

## Escenario 2: PC comprometida

**Qué puede hacer el atacante:**
- Si la bóveda está desbloqueada: leer la Vault Key en memoria y descifrar todo.
- Leer el Credential Manager de Windows (tokens de agy).
- Leer `auth.json` de Codex.
- Leer la caché DPAPI (si existe y la PC está desbloqueada).

**Mitigaciones implementadas:**
1. Autobloqueo a los 15 minutos (configurable) — borra la Vault Key de memoria con `zeroize`.
2. La app corre con permisos de usuario normal (no administrador) — el atacante necesita acceso al mismo usuario de Windows.
3. Los logs nunca contienen secretos.
4. La caché DPAPI está ligada al usuario de Windows — un atacante necesita la sesión del usuario.

**Respuesta:**
1. Revocar el dispositivo comprometido desde otra PC.
2. Cambiar la clave maestra (invalida todas las sesiones existentes).
3. Cambiar los tokens de las cuentas de los proveedores (en cada proveedor directamente).

---

## Escenario 3: Repositorio filtrado (ai_keyvault)

**Qué puede hacer el atacante:**
- Leer todo el código fuente.
- Intentar encontrar secretos hardcodeados (detectado y bloqueado por `gitleaks` en CI).

**Qué NO puede hacer:**
- Encontrar secretos reales (si se han seguido las reglas).
- Acceder al VPS (las llaves SSH no están en el repo).
- Descifrar datos (la clave maestra no está en el repo).

**Mitigaciones implementadas:**
1. `gitleaks` en pre-commit y en CI — bloquea push con secretos.
2. `.gitignore` estricto (`.env`, `auth.json`, `*.key`, `*.pem`).
3. Ningún secreto en fixtures de tests (datos falsos obviamente falsos).
4. Repositorio privado (reduce exposición, aunque no es la única defensa).

**Si se detecta un secreto en el historial:**
1. Revocar inmediatamente el secreto en su proveedor.
2. Limpiar el historial de git (git filter-branch o BFG Repo Cleaner).
3. Forzar push (con precaución en rama main).

---

## Escenario 4: Repositorio filtrado (ai_keyvault_data)

**Qué puede hacer el atacante:**
- Descargar todos los snapshots cifrados.

**Qué NO puede hacer:**
- Descifrar los snapshots (no tiene la Vault Key ni la clave maestra).

**Mitigaciones implementadas:**
1. Los snapshots están cifrados con la clave pública del usuario.
2. La clave privada de descifrado nunca sale de las PCs del usuario.
3. El repositorio es privado.

---

## Escenario 5: Clave maestra olvidada

**Impacto:** el usuario no puede desbloquear la bóveda.

**Mitigación:**
1. La clave de respaldo (guardada en papel o lugar seguro) permite descifrar el wrapped VK alternativo.
2. Con el wrapped VK descifrado, el usuario puede cambiar la clave maestra a una nueva.
3. La clave de respaldo se muestra UNA SOLA VEZ al crear la bóveda — se recuerda al usuario que debe guardarla antes de continuar.

**Si se pierde TAMBIÉN la clave de respaldo:**
- Los datos están irrecuperables. Esto es intencional (cifrado real).
- Documentar claramente en la UI y en el README.

---

## Escenario 6: Laptop robada (PC con sesión activa)

**Si la bóveda está bloqueada:**
- El atacante solo ve la pantalla de desbloqueo.
- Sin la clave maestra, no puede desbloquear.
- La caché DPAPI está protegida por el usuario de Windows — necesita la contraseña de Windows.

**Si la bóveda está desbloqueada:**
- El atacante puede acceder a todos los datos desencriptados.
- Mitigación: autobloqueo a los 15 minutos (configurable a menos).
- Mitigación adicional: el usuario puede revocar el dispositivo desde otra PC.

**Respuesta:**
1. Revocar el dispositivo desde otra PC.
2. Cambiar la contraseña de Windows/BitLocker del laptop robado.
3. Cambiar la clave maestra de AI KeyVault.

---

## Escenario 7: Token de ntfy filtrado (tópico de notificaciones)

**Impacto:**
- El atacante puede enviar notificaciones falsas al celular del usuario.
- O silenciar las alertas reales (suscribiéndose y marcando como leídas).

**Mitigaciones:**
1. El tópico de ntfy es aleatorio y se guarda en la bóveda (cifrado), nunca en el repo.
2. Las notificaciones críticas también se muestran en la app de escritorio (canal redundante).
3. Si se sospecha filtración, cambiar el tópico en Ajustes (genera uno nuevo aleatoriamente).

---

## Superficie de ataque resumida

```
Internet
   │
   ├── 443/tcp (Caddy + TLS) → servidor VPS (contenido cifrado, no descifrable)
   │
PC del usuario
   ├── Credential Manager (Windows) → tokens de agy
   ├── ~/.codex/auth.json → token de Codex
   ├── Caché local cifrada DPAPI → Vault Key (solo con sesión Windows activa)
   └── Proceso ai-keyvault (RAM) → Vault Key en claro (solo cuando desbloqueado)
```

---

## No está en el modelo de amenazas (fuera de alcance)

- Ataques a los propios proveedores (Anthropic, OpenAI, Google) — fuera del control de esta herramienta.
- Ataques de ingeniería social al usuario — fuera del alcance técnico.
- Vulnerabilidades zero-day en Tauri/WebView2/Windows — mitigadas con actualizaciones.
- Ataques físicos avanzados (cold boot, DMA) — fuera del alcance para una herramienta personal.

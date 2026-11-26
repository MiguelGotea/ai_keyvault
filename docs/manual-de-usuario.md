# Manual de Usuario — AI KeyVault

Bienvenido a **AI KeyVault**, tu bóveda personal cifrada y tablero de control para administrar tus cuentas de **Antigravity (agy)** y **Codex** en una o varias computadoras.

---

## 1. Instalación y primer arranque

### ¿Cómo instalar o abrir la aplicación?
- **Con instalador (Windows):** descarga `AI KeyVault_<versión>_x64-setup.exe` (o `.msi`) desde GitHub Releases y ejecútalo. *(Si Windows muestra SmartScreen por no tener firma comercial, haz clic en **«Más información» → «Ejecutar de todas formas»**.)*
- **Versión portable:** descarga `AI-KeyVault-portable-v<versión>-win-x64.zip`, extrae todo en una carpeta y ejecuta **`AI KeyVault.exe`** (mantén `ai-keyvault-poller.exe` en la misma carpeta).

### Primera vez en una PC
Al abrir la app por primera vez verás dos opciones:
1. **Crear usuario nuevo:** si es tu primera PC, ingresa tu nombre, usuario y una **clave maestra** (mínimo 12 caracteres). Al terminar, guarda en un lugar seguro tu **clave de respaldo (Recovery Key)** por si algún día olvidas la clave maestra.
2. **Ya tengo cuenta (iniciar sesión desde otra PC):** si ya usas AI KeyVault en otro equipo, escribe la URL de tu servidor, tu usuario y tu clave maestra. La app descargará tu bóveda cifrada automáticamente.

> **Importante:** Tu clave maestra nunca sale de tu computadora sin cifrar. Si la olvidas y no tienes la clave de respaldo, nadie (ni el servidor) puede recuperar tus datos.

---

## 2. Barra lateral y atajos rápidos

En el menú izquierdo tienes acceso a todas las pantallas:
- **Inicio:** resumen de cuentas, cuota actual y cambio rápido de cuenta.
- **Alertas:** avisos de cuota alta, vencimiento de suscripción y sugerencias de cuenta alternativa.
- **Ping:** envío de ping mínimo para iniciar ventanas de cuota.
- **Chats:** historial de tus conversaciones recientes por cuenta.
- **Cuentas:** tu bóveda cifrada para agregar, editar, copiar credenciales o eliminar cuentas.
- **Mis dispositivos:** lista de PCs autorizadas con opción para revocar acceso.
- **Cooldown:** herramienta para medir tiempos reales de reinicio de cuota.
- **Estadísticas:** consumo de tokens, eficiencia, desperdicio y mapa de calor.
- **Configuración:** ajustes de servidor, alertas, seguridad, tema y respaldos.

En la parte inferior izquierda siempre verás:
- **Estado de conexión** (`Online` / `Offline`) y estado del **Poller**.
- **⌘ Comandos (`Ctrl+K`):** abre la paleta rápida desde cualquier pantalla para navegar, buscar una cuenta, copiar su correo, cambiar de cuenta o bloquear la bóveda usando solo el teclado.
- **🔒 Bloquear bóveda:** cierra inmediatamente la sesión local y pide de nuevo tu clave maestra.

---

## 3. Inicio («Mis cuentas»)

Es tu tablero principal:
- **Barra superior:** indica si el **Poller** (el monitor automático en segundo plano) está corriendo y hace cuánto envió la última lectura de cuota.
- **Botón `↻ Actualizar`:** refresca la lista de cuentas y sus porcentajes de consumo.
- **Lista de cuentas:** cada fila muestra el proveedor (`AGY` o `CODEX`), tu alias, el correo, una etiqueta de color con el **% de cuota usado** (verde = normal, amarillo = ≥80 %, rojo = ≥95 %) y la fecha del próximo reinicio.
- **`Detalles`:** abre la ventana completa de esa cuenta con **6 pestañas**:
  1. **Resumen:** proveedor, alias, correo, última lectura, fecha de fin de suscripción y fecha de registro.
  2. **Cuota y cooldown:** estado detallado de cada pool de modelos y tiempo restante para reinicio.
  3. **Estadísticas:** ritmo de consumo (*burn rate*) y proyección de agotamiento.
  4. **Chat:** sesiones recientes asociadas a esa cuenta.
  5. **Credenciales:** acceso rápido para copiar correo o contraseña.
  6. **Historial:** registro de cambios (*switch*) hacia o desde esa cuenta.
- **`⚡ Usar` (cambiar de cuenta en esta PC):**
  1. Escribe las credenciales de la cuenta seleccionada en Windows (Credential Manager para Antigravity o `~/.codex/auth.json` para Codex).
  2. **Detección de herramientas abiertas:** si tienes abierto Antigravity, VS Code o Codex CLI, la app **no los cierra**, pero te muestra un aviso indicándote qué herramientas debes reiniciar para que tomen la nueva cuenta.
  3. **Botón `↩ Deshacer`:** si te equivocaste de cuenta, haz clic en **`↩ Deshacer`** en el banner superior para restaurar al instante la cuenta que estaba activa antes.

---

## 4. Cuentas («Mi Bóveda»)

Aquí administras tus credenciales cifradas:

### Agregar una cuenta nueva (`+ Agregar cuenta`)
1. Elige el proveedor:
   - **Antigravity (`agy`):** escribe un alias corto (ej. `trabajo`, `Claude 1`) y haz clic en **«Iniciar login OAuth»**. Se abrirá el navegador para que inicies sesión en Google/Antigravity; al terminar, AI KeyVault captura y cifra automáticamente la sesión.
   - **Codex:** inicia sesión primero en Codex CLI en tu PC, escribe un alias y haz clic en **«Importar y guardar»** para capturar tu `~/.codex/auth.json`.
2. **Copiar correo o clave:** usa los botones **`📋 Correo`** y **`🔑 Clave`** para copiar al portapapeles. Por seguridad, el portapapeles se limpia automáticamente después de **30 segundos**.
3. **Editar (`✏️ Editar`):** te permite actualizar el alias, correo, contraseña opcional, detalles (plan, teléfono de recuperación), **fecha de fin de suscripción** (para recibir avisos 7 y 1 día antes) y notas.
4. **Eliminar (`🗑️ Eliminar`):** borra la cuenta de tu bóveda.

---

## 5. Alertas

Muestra los avisos importantes de todas tus cuentas:
- **Alertas de cuota (80 % / 95 %):** cuando una cuenta se acerca al límite, aparece una tarjeta con la cuenta sugerida que tenga menor uso en ese momento y el botón **`⚡ Cambiar a esa cuenta`** (también con opción **`↩ Deshacer`** si la usas).
- **Alertas de suscripción:** te avisa automáticamente cuando faltan **7 días o 1 día** para que venza la fecha de suscripción que configuraste en la cuenta.
- **Alertas de consulta / sesión vencida:** te avisa si el poller tuvo un error o si la sesión activa expiró.
- **Silenciar notificaciones:** usa los botones **`Silenciar 1 h`**, **`4 h`**, **`8 h`** o **`24 h`** para pausar temporalmente los avisos de Windows y `ntfy.sh` sin apagar el monitoreo. Puedes reactivarlas en cualquier momento con **`Reactivar`**.

---

## 6. Ping mínimo

Algunas cuentas solo inician su reloj de reinicio semanal/horario cuando reciben su primer uso:
- Aquí aparecen los pools que están en **0 %** o sin ventana iniciada.
- Haz clic en **`Hacer ping`** para enviar una consulta mínima y activar el contador.
- *(Nota: usa esta función con criterio, ya que representa tráfico automatizado.)*

---

## 7. Chats

Muestra los metadatos (título, fecha de última actividad e ID de sesión) de las conversaciones activas detectadas en tus PCs:
- Usa el selector de la esquina superior derecha para filtrar por cuenta.
- Es una vista de **solo lectura**: el contenido completo de tus conversaciones no se sube ni se modifica.

---

## 8. Mis dispositivos

Muestra todas las computadoras donde has iniciado sesión con tu cuenta de AI KeyVault:
- Cada dispositivo muestra su nombre/ID, si está **`Activo`** o **`Revocado`**, y la fecha de su última actividad.
- Si dejaste de usar una PC o la formateaste, haz clic en **`Revocar acceso`** para invalidar sus permisos en el servidor.

---

## 9. Cooldown («Experimento de cooldown»)

Herramienta para comprobar si el tiempo de reinicio real coincide con la hora reportada por el proveedor:
1. En **Pools actuales**, opcionalmente escribe una nota (por ejemplo, *"tras error 429"*) y haz clic en **`📸 Iniciar observación`** en el pool que quieras medir.
2. El registro pasará a **Bitácora de mediciones** con un cronómetro en curso.
3. Cuando la cuenta vuelva a estar disponible, haz clic en **`✅ Fin`** para calcular la **duración real** y la **desviación** respecto al `reset_time` estimado.

---

## 10. Estadísticas

Analiza tu consumo en ventanas de **7 días**, **14 días** o **30 días**:
- **Tarjetas superiores:** Tokens acumulados, eficiencia proyectada promedio y cuota desperdiciada estimada.
- **Tokens por modelo / pool:** desglose de consumo por cada grupo de modelos (`gemini-weekly`, `claude-gpt-weekly`, etc.).
- **Eficiencia y cuota desperdiciada al reset:** estima cuánto porcentaje de cada cuenta se aprovechará antes de su próximo reinicio.
- **Comparativa y Heatmap:** comparación visual entre cuentas y mapa de calor por día de la semana y hora.

---

## 11. Configuración

### Servidor VPS y Alertas
- **Nombre de esta PC:** identificador con el que esta computadora aparece en lecturas y dispositivos.
- **Intervalo de consulta:** cada cuántos segundos consulta la app.
- **Tópico de `ntfy.sh` y Notificaciones de Windows:** activa o configura tus avisos en PC y celular.

### Ping automático
- Configura la **hora local** y el **máximo de pings diarios por cuenta** si decides usar ping automático.

### Seguridad
- **Autobloqueo (minutos de inactividad):** si pones un número mayor a `0`, la bóveda se bloqueará sola tras ese tiempo sin actividad (`0` = desactivado).
- **Desbloqueo rápido (DPAPI):** permite abrir la bóveda sin teclear la clave maestra en esa PC *(úsalo solo si nadie más tiene acceso a tu sesión de Windows)*.
- **Iniciar con Windows (en bandeja):** abre AI KeyVault minimizado en la bandeja del sistema al encender tu PC.

### Apariencia y modo «Sin agente en esta PC»
- **Tema de la interfaz:** cambia al instante entre **🌙 Oscuro** y **☀️ Claro**.
- **Sin agente en esta PC (solo consulta):** actívalo si estás usando AI KeyVault en una computadora donde **no** tienes instalado Antigravity/Codex o donde no quieres ejecutar el poller ni modificar credenciales locales.

### Respaldo y sesión
- **Último respaldo automático:** muestra la fecha del último snapshot cifrado en `ai_keyvault_data`.
- **Sincronizar bóveda con VPS:** permite **Subir bóveda al VPS** o **Descargar bóveda del VPS** manualmente.
- **Cerrar sesión global en esta PC:** borra la sesión guardada en esta computadora para poder entrar con otro usuario (no borra tus datos del servidor).

> Recuerda hacer clic en **`Guardar cambios`** en la esquina inferior derecha después de modificar tus ajustes.

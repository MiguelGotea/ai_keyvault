# AI KeyVault — Prompt maestro



---

## 0. Cómo quiero que trabajes

Eres el arquitecto y desarrollador principal de **AI KeyVault**, una herramienta **personal** (cuentas propias, PCs propias, sin fines comerciales) para administrar muchas cuentas de agentes de IA de programación.

**Orden obligatorio de trabajo:**

1. Crea un `CLAUDE.md` con las reglas de este documento (seguridad, idioma, estructura, convenciones).
2. Ejecuta la **Fase 0 (investigación)** de la sección 6. **No escribas código de la aplicación todavía.**
3. Entrégame `PLAN.md` + ADRs (`docs/adr/`) con tus decisiones técnicas justificadas, y **detente a esperar mi "aprobado"**.
4. Después trabajas por fases (sección 12), con tests, commits pequeños y demos por fase.

**Reglas de investigación:**

- Usa subagentes en fila para evitar cortes por limite de tokens para investigar.
- Toda afirmación sobre APIs, endpoints o archivos locales **no documentados** debe tener evidencia en `docs/research/`: comando ejecutado, salida (sin secretos), URL, versión del programa, fecha. Marca cada hallazgo como `[VERIFICADO]` o `[SUPUESTO]`.
- No inventes endpoints ni formatos. Si no puedes verificar algo, dilo y diseña para que sea reemplazable.
- Todo lo de la sección 2 **ya está decidido**. Pregúntame solo si algo te bloquea de verdad.

---

## 1. Qué es AI KeyVault

Una "bóveda + tablero" para mis cuentas de Antigravity (`agy`), Codex, y después Claude, Gemini, OpenCode, Grok y otros. Permite:

- Guardar cada cuenta **una sola vez** y abrirla en cualquier PC donde instale la herramienta.
- Ver cuota, plan, reinicios y estadísticas de todas las cuentas.
- **Cambiar de cuenta yo mismo desde la herramienta** (sin que me pase scripts para correr en PowerShell).
- Recibir alertas fuertes antes de quedarme sin cuota.
- Ver mis chats activos de todas mis PCs donde esté instalada la herramienta.

Tengo unas **30 cuentas** (de pago y gratuitas, planes distintos), pero **no debe haber un límite fijo** de cuentas ni de planes.

---

## 2. Decisiones ya tomadas (no preguntar de nuevo)

| Tema                             | Decisión                                                                                                                                                                                                                                                                                                       |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tipo de app                      | **App de escritorio local en Windows** (todas mis PCs son Windows). Sin panel web.                                                                                                                                                                                                                             |
| Nombre                           | **AI KeyVault**                                                                                                                                                                                                                                                                                                |
| Idioma de la interfaz            | **100 % español**, con textos en archivos de i18n (preparado para otros idiomas)                                                                                                                                                                                                                               |
| Tema                             | **Claro y oscuro**, intercambiables                                                                                                                                                                                                                                                                            |
| Colores de marca                 | Principal `#064E3B` (seguridad, confianza). Secundario `#D6A84F` (valor, exclusividad, innovación)                                                                                                                                                                                                             |
| Referencia visual                | Bitwarden se ve moderno, pero quiero algo **más moderno y minimalista** (ver sección 9)                                                                                                                                                                                                                        |
| Stack                            | **Tú decides** (ver sección 4). Prioridad: pocas fricciones con credenciales y con Windows                                                                                                                                                                                                                     |
| Repos de GitHub                  | **2 repos privados**: `ai_keyvault` github.com/MiguelGotea/ai_keyvault (código de la app y del servidor, **sin ningún secreto**) y `ai_keyvault_data` github.com/MiguelGotea/ai_keyvault_data (solo respaldo cifrado versionado)                                                                               |
| Rol de GitHub                    | Solo versionado, respaldo y **deploy con GitHub Actions** al VPS                                                                                                                                                                                                                                               |
| Almacenamiento vivo de la bóveda | **API propia en mi VPS (DigitalOcean)**, con dominio/subdominio propio (Spaceship), para conectarse al VPS y revisar la estructura de tdos los proyectos que existen actualmente usa el MCP ssh-mcp, hay que tener muy aislado este proyecto dentro del VPS para no depender de otros proyecots ni que choquen |
| Consulta de cuota                | **Ambos modos**: cada PC consulta y publica, y el VPS puede consultar 24/7 (ver 5.2 sobre el riesgo)                                                                                                                                                                                                           |
| Frecuencia de consulta           | **10 min** para cuentas activas, **60 min** para inactivas (configurable)                                                                                                                                                                                                                                      |
| Contraseñas                      | Guardar **correo + contraseña cifrados**. El 2FA se queda en mi app de autenticación                                                                                                                                                                                                                           |
| Tokens de sesión                 | **Sí se guardan cifrados** en la bóveda. Me logueo una vez con la herramienta y nunca más; en otra PC abro la cuenta ya guardada                                                                                                                                                                               |
| Claves                           | **Clave maestra + clave de respaldo** (estilo clave de recuperación de Windows)                                                                                                                                                                                                                                |
| Autobloqueo                      | Sí, configurable (por defecto 15 min)                                                                                                                                                                                                                                                                          |
| Chats de otras PCs               | **Solo metadatos**, subidos por un **agente que se instala junto con la app** en cada PC. Una PC sin la herramienta instalada no aporta chats                                                                                                                                                                  |
| Solo chats activos               | **No archivados**                                                                                                                                                                                                                                                                                              |
| Cambio de cuenta                 | **Global** (la cuenta activa de la PC). Solo **avisa** que hay que reiniciar las ventanas/herramientas que la usan; **nunca cierra procesos**                                                                                                                                                                  |
| Alertas                          | Notificación de Windows + **ntfy.sh al celular** para el 95 %. Alertas previas (por ejemplo 80 %) con **sugerencia de cuenta alternativa**                                                                                                                                                                     |
| Ping de cooldown                 | **Manual por defecto**, con opción automática por cuenta (ver sección 8)                                                                                                                                                                                                                                       |
| Más proveedores                  | Claude, Gemini, OpenCode y Grok previstos desde el diseño (adaptadores)                                                                                                                                                                                                                                        |
| Entrega                          | **MVP por fases con tests**                                                                                                                                                                                                                                                                                    |
| Uso                              | Cuentas propias en PCs propias, sin fines de robar credenciales, tokens ni llaves                                                                                                                                                                                                                              |

**Alcance ético (déjalo escrito en `README` y `CLAUDE.md`):** uso personal con cuentas propias. La herramienta no evade bloqueos, no comparte cuentas con terceros, y la rotación y el ping son decisiones mías, configurables y con registro.

---

## 3. Principios de seguridad (no negociables)

1. **Ningún secreto en ningún repositorio.** Ni en el repo `ai_keyvault`, ni en tests, ni en fixtures, ni en logs, ni en commits. Las credenciales del servidor y del deploy viven solo en GitHub Actions Secrets y en el VPS.
2. **Cifrado del lado del cliente.** El servidor guarda solo texto cifrado. La clave que descifra nunca sale de mis PCs.
3. **Los repos privados no son una bóveda.** `ai_keyvault_data` solo recibe snapshots **ya cifrados**, por si el VPS se pierde.
4. Un hallazgo de secreto en el repo bloquea el CI.

---

## 4. Arquitectura (propón y justifica; esto es una guía, no una orden)

**Componentes:**

1. **App de escritorio (Windows)**: interfaz + núcleo seguro. Candidatos a evaluar con una matriz de decisión escrita: **Tauri 2** (núcleo en Rust + interfaz web), **.NET 9** (WinUI/WPF) y Electron. Inclinación inicial: Tauri 2, con núcleo en Rust (Credential Manager, cifrado, DPAPI) e interfaz en TypeScript. Justifica si eliges otro. Requisitos: arranque rápido, ejecutable único o instalador pequeño, poco consumo de RAM.
2. **Agente local** (viene con la app, un solo instalador): proceso en segundo plano / tarea programada de Windows que:
   - consulta la cuota,
   - lee los chats locales (solo metadatos),
   - envía alertas,
   - ejecuta los cambios de cuenta que yo pida,
   - arranca al iniciar sesión en Windows.
3. **Servidor en el VPS**: API pequeña (Rust/axum, Go o Node, decide tú), SQLite o Postgres, detrás de **Caddy** con TLS en un subdominio de mi dominio de Spaceship (si no es necesario asignarle al dominio solo usar el URL). **Sin interfaz web**, solo API. Despliegue con GitHub Actions (llave SSH restringida o equivalente), servicio `systemd` con sandboxing, usuario no root, firewall, fail2ban, backups.
4. **Adaptadores de proveedor** (`Provider` interface común): un módulo por agente (`agy`, `codex`, `claude`, `gemini`, `opencode`, `grok`). Agregar un proveedor nuevo = agregar un adaptador, sin tocar el resto.

**Interfaz común de un adaptador (mínima):**
`detectar_instalación`, `leer_login_actual`, `escribir_login(cuenta)`, `verificar_login`, `leer_cuota`, `leer_plan_y_vencimiento`, `listar_chats_activos`, `leer_uso_por_modelo`, `ping_mínimo`, `iniciar_login_guiado`.

**Estructura sugerida del repo (monorepo):**

```
ai_keyvault/
  apps/desktop/          # app de escritorio
  crates|packages/core/  # bóveda, cifrado, sincronización
  providers/{agy,codex,claude,gemini,opencode,grok}/
  server/                # API del VPS
  agent/                 # agente local
  docs/{research,adr}/
  .scripts/               # herramientas de desarrollo (sin secretos)
  .github/workflows/     # CI, escaneo de secretos, deploy, respaldo
  CLAUDE.md  PLAN.md  README.md  THIRD_PARTY_NOTICES.md
```

---

## 5. Modelo de seguridad detallado

### 5.1 Bóveda y claves

- **Clave maestra** → KDF **Argon2id** (parámetros justificados en un ADR) → clave de derivación.
- Una **Vault Key aleatoria** cifra todo el contenido; se guarda envuelta (wrapped) de **dos** formas: con la clave maestra y con la **clave de respaldo**.
- **Clave de respaldo**: generada al crear la bóveda, mostrada **una sola vez**, con formato legible (grupos de dígitos, estilo clave de recuperación de Windows) e impresión/exportación. Permite abrir la bóveda y cambiar la clave maestra si la olvido.
- Cifrado autenticado (XChaCha20-Poly1305 o AES-256-GCM), nonce por registro, AAD con id de registro y versión.
- Cifrado **por campo/registro**: correo, contraseña, tokens, notas, nombres de chats y metadatos de cuenta van cifrados. El servidor solo ve IDs opacos, tamaños y fechas de sincronización.
- Separar la **clave de autenticación** (la que se usa para entrar al servidor) de la **clave de cifrado** (nunca sale del cliente), como hace Bitwarden.

### 5.2 Servidor y modos de consulta

- **Modo por defecto: conocimiento cero.** El servidor no puede leer nada; solo almacena y sincroniza texto cifrado.
- **Consulta desde el VPS (opcional, por cuenta, con consentimiento explícito):** para que la cuota se actualice aunque mis PCs estén apagadas, el VPS necesita un token vivo. Diseña esto como **delegación explícita**: el refresh token de esa cuenta se cifra con una clave del servidor (fuera del repo, solo en el VPS), con aviso claro del riesgo, botón para **revocar la delegación**, y rotación de token. Si el VPS se compromete, solo quedan expuestas las cuentas delegadas.
- **Evitar consultas duplicadas:** si varias PCs y el VPS tienen la misma cuenta, usa un **lease/lock por cuenta** en el servidor para que solo uno consulte por intervalo. Las lecturas se reconcilian por marca de tiempo y fuente.
- Autenticación de dispositivos: cada PC se registra con su par de claves (Ed25519); las peticiones van firmadas. Límite de intentos y bloqueo progresivo.
- Entrar desde una PC nueva: **URL del servidor + usuario + clave maestra** (y opcionalmente aprobación desde un dispositivo ya registrado). Sin eso, no se descifra nada.

### 5.3 En cada PC

- Caché local cifrada. Opción de **DPAPI** para desbloqueo rápido por usuario de Windows, además de la clave maestra.
- Autobloqueo configurable. Al bloquear, se borran las claves de memoria (zeroize).
- **Portapapeles:** copiar correo/contraseña con un clic y **limpiar el portapapeles automáticamente** (por ejemplo a los 30 s).
- Los logs **nunca** contienen tokens, contraseñas ni correos completos.
- La app corre con permisos de **usuario normal**, no pide administrador. Documenta qué necesita y por qué.

### 5.4 Repositorios y CI

- `gitleaks` (o similar) en pre-commit y en CI; `.gitignore` estricto; Dependabot; `cargo audit`/`npm audit` en CI.
- Respaldo: un job en el VPS (o Action) sube **snapshots cifrados** a `ai_keyvault_data` con una **deploy key de solo escritura**. Define retención y cómo restaurar (con prueba automática de restauración).
- Entrega un **modelo de amenazas** (`docs/threat-model.md`): VPS comprometido, PC comprometida, repo filtrado, clave maestra olvidada, laptop robada.

---

## 6. FASE 0 — Investigación obligatoria (antes de diseñar el plan)

Guarda todo en `docs/research/` con evidencia. Los experimentos que toman tiempo **deben arrancar cuanto antes** (ver 6.1).

### 6.1 Experimento del cooldown (el más importante)

**Pregunta:** la ventana de tokens (por ejemplo 5 h) empieza a contar:

- **H1:** desde mi **primer uso** de la ventana,
- **H2:** desde el instante en que **consumo el 100 %**, o
- **H3:** en un horario fijo/rodante independiente de mi uso.

**Por qué importa:**

- Si es H1 → conviene un "ping mínimo" al inicio del día para arrancar el reloj, y una alerta del 95 % es menos crítica.
- Si es H2 → si una tarea larga se corta al llegar al 100 %, hay que reanudarla con el reinicio; por eso la alerta del 95 % debe ser **fuerte**, para cambiar de cuenta en una pausa natural. Y en H2 un "ping" no sirve de nada.

**Método (propuesto, mejóralo):**

1. Construye cuanto antes un **registrador mínimo** (`cooldown-logger`) que, para cada cuenta, guarde cada pocos minutos: marca de tiempo, `% usado`, `resetTime` de **cada ventana** (en agy: `gemini-5h`, `gemini-weekly`, `3p-5h`, `3p-weekly`; en Codex: ventana primaria y secundaria).
2. Usa cuentas que no he tocado en horas y registra: ¿hay `resetTime` antes del primer uso? ¿Cambia cuando uso por primera vez? ¿Se mueve con cada uso o queda fijo?
3. Ejecuta pruebas controladas con prompts mínimos y anota la relación entre `primer uso → resetTime`.
4. Repite para el pool Gemini y el pool 3P (Claude+GPT) de agy, para la ventana semanal, y para Codex.
5. Pídeme las acciones manuales que necesites (por ejemplo "usa esta cuenta con un mensaje corto ahora").
6. Entrega la conclusión por proveedor y pool: **H1, H2 o H3**, con las tablas de evidencia. **La lógica de alertas y de ping depende de este resultado**, así que se debe poder cambiar por configuración sin recompilar.

### 6.2 Antigravity (`agy`)

- Cuota: `loadCodeAssist` y `retrieveUserQuotaSummary` en `cloudcode-pa.googleapis.com` (usadas por `antigravity-swap`). Verifica en mi versión de agy.
- **Tokens consumidos y por modelo:** investiga si se pueden obtener (esquema de `~/.gemini/antigravity-cli/conversations/*.db`, `brain/`, logs, salida de `agy`) y **cómo lo hacen herramientas como "AgentRoom"** (u otras similares) que muestran consumo por modelo. Documenta qué es fiable y qué es estimación.
- Cómo marca agy un chat como activo vs archivado.
- Login en Windows: Credential Manager `gemini:antigravity` (global por máquina). Confirma el límite de tamaño y el formato del JSON.
- Plan de la cuenta y fecha de fin de suscripción (si existe forma de obtenerlos).

### 6.3 Codex

- `~/.codex/auth.json` (formato, rotación de tokens), `id_token` (plan, `account_id`, fecha de fin de suscripción).
- Endpoints de uso y perfil usados por `codex-switcher`: `/backend-api/wham/profiles/me`, límites primario y secundario, créditos. Verifica.
- Sesiones locales (`~/.codex/sessions`, archivadas, eventos de tokens) para chats, tokens por día y por modelo, y cómo se distinguen **activas vs archivadas**.

### 6.4 Claude, Gemini, OpenCode, Grok

- Dónde guardan credenciales y chats, cómo se consulta la cuota, y si hay herramienta de cambio de cuenta de referencia (`claude-swap`, `harness-swap`/`hswap`). Resultado esperado: una **tabla de compatibilidad** por proveedor y operación (listar, cambiar, cuota, chats, tokens), con lo no soportado marcado claramente.

### 6.5 Chat de consumo vs. chat de código (nueva incorporación)

Además de los agentes de código, cada proveedor tiene su **chat normal** (Gemini, ChatGPT, Claude), con cuota **distinta** a la de código. Investiga si se puede obtener **al menos lo básico**: % de uso y fecha de reinicio.

- Prefiere métodos que usan las propias apps oficiales y endpoints estables.
- **No** guardes cookies del navegador ni hagas scraping frágil sin avisarme. Si solo es posible así, documéntalo como "opcional / experimental" y déjalo para una fase posterior.
- Esta información va en **otra pestaña** dentro del detalle de cada cuenta.

### 6.6 Proyectos de referencia (licencias a verificar antes de reutilizar código)

- `hrmasss/antigravity-swap` (MIT): credenciales en Windows, cuota, verificación con `agy models`, tests con un `fake_agy`.
- `Lampese/codex-switcher` (MIT): cambio de `auth.json`, uso, estadísticas.
- `bashar94/codex-cli-account-switcher` (MIT).
- `realiti4/claude-swap` y `harness-swap`.

Si copias o adaptas código: conserva los avisos de copyright y registra todo en `THIRD_PARTY_NOTICES.md`. Verifica la licencia de **cada** dependencia.

### 6.7 Entregables de la Fase 0

`docs/research/*.md`, `docs/adr/*.md` (stack, cifrado, sincronización, modelo de consulta, ping), `docs/threat-model.md`, `PLAN.md` con fases, riesgos y criterios de aceptación. **Luego te detienes y esperas mi aprobación.**

---

## 7. Funcionalidades (mapeo de mis requisitos)

1. **Interfaz amigable** (sección 9).
2. **Lista de todas las cuentas** de todos los proveedores, con filtros (proveedor, plan, estado, PC) y búsqueda.
3. **Guardar correo y contraseña** para copiar y pegar rápido si se cierra una sesión.
4. **Créditos/cuota disponibles por cuenta**, con consulta programada. Si usé la cuenta en otra PC, debe actualizarse sola. Para agy se muestran **dos grupos**: cuota **Gemini** y cuota **3P (Claude + GPT)**. Para Codex, ventanas primaria y secundaria y créditos. Cada lectura muestra "actualizado hace X por PC-Y / VPS".
5. **Cooldown de modelos consumidos** (por modelo o pool, con cuenta regresiva).
6. **Historial de chats activos** de todas mis PCs con la herramienta instalada (sección 10).
7. **Fecha de reinicio** de cada ventana.
8. **Plan de la cuenta.**
9. **Estadísticas** (sección 11).
10. **Fecha fin de suscripción**, con alertas (por ejemplo 7 días y 1 día antes).
11. **Alertas de %** (sección 8).
12. **Herramienta de ping mínimo** (sección 8).
13. **Estructura por adaptadores** para más proveedores.
14. **Sin credenciales en GitHub.**
15. **Entrar rápido desde otra PC** con URL + clave maestra.
16. **Cambio de cuenta hecho por la herramienta** (sección 8.3).
17. **Multi-dispositivo**: pantalla de "Mis dispositivos" (nombre de PC, último contacto, revocar acceso).
18. **Pestaña de chat por cuenta** (6.5).

**Modelo de datos (borrador, afínalo):** `Cuenta`, `Credencial` (cifrada), `Lectura` (serie temporal: cuenta, ventana, % usado, reinicio, fuente, fecha), `Plan`, `Chat` (metadatos), `Dispositivo`, `Evento` (cambios, pings, alertas), `Configuración`.

---

## 8. Alertas, ping y cambio de cuenta

### 8.1 Alertas

- Umbrales configurables por cuenta/proveedor. Por defecto: **previa en 80 %** y **crítica en 95 %**.
- **Crítica (95 %)**: notificación persistente de Windows **y** **ntfy.sh** al celular con prioridad alta. El tópico de ntfy es **aleatorio y se guarda en la bóveda**, no en el repo.
- Cada alerta incluye una **sugerencia**: qué cuenta del mismo proveedor tiene más cuota libre (o cuál se reinicia antes), y botón "Cambiar a esa cuenta".
- Alertas adicionales: "cuenta X ya se reinició", fin de suscripción próximo, cuenta con login vencido o rechazado, consulta fallando varias veces.
- **El comportamiento depende del resultado del experimento 6.1** (ver "Por qué importa"). Haz que el texto y la urgencia se adapten al escenario H1/H2/H3 detectado.
- Anti-ruido: no repetir la misma alerta, con enfriamiento y "silenciar por X horas".

### 8.2 Ping mínimo (iniciar el cooldown)

- **Por defecto: manual.** Una pantalla "Cuentas que necesitan ping" lista las que cumplen los requisitos y cada una tiene un botón "Hacer ping".
- **Automático: opcional, por cuenta**, apagado por defecto. La regla depende del experimento 6.1:
  - Si **H1** (el reloj empieza con el primer uso): ping solo a cuentas **que no han iniciado uso en la ventana actual**.
  - Si **H2** (el reloj empieza al llegar al 100 %): el ping automático por inicio de día **no tiene sentido**; si se activa, solo aplica a cuentas que ya llegaron al límite, y debe explicarse.
  - Si **H3**: documentar y decidir según evidencia.
- **Guardas:** consumo mínimo real (el mensaje más corto posible), límite diario por cuenta, horarios configurables, registro de cada ping, parada automática ante errores o avisos de abuso.
- **Cómo se ejecuta:** evalúa llamar al servicio directamente con el token de la cuenta, o usar el CLI oficial con un directorio aislado (`CODEX_HOME` para Codex). En agy (login global en Windows) cambiar la cuenta activa solo para un ping es intrusivo: justifica la opción elegida y avísame.
- **Aviso de riesgo (documentar en el ADR):** el ping es tráfico automatizado en cuentas personales; los proveedores pueden tener políticas sobre uso automático. Debe poder apagarse por cuenta y globalmente.

### 8.3 Cambio de cuenta (global, hecho por la herramienta)

- La herramienta **ejecuta** el cambio; yo no corro scripts.
- **Codex:** sincroniza de vuelta el `auth.json` actual a su cuenta (por si Codex renovó tokens) y escribe el `auth.json` de la cuenta elegida de forma atómica. Respalda el anterior para poder deshacer.
- **agy en Windows:** escribe el login en Credential Manager (`gemini:antigravity`), verifica con `agy models` (no gasta cuota) y, si el login es rechazado, **revierte** y marca la cuenta en cuarentena.
- **Solo avisa** que debo reiniciar las ventanas o herramientas que usan la cuenta anterior (agy y otras, incluidas las mías propias). **Nunca cierra procesos.**
- Detecta procesos de `agy`/`codex` en ejecución para mostrar el aviso, sin tocarlos.
- Registro de cada cambio en el historial de eventos, con botón "Deshacer".
- Login guiado (una sola vez): la herramienta lanza el login oficial del proveedor, captura el resultado, lo cifra en la bóveda y valida. En otra PC, **abrir cuenta guardada** = escribir el login descifrado en esa PC.

---

## 9. Diseño de la interfaz

- **Nombre:** AI KeyVault. **Icono:** ya lo tengo generado. Lo dejare en la carpeta de este md; guárdalo en `assets/branding/` y deriva todos los tamaños (app, bandeja, instalador, favicon).
- **Colores:** principal `#064E3B`, secundario `#D6A84F`. Genera escalas (50–900) para ambos temas y define tokens CSS. **Verifica contraste WCAG AA**: el dorado se usa para acentos, bordes, iconos y estados destacados, **no** como texto sobre fondo claro.
- **Estilo:** minimalista y moderno, más cercano a Linear, Raycast, 1Password 8 o Vercel que a Bitwarden: mucho aire, tipografía clara (Inter o Geist), bordes sutiles, microanimaciones discretas, **atajos de teclado** y **paleta de comandos (Ctrl+K)** para "cambiar a cuenta…", "copiar contraseña de…", "ir a estadísticas".
- **Español 100 %**: sin textos en inglés en la interfaz, fechas y números con formato local.
- **Pantallas mínimas:**
  1. Desbloqueo (clave maestra / recuperación con clave de respaldo).
  2. **Panel**: tarjetas o tabla de cuentas con proveedor, plan, % usado (barras por ventana), próximo reinicio, estado y botón de cambio rápido.
  3. **Detalle de cuenta** con pestañas: *Resumen*, *Cuota y cooldown*, *Estadísticas*, *Chat (consumo del chat normal)*, *Credenciales*, *Historial*.
  4. **Estadísticas globales.**
  5. **Chats activos** (todas mis PCs).
  6. **Alertas** y configuración de umbrales.
  7. **Ping** (cuentas que lo necesitan).
  8. **Experimento de cooldown** (resultados por proveedor).
  9. **Mis dispositivos.**
  10. **Ajustes** (tema, autobloqueo, frecuencias, ntfy, copias de seguridad).
- La app debe sentirse **instantánea**: arranque rápido, datos en caché al abrir y actualización en segundo plano.

---

## 10. Historial de chats (solo metadatos, solo activos)

- El agente de cada PC lee los chats locales de cada herramienta y sube **solo metadatos cifrados**: proveedor, cuenta asociada (si se puede inferir), título, carpeta/proyecto, fecha de creación y última actividad, cantidad de mensajes o tokens si es fiable, **PC de origen** y estado.
- **Solo activos**: excluye archivados. Investiga cómo detectar ese estado en cada herramienta (6.2, 6.3).
- Una PC **sin** AI KeyVault instalada no aporta datos; la interfaz debe indicarlo ("sin agente en esta PC").
- El contenido de los chats **no se sube**. Muestra pistas útiles (por ejemplo, el comando para reanudar en esa PC).

---

## 11. Estadísticas (todas las que aporten seguimiento)

Como las APIs normalmente no entregan historial, **la herramienta guarda su propia serie temporal** de lecturas y eventos. Ideas mínimas:

- Consumo **por fecha**, por **modelo**, por **cuenta**, por **proveedor**, por **PC** y por **proyecto**; tokens de entrada/salida/en caché/razonamiento donde existan.
- **Ritmo de consumo** (burn rate) y **proyección**: "a este ritmo llegas al 100 % en X horas".
- **Mapa de calor** de horas y días con más uso.
- **Eficiencia**: tokens por % de cuota, y cuota **desperdiciada** al reiniciarse.
- Comparativa entre cuentas, planes y proveedores (cuál rinde más).
- Rachas, tarea más larga, picos diarios, modo de razonamiento más usado (lo que el proveedor entregue).
- **Línea de tiempo** de reinicios y cambios de cuenta.
- Aprovechamiento de planes: ¿una cuenta de pago casi sin uso?
- Informe semanal automático y **exportación CSV/JSON**.
- Cada estadística indica su **fuente y confiabilidad** (`oficial`, `derivada`, `estimada`).

---

## 12. Fases (MVP incremental, con tests)

Cada fase termina con: tests pasando, CI en verde, escaneo de secretos limpio, demo corta y **actualización de `PLAN.md`**.

| Fase  | Contenido                                                                                                                                         | Criterios de aceptación                                                                                         |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| **0** | Investigación, ADRs, modelo de amenazas, `cooldown-logger` (arranca ya)                                                                           | Documentos con evidencia; logger en marcha; espero mi "aprobado"                                                |
| **1** | Núcleo seguro: bóveda, Argon2id, clave de respaldo, cifrado por registro, autobloqueo, portapapeles                                               | Tests de cifrado/recuperación; tests de que nada sensible aparece en logs ni disco en claro; CI con `gitleaks`  |
| **2** | Servidor en VPS: API, sincronización, registro de dispositivos, deploy con Actions, respaldo cifrado a `ai_keyvault_data`, prueba de restauración | Entrar desde una segunda PC con URL + clave maestra y ver las mismas cuentas; servidor sin poder leer los datos |
| **3** | Adaptadores **agy + Codex**: login guiado, listar, cambiar cuenta, verificación, deshacer                                                         | Tests con un `fake_agy`/`fake_codex`; cambio real probado; sin tocar procesos                                   |
| **4** | Cuota y planes: consulta programada (10/60 min), lease anti-duplicado, panel y detalle                                                            | Datos coherentes entre PCs; lecturas con fuente y antigüedad                                                    |
| **5** | Alertas: Windows + ntfy.sh, previas, sugerencias, fin de suscripción                                                                              | Alertas de prueba; sin spam; ntfy sin tópico en el repo                                                         |
| **6** | Agente de chats (metadatos, solo activos) y vista multi-PC                                                                                        | Un chat creado en PC-A aparece en PC-B; archivado no aparece                                                    |
| **7** | Estadísticas e historial propio                                                                                                                   | Gráficas por fecha/modelo/cuenta; exportación                                                                   |
| **8** | Ping (manual + automático opcional) según resultado del experimento                                                                               | Guardas, registro y apagado global probados                                                                     |
| **9** | Proveedores nuevos (Claude, Gemini, OpenCode, Grok) y pestaña de chat de consumo                                                                  | Un adaptador nuevo sin cambios en el núcleo                                                                     |

**Pruebas obligatorias:** unitarias (cifrado, KDF, reconciliación de lecturas, reglas de alertas), de integración (adaptadores con programas falsos, servidor con base de datos temporal), e2e básicos de la interfaz, y una prueba de **restauración desde el respaldo**.

---

## 13. Calidad y entrega

- Código tipado, con linter y formato automáticos. Mensajes de commit convencionales.
- Instalador de Windows generado por **GitHub Actions** (indica el tema de firma de código y la advertencia de SmartScreen) y versión **portable** si es viable.
- Telemetría: **ninguna** hacia terceros.
- Manejo de errores claro y en español (token vencido, sin internet, agy no instalado, cuenta en cuarentena).
- Escalabilidad: 30 cuentas hoy, cientos mañana. Consultas con concurrencia limitada, reintentos con backoff y jitter.
- Documentación: `README.md` (instalación en una PC nueva paso a paso), `docs/operaciones.md` (VPS, deploy, respaldo, restauración, rotación de claves).

---

## 14. Lo que NUNCA debe llegar a los repositorios

Contraseñas, correos reales de mis cuentas, tokens de acceso o de refresco, `auth.json`, blobs de login de agy, claves maestra o de respaldo, tópico de ntfy, llaves SSH, `.env` reales, bases de datos o snapshots **sin cifrar**, capturas con datos reales, y logs con cualquiera de lo anterior. Los fixtures de tests usan **datos falsos obviamente falsos**.

---

## 15. Primer mensaje que espero de ti

1. Confirma que entendiste el alcance (3–5 líneas).
2. Crea `CLAUDE.md` y la estructura de `docs/`.
3. Lanza la investigación de la sección 6 (con subagentes en fila) y arranca el `cooldown-logger`.
4. Pídeme cualquier acción manual que necesites para los experimentos (por ejemplo, usar una cuenta concreta con un mensaje corto).
5. Cuando termines la Fase 0, entrégame `PLAN.md` + ADRs y **espera mi aprobación** antes de programar.



# 16. Referencias locales (Repositorios con proyectos funcionales dentro de la misma carpeta de este repositorio)

1. retails_nicaragua: repositorio y carpeta local de proyecto web con crons, base de datos y uso de dominio, tiene el archivo gitpush.ps1 bastante completo para actualziarcion a github y el archivo deploy.yml que se encarga de subirlo al VPS despues de  cada push

2. GMBWorker: servicio que vive en VPS solo como servicio con su respectivo gitpush y yml estadnar , igalmente este servicio esta aislado de todos los otros proyectos que viven en el VPS para no romeper nada si se elimina ni romper nada en la implementacion

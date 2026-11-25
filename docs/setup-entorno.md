# Entorno de Desarrollo y Herramientas — AI KeyVault (Windows)

**Fecha:** 2026-11-25 (actualizado tras completar segunda etapa — v0.2.0)
**Propósito:** Especificar de forma exhaustiva las herramientas, versiones, paquetes y variables de entorno necesarias para configurar y continuar el desarrollo de AI KeyVault en cualquier máquina Windows sin entrar en contradicciones ni romper dependencias.

> [!NOTE]
> Primera etapa (backend + poller) solo requiere Rust + MSYS2. Node.js y las herramientas de Tauri son para la **segunda etapa** (app de escritorio).

---

## 1. Herramientas Base del Sistema Operativo

| Herramienta | Versión / Canal | Método de instalación recomendado | Propósito |
|-------------|-----------------|-----------------------------------|-----------|
| **Git para Windows** | 2.40+ | `winget install --id Git.Git -e` | Control de versiones, subida a repositorios |
| **Node.js** | v20 LTS / v22 | `winget install --id OpenJS.NodeJS.LTS -e` | Toolchain del frontend Tauri / React |
| **Python** | 3.10+ (verificado 3.12) | `winget install --id Python.Python.3.12 -e` | Cooldown logger, token analyzer, scripts auxiliares |
| **Rustup & Cargo** | Rust 1.80+ (verificado 1.100.0) | `winget install --id Rustlang.Rustup -e` | Compilador y gestor de paquetes de Rust |
| **MSYS2 / MinGW-w64** | 2024+ (verificado) | `winget install -e --id MSYS2.MSYS2` | Linker GNU (`gcc`, `ld`, `dlltool`) para compilar Rust sin Visual Studio |

---

## 2. Configuración específica del Toolchain de Rust (Windows)

En Windows se utiliza el target **GNU** (`stable-x86_64-pc-windows-gnu`) para permitir compilaciones limpias y autónomas sin requerir la suite pesada de Visual Studio C++ Build Tools:

```powershell
# 1. Configurar toolchain por defecto
rustup toolchain install stable-x86_64-pc-windows-gnu --default
rustup default stable-x86_64-pc-windows-gnu

# 2. Instalar herramientas de calidad
rustup component add clippy rustfmt
```

---

## 3. Configuración de Linker y Herramientas GNU (MSYS2)

Una vez instalado MSYS2 en `C:\msys64`:

```powershell
# Desde PowerShell como Administrador o usuario normal con permisos:
& "C:\msys64\usr\bin\pacman.exe" -S --noconfirm mingw-w64-x86_64-binutils mingw-w64-x86_64-gcc
```

### Rutas en el PATH del sistema o sesión de PowerShell:
Para que `cargo` encuentre `gcc.exe` y `dlltool.exe`:
- `C:\msys64\mingw64\bin`
- `%USERPROFILE%\.cargo\bin`

En PowerShell, para la sesión actual:
```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:PATH
```

---

## 4. Dependencias de Python (Scripts y Loggers)

Para ejecutar `agent/cooldown-logger/cooldown_logger.py` y herramientas locales:

```bash
pip install requests
pip install pywin32 keyring  # Lectura segura de Credential Manager en Windows
```

---

## 5. Arquitectura del Monorepo y Componentes

```
ai_keyvault/
  ├── Cargo.toml            # Workspace raíz de Rust
  ├── crates/
  │   ├── core/             # Núcleo de criptografía (Argon2id, XChaCha20, Zeroize)
  │   └── poller/           # Agente de recolección de cuota (binario Windows)
  ├── server/               # API del servidor VPS (Axum + SQLite)
  ├── providers/
  │   ├── agy/              # Adaptador Antigravity (Credential Manager, chats, switch)
  │   └── codex/            # Adaptador Codex (auth.json, state_5.sqlite, chats, switch)
  ├── apps/
  │   └── desktop/          # [SEGUNDA ETAPA] Aplicación de escritorio Tauri 2
  └── docs/                 # Documentación y registros de arquitectura
```

---

## 6. Variables de entorno del Poller (ejecución local)

Para correr el poller en desarrollo o prueba manual:

```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:PATH
$env:AKYV_SERVER_URL   = "http://198.211.97.243:3010"   # URL del servidor
$env:AKYV_DEVICE_ID    = "TABLET-R5MH20CH"              # Nombre identificador de esta PC
$env:AKYV_POLL_INTERVAL = "600"                          # Segundos entre ciclos (default 600)
$env:AKYV_PING_ENABLED  = "true"                         # Habilitar ping automático
$env:AKYV_PING_HOUR     = "8"                            # Hora UTC para el ping
$env:AKYV_PING_MAX_PCT  = "5.0"                          # Solo hacer ping si cuota < 5%
$env:AKYV_NTFY_TOPIC    = "SECRETO"                      # Tópico de ntfy.sh (NUNCA en el repo)
$env:RUST_LOG           = "ai_keyvault_poller=info"

cargo run --package ai-keyvault-poller
```

---

## 7. Verificación de Salud del Entorno

En cualquier máquina nueva, ejecutar este comando en PowerShell para verificar que todo está listo:

```powershell
# Configurar PATH
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:PATH

# Comprobar versiones
cargo --version
rustc --version
gcc --version
node --version  # Solo necesario para segunda etapa (Tauri)
python --version

# Verificar workspace completo (sin correr tests)
cd C:\ruta\a\ai_keyvault
cargo check --workspace 2>&1

# Ejecutar todos los tests (con red, fuera del sandbox)
cargo test --workspace 2>&1

# Calidad de código
cargo fmt --all --check 2>&1
cargo clippy --all-targets -- -D warnings 2>&1
```

Si los tests pasan con `test result: ok`, el entorno está 100% operativo y listo para compilar.

---

## 8. Acceso al VPS (SSH)

```powershell
# Conexión directa (requiere la llave privada del deploy)
ssh ai-keyvault-deploy@198.211.97.243

# Verificar estado del servicio
ssh ai-keyvault-deploy@198.211.97.243 "systemctl status ai-keyvault --no-pager"

# Ver logs en tiempo real
ssh ai-keyvault-deploy@198.211.97.243 "journalctl -u ai-keyvault -f"

# Verificar endpoints
curl.exe http://198.211.97.243:3010/health
```

**Nota:** El acceso SSH al VPS también está disponible vía MCP `ssh-mcp` con connectionId `ai-keyvault-vps` para operaciones remotas desde el agente.

---

## 9. Herramientas adicionales para Segunda Etapa (Tauri 2)

> Instalar cuando se inicie la Fase 10 (app de escritorio).
> **Versiones verificadas en la PC de desarrollo:** pnpm 12.23.0, Tauri CLI 2.12.6, Node.js 24.18.0

### 9.1 Dependencias base para Tauri

| Herramienta | Versión mínima | Notas |
|-------------|---------------|-------|
| **Visual Studio Build Tools 2022** | 14.44+ | Cargar "Desarrollo de escritorio con C++". Incluye `cl.exe`, linker MSVC, CRT headers |
| **WebView2 Runtime** | Incluido en Windows 11 / 10 20H2+ | Si no está: Tauri lo descarga automáticamente en instalador (`downloadBootstrapper`) |
| **Node.js** | v20 LTS+ | Para el frontend React/Vite |
| **pnpm** | 12+ | Gestor de paquetes — más rápido que npm para monorepos |
| **Tauri CLI v2** | 2.x | Orquesta la compilación de Rust + Vite |

```powershell
# Instalar Visual Studio Build Tools (si no está)
# Descargar desde: https://visualstudio.microsoft.com/visual-cpp-build-tools/
# Cargar: "Desarrollo de escritorio con C++"

# Instalar Tauri CLI v2
cargo install tauri-cli --version "^2"

# Instalar pnpm
npm install -g pnpm

# Verificar
cargo tauri --version  # → tauri-cli 2.x.x
pnpm --version         # → 12.x.x
```

### 9.2 Dual Toolchain Rust: GNU (backend) + MSVC (desktop)

Este proyecto usa **dos toolchains Rust simultáneamente**:

| Contexto | Toolchain | Target | Dónde se usa |
|----------|-----------|--------|-------------|
| Backend / Poller (Linux cross-compile) | `stable-x86_64-pc-windows-gnu` (**default**) | `x86_64-pc-windows-gnu` | Raíz del repo, `crates/`, `providers/`, `server/` |
| App de escritorio Tauri | `stable-x86_64-pc-windows-msvc` | `x86_64-pc-windows-msvc` | `apps/desktop/src-tauri/` |

La selección automática funciona así: **ambos** `apps/desktop/rust-toolchain.toml` **y** `apps/desktop/src-tauri/rust-toolchain.toml` declaran `channel = "stable-x86_64-pc-windows-msvc"`. Cuando `cargo tauri build` se ejecuta desde `apps/desktop/`, Cargo lee el archivo raíz del directorio de trabajo y usa MSVC automáticamente. El resto del repo sigue con GNU.

> [!IMPORTANT]
> El `rust-toolchain.toml` debe existir en **DOS** ubicaciones: `apps/desktop/` y `apps/desktop/src-tauri/`. Si solo está en `src-tauri/`, `pnpm tauri build` ejecutado desde `apps/desktop/` usará el toolchain GNU del repo raíz y buscará el sidecar con sufijo `-gnu.exe` → error `resource path doesn't exist`. Verificado en producción.

```powershell
# Instalar el toolchain MSVC adicional (conservando GNU como default)
rustup toolchain install stable-x86_64-pc-windows-msvc
# Agregar el target explícitamente (requerido antes del primer build)
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc --toolchain stable-x86_64-pc-windows-msvc

# Verificar que GNU sigue siendo el default
rustup show
# "active toolchain: stable-x86_64-pc-windows-gnu"

# Verificar que MSVC está instalado
rustup toolchain list
# stable-x86_64-pc-windows-gnu (active, default)
# stable-x86_64-pc-windows-msvc
```

> [!IMPORTANT]
> **NUNCA** cambiar el toolchain por defecto a MSVC. El backend usa la libc GNU para cross-compilar a Linux. Si se cambia el default, el CI de deploy-server fallará.

### 9.3 Naming convention del sidecar

Tauri exige que el sidecar esté en `src-tauri/binaries/` con el triple de target exacto como sufijo:

```
apps/desktop/src-tauri/binaries/ai-keyvault-poller-x86_64-pc-windows-msvc.exe
```

En `tauri.conf.json` se referencia SIN el sufijo:
```json
"externalBin": ["binaries/ai-keyvault-poller"]
```

Tauri agrega automáticamente `-x86_64-pc-windows-msvc` al buscar el binario.

Script para compilar y copiar el sidecar:
```powershell
# Desde la raiz del repositorio

# Paso previo (una vez por maquina): agregar el target MSVC si no esta instalado
rustup target add x86_64-pc-windows-msvc

# Compilar el sidecar con MSVC
cargo build --release --package ai-keyvault-poller --target x86_64-pc-windows-msvc
.\.scripts\copy-sidecar.ps1
```

> [!IMPORTANT]
> `rustup target add x86_64-pc-windows-msvc` debe ejecutarse **antes** del primer
> `cargo build --target x86_64-pc-windows-msvc`. Sin este paso se obtiene
> `error[E0463]: can't find crate for 'std'`. Solo se necesita una vez por maquina.

### 9.4 Workspace Cargo aislado

`apps/desktop/src-tauri/Cargo.toml` declara su propio `[workspace]`. Esto significa:
- Tiene su propio `Cargo.lock` independiente del workspace raíz
- Tiene su propio `target/` (puede llegar a ser varios GB — normal)
- Compilar desde la raíz con `cargo build` **no** compila la app de escritorio
- Siempre compilar desde `apps/desktop/` con `pnpm tauri dev` o `pnpm tauri build`

### 9.5 pnpm approve-builds (primera vez)

En la primera instalación de pnpm en el proyecto, puede aparecer:
```
ERR_PNPM_IGNORED_BUILDS: esbuild@0.21.5 y esbuild@0.25.12 requieren aprobación
```

Solucion (una vez, desde `apps/desktop/`):
```powershell
cd apps\desktop
# En CI o entornos no interactivos, usar --all (evita prompt)
pnpm approve-builds --all
pnpm install
```

> [!IMPORTANT]
> En CI (GitHub Actions) usar **`pnpm approve-builds --all`** (sin interactividad).
> La version sin flag (`pnpm approve-builds`) abre un prompt que bloquea el pipeline.
> Tras ejecutarlo, `pnpm-lock.yaml` se actualiza y `--frozen-lockfile` pasara en CI.
> El archivo `apps/desktop/.npmrc` con `onlyBuiltDependencies[]=esbuild` esta
> incluido en el repo para que futuros `pnpm install` no requieran re-aprobar.


Los íconos de Tauri se generan con un script PowerShell desde las imágenes de branding:
```powershell
# Desde la raíz del repositorio
.\.scripts\gen-icons.ps1
```

Genera en `apps/desktop/src-tauri/icons/`:
- `32x32.png`, `128x128.png`, `128x128@2x.png` — íconos de la app
- `icon.ico` — ícono multi-tamaño para Windows
- `tray-icon.png` — ícono monocromo para la bandeja del sistema

### 9.7 Proceso completo de build de la app de escritorio

```powershell
# Desde la raíz del repositorio
# Script guía completo:
.\.scripts\build-fase10.ps1

# O paso a paso:

# 1. Frontend
cd apps\desktop
pnpm install
pnpm typecheck     # Verificar TypeScript

# 2. Compilar sidecar poller para MSVC
cd ..\..
cargo build --release --package ai-keyvault-poller --target x86_64-pc-windows-msvc
.\.scripts\copy-sidecar.ps1

# 3. Build Tauri (desde apps/desktop/)
cd apps\desktop
pnpm tauri build   # Genera .msi en src-tauri/target/release/bundle/msi/

# 4. Dev mode (sin sidecar — para probar solo la UI)
pnpm tauri dev
```

### 9.8 Archivo .npmrc preconfigurado

`apps/desktop/.npmrc` esta incluido en el repo con:
```
onlyBuiltDependencies[]=esbuild
```

Esto evita que `pnpm install` falle por scripts no aprobados de `esbuild` en
instalaciones limpias. No es necesario correr `pnpm approve-builds` nuevamente
si el `.npmrc` y el `pnpm-lock.yaml` actualizado ya estan en el repo.

### 9.9 Estrategia de versionado

Al subir de version, actualizar **tres archivos juntos**:

| Archivo | Campo | Ejemplo |
|---------|-------|---------|
| `apps/desktop/src-tauri/Cargo.toml` | `version = "..."` | `0.2.0` |
| `apps/desktop/src-tauri/tauri.conf.json` | `"version": "..."` | `"0.2.0"` |
| `apps/desktop/package.json` | `"version": "..."` | `"0.2.0"` |

Convencion:
- **Patch** (`0.2.x`): correcciones de bugs, ajustes menores
- **Minor** (`0.x.0`): ronda completa de funcionalidades nuevas
- **Major** (`x.0.0`): cambio de arquitectura o ruptura de compatibilidad

Commit de release: `chore(release): bump a vX.Y.Z`

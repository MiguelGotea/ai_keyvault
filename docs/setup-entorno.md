# Entorno de Desarrollo y Herramientas — AI KeyVault (Windows)

**Fecha:** 2026-11-25 (actualizado al cierre de primera etapa)
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

> Instalar solo cuando se inicie la Fase 10 (app de escritorio).

```powershell
# Tauri CLI
cargo install tauri-cli --version "^2"

# Node packages para el frontend
npm install -g pnpm

# Verificar Tauri
cargo tauri --version
```

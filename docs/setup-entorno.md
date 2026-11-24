# Entorno de Desarrollo y Herramientas — AI KeyVault (Windows)

**Fecha:** 2026-11-23  
**Propósito:** Especificar de forma exhaustiva las herramientas, versiones, paquetes y variables de entorno necesarias para configurar y continuar el desarrollo de AI KeyVault en cualquier máquina Windows sin entrar en contradicciones ni romper dependencias.

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
  │   └── core/             # Núcleo de criptografía (Argon2id, XChaCha20, Zeroize)
  ├── server/               # API del servidor VPS (Axum + SQLite)
  ├── apps/
  │   └── desktop/          # Aplicación de escritorio Tauri 2
  ├── providers/            # Adaptadores para agy, codex, claude, gemini, etc.
  ├── agent/                # Agente local de Windows (chats, background task)
  └── docs/                 # Documentación y registros de arquitectura
```

---

## 6. Verificación de Salud del Entorno

En cualquier máquina nueva, ejecutar este comando en PowerShell para verificar que todo está listo:

```powershell
# Comprobar versiones
cargo --version
rustc --version
gcc --version
node --version
python --version

# Ejecutar pruebas del núcleo
cd c:\Users\User\Desktop\Sistema\PitayaWeb\ai_keyvault
cargo test --manifest-path crates/core/Cargo.toml
```

Si los tests pasan con `test result: ok`, el entorno está 100% operativo y listo para compilar.

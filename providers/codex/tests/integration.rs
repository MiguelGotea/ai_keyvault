//! Tests de integración real para el provider Codex.
//!
//! **Estos tests requieren:**
//! - Windows con Codex instalado y autenticado (`~/.codex/auth.json` existe)
//! - `auth.json` con un token válido (no vencido)
//!
//! **Cómo correr:**
//! ```powershell
//! $env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
//! cargo test --package ai-keyvault-provider-codex -- --include-ignored
//! ```
//!
//! Estos tests NO corren en CI (Linux) por estar marcados `#[ignore]`.

use ai_keyvault_provider_codex::auth::{default_auth_path, CodexAuth, CodexAuthManager};

// ─────────────────────────────────────────────────────────────────────────────
// Test I-1: Leer auth.json real de Codex
// ─────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "Requiere Windows + ~/.codex/auth.json con credenciales reales"]
fn test_leer_auth_json_real() {
    let path = default_auth_path();
    assert!(
        path.exists(),
        "~/.codex/auth.json debe existir: {}",
        path.display()
    );

    let manager = CodexAuthManager::new();
    let auth = manager.read().expect("Debe leer auth.json correctamente");

    assert!(
        !auth.access_token.is_empty(),
        "El access_token no debe estar vacío"
    );

    println!("✅ Email: {:?}", auth.email);
    println!(
        "✅ Token válido: {} ({}min restantes)",
        auth.is_valid(),
        auth.minutes_remaining().unwrap_or(0)
    );
    println!("✅ Ruta: {}", auth.source_path.display());
}

// ─────────────────────────────────────────────────────────────────────────────
// Test I-2: Verificar que el token no está vencido
// ─────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "Requiere Windows + ~/.codex/auth.json con token no vencido"]
fn test_codex_token_no_vencido() {
    let manager = CodexAuthManager::new();
    let auth = manager.read().expect("Debe leer auth.json");

    assert!(
        auth.is_valid(),
        "El token de Codex no debe estar vencido. Minutos restantes: {:?}",
        auth.minutes_remaining()
    );

    let mins = auth.minutes_remaining().unwrap_or(0);
    println!("✅ Token válido por {}min más", mins);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test I-3: Roundtrip atómico de auth.json (escribe y verifica, luego revierte)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "Requiere Windows + ~/.codex/auth.json — MODIFICA EL ARCHIVO TEMPORALMENTE"]
fn test_roundtrip_codex_auth_json() {
    let manager = CodexAuthManager::new();

    // 1. Leer original
    let original = manager.read().expect("Debe leer auth.json original");
    let original_token = original.access_token.clone();

    // 2. Escribir mismo contenido (simula un switch a sí mismo)
    let mismo_contenido = CodexAuth {
        email: original.email.clone(),
        access_token: original.access_token.clone(),
        id_token: original.id_token.clone(),
        refresh_token: original.refresh_token.clone(),
        expiry: original.expiry,
        source_path: original.source_path.clone(),
    };

    manager
        .write_atomic(&mismo_contenido)
        .expect("Escritura atómica debe funcionar con datos reales");

    // 3. Verificar que se puede leer después de escribir
    let despues_de_escribir = manager.read().expect("Debe leer tras escritura atómica");
    assert_eq!(
        despues_de_escribir.access_token, original_token,
        "El token debe ser el mismo tras roundtrip"
    );

    // 4. Revertir a partir del .bak
    manager.revert().expect("Revert debe restaurar el .bak");

    // 5. Verificar que el original está restaurado
    let restaurado = manager.read().expect("Debe leer tras revert");
    assert_eq!(
        restaurado.access_token, original_token,
        "El token restaurado debe ser el original"
    );

    println!("✅ Roundtrip completo sin pérdida de datos");
    println!("✅ Email tras restore: {:?}", restaurado.email);
}

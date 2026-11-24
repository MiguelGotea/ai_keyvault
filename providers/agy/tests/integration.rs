//! Tests de integración real para el provider agy.
//!
//! **Estos tests requieren:**
//! - Windows con Credential Manager activo
//! - agy instalado y autenticado (`agy models` debe responder)
//! - La credencial `gemini:antigravity` debe existir en Credential Manager
//!
//! **Cómo correr:**
//! ```powershell
//! $env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
//! cargo test --package ai-keyvault-provider-agy -- --include-ignored
//! ```
//!
//! Estos tests NO corren en CI (Linux) por estar marcados `#[ignore]`.

use ai_keyvault_provider_agy::credential::AgyCredentialManager;
use ai_keyvault_provider_agy::quota::{parse_usage_output, verify_token};

// ─────────────────────────────────────────────────────────────────────────────
// Test I-1: Leer credencial real de agy desde Credential Manager
// ─────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "Requiere Windows + Credential Manager con gemini:antigravity"]
fn test_leer_credencial_agy_real() {
    let cred = AgyCredentialManager::read()
        .expect("Debe leer la credencial de Credential Manager");

    assert!(
        !cred.access_token.is_empty(),
        "El access_token no debe estar vacío"
    );
    assert!(
        cred.access_token.starts_with("ya29."),
        "El access_token de agy debe comenzar con ya29., fue: {}",
        &cred.access_token[..8.min(cred.access_token.len())]
    );
    assert!(
        cred.email.is_some(),
        "El email debe estar presente en el blob"
    );
    assert!(
        !cred.email.as_deref().unwrap_or("").is_empty(),
        "El email no debe estar vacío"
    );

    println!("✅ Email detectado: {}", cred.email.as_deref().unwrap_or("?"));
    println!(
        "✅ Token válido: {} ({}min restantes)",
        cred.is_valid(),
        cred.minutes_remaining().unwrap_or(0)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test I-2: Verificar token con `agy models`
// ─────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "Requiere Windows + agy instalado y autenticado"]
fn test_verificar_agy_models() {
    let models = verify_token().expect("verify_token() debe funcionar con agy instalado");

    assert!(!models.is_empty(), "agy models debe devolver al menos 1 modelo");
    println!("✅ Modelos disponibles ({}): {:?}", models.len(), models);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test I-3: Parsear salida real de `agy /usage`
// ─────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "Requiere Windows + agy instalado y autenticado"]
fn test_parsear_usage_real() {
    // Ejecutar `agy /usage` real
    let output = std::process::Command::new("agy")
        .args(["/usage"])
        .output()
        .expect("agy debe estar instalado y en PATH");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("--- Salida de agy /usage ---\n{}", stdout);

    let readings = parse_usage_output(&stdout);
    assert!(
        readings.is_ok(),
        "Debe parsear la salida real de agy /usage: {:?}",
        readings.err()
    );

    let readings = readings.unwrap();
    assert!(
        !readings.is_empty(),
        "Debe haber al menos 1 lectura de cuota"
    );

    for r in &readings {
        println!(
            "✅ Pool {:?}: {:.2}% restante, reset_at: {:?}",
            r.pool, r.percent_remaining, r.reset_at
        );
    }
}

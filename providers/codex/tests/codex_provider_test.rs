//! Tests de provider Codex — usan fake_codex (sin auth.json real).

use ai_keyvault_provider_codex::auth::{CodexAuth, CodexAuthManager};
use ai_keyvault_provider_codex::switcher::{CodexSwitchResult, CodexSwitcher};
use std::path::PathBuf;
use tempfile::TempDir;

// Helper: crear un auth.json de prueba en un directorio temporal
fn setup_fake_auth(dir: &TempDir, access_token: &str, expiry_offset_secs: i64) -> PathBuf {
    let path = dir.path().join("auth.json");
    let expiry = chrono::Utc::now().timestamp() + expiry_offset_secs;
    let content = serde_json::json!({
        "tokens": {
            "access_token": access_token,
            "id_token": fake_jwt(access_token),
            "refresh_token": "refresh_fake",
            "token_type": "Bearer",
            "expiry_time": expiry
        }
    });
    std::fs::write(&path, serde_json::to_string_pretty(&content).unwrap()).unwrap();
    path
}

/// Genera un JWT falso con email en el payload (para tests).
fn fake_jwt(token_hint: &str) -> String {
    let header = base64_url_encode(br#"{"alg":"RS256","typ":"JWT"}"#);
    let email = format!(
        "{}@test.com",
        token_hint.replace("ya29.", "").get(..8).unwrap_or("test")
    );
    let payload =
        base64_url_encode(format!(r#"{{"email":"{}","exp":9999999999}}"#, email).as_bytes());
    format!("{}.{}.fakesig", header, payload)
}

fn base64_url_encode(input: &[u8]) -> String {
    // Codificación base64url sin padding (suficiente para tests)
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as u32;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as u32
        } else {
            0
        };
        out.push(TABLE[((b0 >> 2) & 0x3F) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < input.len() {
            out.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        }
        if i + 2 < input.len() {
            out.push(TABLE[(b2 & 0x3F) as usize] as char);
        }
        i += 3;
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Lectura de auth.json
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_leer_auth_json() {
    let dir = TempDir::new().unwrap();
    let path = setup_fake_auth(&dir, "ya29.test_token", 3600);

    let manager = CodexAuthManager::with_path(path);
    let auth = manager.read().expect("Debe leer auth.json correctamente");

    assert_eq!(auth.access_token, "ya29.test_token");
    assert!(auth.is_valid(), "Token con expiry +1h debe ser válido");
    assert!(auth.minutes_remaining().unwrap_or(0) > 50);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Escritura atómica de auth.json
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_escritura_atomica() {
    let dir = TempDir::new().unwrap();
    let path = setup_fake_auth(&dir, "token_original", 3600);

    let manager = CodexAuthManager::with_path(path.clone());

    // Leer el original
    let original = manager.read().unwrap();
    assert_eq!(original.access_token, "token_original");

    // Escribir nueva credencial
    let nueva = CodexAuth {
        email: Some("nuevo@test.com".to_string()),
        access_token: "token_nuevo".to_string(),
        id_token: Some(fake_jwt("token_nuevo")),
        refresh_token: Some("refresh_nuevo".to_string()),
        expiry: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        source_path: path.clone(),
    };
    manager
        .write_atomic(&nueva)
        .expect("Escritura atómica debe funcionar");

    // Verificar que el archivo fue actualizado
    let leido = manager.read().unwrap();
    assert_eq!(leido.access_token, "token_nuevo");

    // Verificar que el .bak existe (para revertir)
    let bak_path = path.with_extension("json.bak");
    assert!(
        bak_path.exists(),
        "El .bak debe existir después de la escritura atómica"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Revertir con .bak
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_revertir_auth_json() {
    let dir = TempDir::new().unwrap();
    let path = setup_fake_auth(&dir, "token_a", 3600);
    let manager = CodexAuthManager::with_path(path.clone());

    let token_b = CodexAuth {
        email: None,
        access_token: "token_b".to_string(),
        id_token: None,
        refresh_token: None,
        expiry: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        source_path: path.clone(),
    };
    manager.write_atomic(&token_b).unwrap();

    // Verificar que token_b está activo
    assert_eq!(manager.read().unwrap().access_token, "token_b");

    // Revertir
    manager
        .revert()
        .expect("Revert debe funcionar con .bak presente");

    // Verificar que token_a está restaurado
    assert_eq!(manager.read().unwrap().access_token, "token_a");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Cambio de cuenta exitoso (fake_codex: verificación siempre OK)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_switch_exitoso_con_fake_codex() {
    let dir = TempDir::new().unwrap();
    let path = setup_fake_auth(&dir, "token_cuenta_a", 3600);
    let switcher = CodexSwitcher::with_path(path.clone());

    let cuenta_b = CodexAuth {
        email: Some("cuenta_b@test.com".to_string()),
        access_token: "token_cuenta_b".to_string(),
        id_token: None,
        refresh_token: Some("refresh_b".to_string()),
        expiry: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        source_path: path.clone(),
    };

    // fake_codex: verificación siempre exitosa
    let result = switcher.switch(&cuenta_b, |_token| Ok(()));

    match result {
        CodexSwitchResult::Success => {
            // Verificar que el archivo fue actualizado
            let manager = CodexAuthManager::with_path(path);
            let leido = manager.read().unwrap();
            assert_eq!(leido.access_token, "token_cuenta_b");
        }
        other => panic!("Se esperaba Success, se obtuvo: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Cambio fallido con reversión (fake_codex: token rechazado)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_switch_fallido_revierte() {
    let dir = TempDir::new().unwrap();
    let path = setup_fake_auth(&dir, "token_original_valido", 3600);
    let switcher = CodexSwitcher::with_path(path.clone());

    let cuenta_mala = CodexAuth {
        email: None,
        access_token: "token_invalido".to_string(),
        id_token: None,
        refresh_token: None,
        expiry: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        source_path: path.clone(),
    };

    // fake_codex: token siempre rechazado
    let result = switcher.switch(&cuenta_mala, |_token| {
        Err("401 Unauthorized: token inválido".to_string())
    });

    match result {
        CodexSwitchResult::Reverted { reason } => {
            assert!(
                reason.contains("401"),
                "El motivo debe contener el error de autenticación"
            );
            // Verificar que el token original fue restaurado
            let manager = CodexAuthManager::with_path(path);
            let leido = manager.read().unwrap();
            assert_eq!(
                leido.access_token, "token_original_valido",
                "El token original debe haberse restaurado tras el fallo"
            );
        }
        other => panic!("Se esperaba Reverted, se obtuvo: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: Token vencido es detectado
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_token_vencido_es_detectado() {
    let dir = TempDir::new().unwrap();
    // expiry_offset_secs negativo = ya venció
    let path = setup_fake_auth(&dir, "token_vencido", -3600);
    let manager = CodexAuthManager::with_path(path);
    let auth = manager.read().unwrap();
    assert!(
        !auth.is_valid(),
        "Un token con expiry en el pasado debe reportarse como inválido"
    );
}

//! Tests de provider agy — usan fake_agy (sin Credential Manager real).

use ai_keyvault_provider_agy::quota::{parse_usage_output, AgyPool};

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Parsear salida de `agy /usage` con cuota parcial
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_con_cuota_parcial() {
    let fake_output = r#"
/usage
└ Models & Quota

  Account: test@gmail.com

GEMINI MODELS
  Models within this group: Gemini 3.8 Flash

  Weekly Limit Remaining
    [█████████████████████████░] 98.41%
    Refreshes in 167h 55m

CLAUDE AND GPT MODELS
  Models within this group: Claude Sonnet 4.6, Claude Opus 4.6, GPT-OSS 120B

  Weekly Limit Remaining
    [████████████████████████░░] 97.48%
    Refreshes in 167h 59m
"#;

    let readings = parse_usage_output(fake_output).expect("Debe parsear correctamente");
    assert_eq!(readings.len(), 2, "Deben existir 2 pools");

    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    assert!(
        (gemini.percent_remaining - 98.41).abs() < 0.01,
        "% Gemini incorrecto"
    );
    assert!(
        (gemini.percent_used - 1.59).abs() < 0.01,
        "% usado Gemini incorrecto"
    );
    assert!(
        gemini.reset_at.is_some(),
        "Gemini debe tener reset_at cuando hay consumo"
    );

    let claude = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt)
        .unwrap();
    assert!(
        (claude.percent_remaining - 97.48).abs() < 0.01,
        "% Claude incorrecto"
    );
    assert!(
        claude.reset_at.is_some(),
        "Claude debe tener reset_at cuando hay consumo"
    );

    // Verificar que el email se extrae correctamente
    assert_eq!(gemini.account_email.as_deref(), Some("test@gmail.com"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Parsear salida con cuota al 100% (recién reiniciada)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_cuota_completa() {
    let fake_output = r#"
  Account: test@gmail.com

GEMINI MODELS
  Weekly Limit Remaining
    [██████████████████████████] 100.00%
    Quota available

CLAUDE AND GPT MODELS
  Weekly Limit Remaining
    [██████████████████████████] 100.00%
    Quota available
"#;

    let readings = parse_usage_output(fake_output).expect("Debe parsear cuota al 100%");
    assert_eq!(readings.len(), 2);

    for r in &readings {
        assert!(
            (r.percent_remaining - 100.0).abs() < 0.01,
            "% debe ser 100.0"
        );
        assert!((r.percent_used - 0.0).abs() < 0.01, "% usado debe ser 0.0");
        assert!(
            r.reset_at.is_none(),
            "Sin reset_at cuando la cuota está completa"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Tiempo de reset se calcula correctamente
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_reset_at_es_futuro() {
    let fake_output = r#"
GEMINI MODELS
  Weekly Limit Remaining
    [██████████████████] 50.00%
    Refreshes in 10h 30m

CLAUDE AND GPT MODELS
  Weekly Limit Remaining
    [████████████████████████░] 95.00%
    Refreshes in 2h
"#;

    let readings = parse_usage_output(fake_output).expect("Debe parsear");
    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    let claude = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt)
        .unwrap();

    // reset_at debe ser una fecha en el futuro
    let now = chrono::Utc::now();
    if let Some(ref reset) = gemini.reset_at {
        let parsed: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::parse_from_rfc3339(reset).unwrap().into();
        assert!(parsed > now, "reset_at de Gemini debe estar en el futuro");
        // 10h 30m = 630 min ≈ 37800s
        let diff = (parsed - now).num_minutes();
        assert!(
            diff > 620 && diff <= 631,
            "Diferencia de tiempo incorrecta: {}min",
            diff
        );
    }

    if let Some(ref reset) = claude.reset_at {
        let parsed: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::parse_from_rfc3339(reset).unwrap().into();
        let diff = (parsed - now).num_minutes();
        assert!(
            diff > 110 && diff <= 121,
            "Diferencia de tiempo incorrecta para Claude: {}min",
            diff
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Parsing de blob de Credential Manager (fake_agy)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_credential_blob() {
    use ai_keyvault_provider_agy::credential::AgyCredentialManager;

    // Blob real (formato verificado en campo)
    let fake_blob = serde_json::json!({
        "token": {
            "access_token": "ya29.fake_token_for_test",
            "token_type": "Bearer",
            "refresh_token": "1//fake_refresh",
            "expiry": "2099-12-31T23:59:59+00:00",
            "expires_in": 3599
        },
        "auth_method": "oauth2",
        "id_token_email": "test@gmail.com",
        "writer_id": "test-writer"
    });

    // Simular lectura del blob (acceso interno via método de test)
    // Usamos el helper público parse_blob_from_bytes
    let blob_bytes = serde_json::to_vec(&fake_blob).unwrap();
    let cred = AgyCredentialManager::parse_blob_for_test(&blob_bytes)
        .expect("Debe parsear el blob correctamente");

    assert_eq!(cred.access_token, "ya29.fake_token_for_test");
    assert_eq!(cred.email.as_deref(), Some("test@gmail.com"));
    assert!(cred.is_valid(), "Token con expiry 2099 debe ser válido");
    assert!(
        cred.minutes_remaining().unwrap_or(0) > 100_000,
        "Debe tener muchos minutos restantes"
    );
}

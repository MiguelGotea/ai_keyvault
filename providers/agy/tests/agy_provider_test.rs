//! Tests de provider agy — usan salida fake del formato real verificado.
//!
//! Formato real verificado (2026-11-25):
//! ```text
//! Quota:
//! Gemini Models          Weekly Limit Remaining     100%  2026-12-01 19:19 CST
//! Claude and GPT models  Weekly Limit Remaining     49%   2026-12-01 18:57 CST
//! Claude and GPT models  Five Hour Limit Remaining  49%   2026-11-24 23:57 CST
//! ```

use ai_keyvault_provider_agy::quota::{parse_usage_output, AgyPool};

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Parsear salida con cuota parcial — formato local con 3 pools
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_con_cuota_parcial() {
    let fake_output = "\
Quota:
Gemini Models          Weekly Limit Remaining     98%   2026-12-01 19:19 CST
Claude and GPT models  Weekly Limit Remaining     62%   2026-12-01 18:57 CST
Claude and GPT models  Five Hour Limit Remaining  62%   2026-11-24 23:57 CST
";

    let readings = parse_usage_output(fake_output).expect("Debe parsear correctamente");
    assert_eq!(readings.len(), 3, "Deben existir 3 pools (gemini-weekly, claude-gpt-weekly, claude-gpt-5h)");

    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    assert!(
        (gemini.percent_remaining - 98.0).abs() < 0.01,
        "% Gemini incorrecto: {}",
        gemini.percent_remaining
    );
    assert_eq!(gemini.window, "weekly", "Gemini debe ser ventana weekly");
    assert!(
        gemini.reset_at.is_some(),
        "Gemini debe tener reset_at cuando hay consumo"
    );

    let claude_weekly = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt && r.window == "weekly")
        .unwrap();
    assert!(
        (claude_weekly.percent_remaining - 62.0).abs() < 0.01,
        "% Claude weekly incorrecto: {}",
        claude_weekly.percent_remaining
    );

    let claude_5h = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt && r.window == "5h")
        .unwrap();
    assert!(
        (claude_5h.percent_remaining - 62.0).abs() < 0.01,
        "% Claude 5h incorrecto: {}",
        claude_5h.percent_remaining
    );
    assert_eq!(claude_5h.window, "5h", "Claude 5h debe tener window='5h'");

    // account_email se setea por el caller, no por parse_usage_output
    assert!(gemini.account_email.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Parsear salida con cuota al 100%
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_cuota_completa() {
    let fake_output = "\
Gemini Models          Weekly Limit Remaining     100%  2026-12-01 19:19 CST
Claude and GPT models  Weekly Limit Remaining     100%  2026-12-01 18:57 CST
Claude and GPT models  Five Hour Limit Remaining  100%  2026-11-24 23:57 CST
";

    let readings = parse_usage_output(fake_output).expect("Debe parsear cuota al 100%");
    assert_eq!(readings.len(), 3);

    for r in &readings {
        assert!(
            (r.percent_remaining - 100.0).abs() < 0.01,
            "% debe ser 100.0, got {}",
            r.percent_remaining
        );
        assert!(
            (r.percent_used - 0.0).abs() < 0.01,
            "% usado debe ser 0.0, got {}",
            r.percent_used
        );
        assert!(r.reset_at.is_some(), "reset_at debe estar presente");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Timestamp ISO 8601 se guarda como string válido
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_reset_at_es_iso8601_valido() {
    // Usar formato ISO (soportado como fallback)
    let fake_output = "Gemini Models\tWeekly Limit Remaining\t50%\t2026-12-02T00:32:25Z\nClaude and GPT models\tWeekly Limit Remaining\t95%\t2026-12-02T00:29:10Z\n";

    let readings = parse_usage_output(fake_output).expect("Debe parsear");
    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    let claude = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt)
        .unwrap();

    if let Some(ref reset) = gemini.reset_at {
        let parsed: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::parse_from_rfc3339(reset).unwrap().into();
        assert!(
            parsed > chrono::Utc::now(),
            "reset_at de Gemini debe estar en el futuro"
        );
        assert_eq!(reset, "2026-12-02T00:32:25Z");
    } else {
        panic!("Gemini debe tener reset_at");
    }

    if let Some(ref reset) = claude.reset_at {
        assert_eq!(reset, "2026-12-02T00:29:10Z");
    } else {
        panic!("Claude debe tener reset_at");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Timestamp local ("2026-12-01 19:19 CST") se guarda como string
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_reset_at_formato_local() {
    let fake_output = "\
Gemini Models          Weekly Limit Remaining     50%  2026-12-01 19:19 CST
Claude and GPT models  Five Hour Limit Remaining  80%  2026-11-24 23:57 CST
";

    let readings = parse_usage_output(fake_output).expect("Debe parsear");
    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    let claude = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt)
        .unwrap();

    assert_eq!(
        gemini.reset_at.as_deref(),
        Some("2026-12-01 19:19 CST"),
        "Debe guardar el timestamp local tal como viene"
    );
    assert_eq!(claude.window, "5h");
    assert!(claude.reset_at.is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Parsing de blob de Credential Manager (fake_agy)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_credential_blob() {
    use ai_keyvault_provider_agy::credential::AgyCredentialManager;

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

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: Salida vacía devuelve error
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_salida_vacia_devuelve_error() {
    let result = parse_usage_output("   \n\n  ");
    assert!(result.is_err(), "Salida vacía debe devolver error");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7: Líneas irrelevantes y header "Quota:" se ignoran
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_ignora_lineas_irrelevantes() {
    let fake_output = "\
Quota:
Some random header
Gemini Models          Weekly Limit Remaining  75%  2026-12-01 19:19 CST
Otro texto ignorado
Claude and GPT models  Weekly Limit Remaining  80%  2026-12-01 18:57 CST
Claude and GPT models  Five Hour Limit Remaining  80%  2026-11-24 23:57 CST
Final line
";

    let readings = parse_usage_output(fake_output).expect("Debe parsear 3 pools");
    assert_eq!(readings.len(), 3);
    assert!(readings.iter().any(|r| r.pool == AgyPool::Gemini));
    assert!(readings.iter().any(|r| r.pool == AgyPool::ClaudeGpt && r.window == "weekly"));
    assert!(readings.iter().any(|r| r.pool == AgyPool::ClaudeGpt && r.window == "5h"));
}

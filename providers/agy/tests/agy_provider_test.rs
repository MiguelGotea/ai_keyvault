//! Tests de provider agy — usan salida fake del nuevo formato TSV.
//!
//! Formato real verificado (2026-11-25):
//! ```text
//! Gemini Models   Weekly Limit Remaining  100%    2026-12-02T00:32:25Z
//! Claude and GPT models   Weekly Limit Remaining  62%     2026-12-02T00:29:10Z
//! ```

use ai_keyvault_provider_agy::quota::{parse_usage_output, AgyPool};

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Parsear salida con cuota parcial (formato TSV real)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_con_cuota_parcial() {
    let fake_output = "Gemini Models\tWeekly Limit Remaining\t98.41%\t2026-12-02T00:32:25Z\nClaude and GPT models\tWeekly Limit Remaining\t62%\t2026-12-02T00:29:10Z\n";

    let readings = parse_usage_output(fake_output).expect("Debe parsear correctamente");
    assert_eq!(readings.len(), 2, "Deben existir 2 pools");

    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    assert!(
        (gemini.percent_remaining - 98.41).abs() < 0.01,
        "% Gemini incorrecto: {}",
        gemini.percent_remaining
    );
    assert!(
        (gemini.percent_used - 1.59).abs() < 0.01,
        "% usado Gemini incorrecto: {}",
        gemini.percent_used
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
        (claude.percent_remaining - 62.0).abs() < 0.01,
        "% Claude incorrecto: {}",
        claude.percent_remaining
    );
    assert!(
        (claude.percent_used - 38.0).abs() < 0.01,
        "% usado Claude incorrecto: {}",
        claude.percent_used
    );
    assert!(
        claude.reset_at.is_some(),
        "Claude debe tener reset_at cuando hay consumo"
    );

    // account_email se setea por el caller (read_quota), no por parse_usage_output
    assert!(
        gemini.account_email.is_none(),
        "parse_usage_output no setea email (lo hace read_quota)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Parsear salida con cuota al 100%
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_cuota_completa() {
    let fake_output = "Gemini Models\tWeekly Limit Remaining\t100%\t2026-12-02T00:32:25Z\nClaude and GPT models\tWeekly Limit Remaining\t100%\t2026-12-02T00:29:10Z\n";

    let readings = parse_usage_output(fake_output).expect("Debe parsear cuota al 100%");
    assert_eq!(readings.len(), 2);

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
        // Con 100% el timestamp sigue presente (es cuando renueva la semana)
        assert!(r.reset_at.is_some(), "reset_at debe estar presente");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: El timestamp ISO se parsea como reset_at válido
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_reset_at_es_iso8601_valido() {
    let fake_output =
        "Gemini Models\tWeekly Limit Remaining\t50%\t2026-12-02T00:32:25Z\nClaude and GPT models\tWeekly Limit Remaining\t95%\t2026-12-02T00:29:10Z\n";

    let readings = parse_usage_output(fake_output).expect("Debe parsear");
    let gemini = readings.iter().find(|r| r.pool == AgyPool::Gemini).unwrap();
    let claude = readings
        .iter()
        .find(|r| r.pool == AgyPool::ClaudeGpt)
        .unwrap();

    // Los ISO timestamps deben parsearse como fechas válidas
    if let Some(ref reset) = gemini.reset_at {
        let parsed: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::parse_from_rfc3339(reset).unwrap().into();
        // Debe ser en el futuro respecto a ahora
        assert!(
            parsed > chrono::Utc::now(),
            "reset_at de Gemini debe estar en el futuro"
        );
        assert_eq!(reset, "2026-12-02T00:32:25+00:00");
    } else {
        panic!("Gemini debe tener reset_at");
    }

    if let Some(ref reset) = claude.reset_at {
        assert_eq!(reset, "2026-12-02T00:29:10+00:00");
    } else {
        panic!("Claude debe tener reset_at");
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
// Test 5: Salida vacía devuelve error
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_salida_vacia_devuelve_error() {
    let result = parse_usage_output("   \n\n  ");
    assert!(result.is_err(), "Salida vacía debe devolver error");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: Solo se parsean líneas Gemini/Claude (ignora otras)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_parse_usage_ignora_lineas_irrelevantes() {
    let fake_output = "Some random header\nGemini Models\tWeekly Limit Remaining\t75%\t2026-12-02T00:32:25Z\nOtro texto ignorado\nClaude and GPT models\tWeekly Limit Remaining\t80%\t2026-12-02T00:29:10Z\nFinal line\n";

    let readings = parse_usage_output(fake_output).expect("Debe parsear 2 pools");
    assert_eq!(readings.len(), 2);
    assert!(readings.iter().any(|r| r.pool == AgyPool::Gemini));
    assert!(readings.iter().any(|r| r.pool == AgyPool::ClaudeGpt));
}

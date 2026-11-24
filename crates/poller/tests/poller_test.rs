//! Tests del poller — sin red, sin providers reales.

use ai_keyvault_poller::models::{PollerConfig, QuotaSnapshot};
use chrono::Utc;

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: PollerConfig defaults correctos
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_poller_config_defaults() {
    let config = PollerConfig::new("http://localhost:3010", "pc-test");
    assert_eq!(config.server_url, "http://localhost:3010");
    assert_eq!(config.device_id, "pc-test");
    assert_eq!(config.poll_interval_secs, 600);
    assert_eq!(config.lease_duration_secs, 600);
}

#[test]
fn test_poller_config_with_interval() {
    let config = PollerConfig::new("http://localhost:3010", "pc-test").with_interval(300);
    assert_eq!(config.poll_interval_secs, 300);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: QuotaSnapshot se serializa correctamente a JSON
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_quota_snapshot_serialization() {
    let snapshot = QuotaSnapshot {
        account_id: "test@gmail.com".to_string(),
        provider: "agy".to_string(),
        pool: "gemini".to_string(),
        tokens_used: 15_900,
        tokens_limit: 1_000_000,
        percent_used: 1.59,
        percent_remaining: 98.41,
        reset_time: Some("2026-11-30T00:00:00+00:00".to_string()),
        source: "pc-test".to_string(),
        read_at: Utc::now(),
    };

    let json = serde_json::to_string(&snapshot).expect("Debe serializar a JSON");
    assert!(json.contains("test@gmail.com"));
    assert!(json.contains("agy"));
    assert!(json.contains("gemini"));
    assert!(json.contains("98.41"));

    // Verificar que deserializa de vuelta correctamente
    let back: QuotaSnapshot = serde_json::from_str(&json).expect("Debe deserializar");
    assert_eq!(back.account_id, "test@gmail.com");
    assert_eq!(back.provider, "agy");
    assert!((back.percent_remaining - 98.41).abs() < 0.001);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: QuotaSnapshot para Codex (pool "default", percent_remaining = 100 cuando válido)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_quota_snapshot_codex_token_valido() {
    let snapshot = QuotaSnapshot {
        account_id: "usuario@openai.com".to_string(),
        provider: "codex".to_string(),
        pool: "default".to_string(),
        tokens_used: 0,
        tokens_limit: 100,
        percent_used: 0.0,
        percent_remaining: 100.0,
        reset_time: Some("2026-11-25T10:00:00+00:00".to_string()),
        source: "pc-dev".to_string(),
        read_at: Utc::now(),
    };

    assert_eq!(snapshot.provider, "codex");
    assert_eq!(snapshot.pool, "default");
    assert!((snapshot.percent_remaining - 100.0).abs() < 0.001);
    assert!(snapshot.reset_time.is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Payload de upload tiene los campos requeridos por el servidor
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_upload_payload_tiene_campos_servidor() {
    let snapshot = QuotaSnapshot {
        account_id: "test@gmail.com".to_string(),
        provider: "agy".to_string(),
        pool: "claude_gpt".to_string(),
        tokens_used: 25_200,
        tokens_limit: 1_000_000,
        percent_used: 2.52,
        percent_remaining: 97.48,
        reset_time: None,
        source: "pc-test".to_string(),
        read_at: Utc::now(),
    };

    // Simular el body que construye uploader::upload_snapshots
    let body = serde_json::json!({
        "account_id": snapshot.account_id,
        "provider": snapshot.provider,
        "pool": snapshot.pool,
        "tokens_used": snapshot.tokens_used,
        "tokens_limit": snapshot.tokens_limit,
        "percent_used": snapshot.percent_used,
        "reset_time": snapshot.reset_time,
        "source": snapshot.source,
    });

    // Verificar que tiene todos los campos que espera el endpoint POST /api/quota/readings
    assert!(body["account_id"].is_string());
    assert!(body["provider"].is_string());
    assert!(body["pool"].is_string());
    assert!(body["tokens_used"].is_number());
    assert!(body["tokens_limit"].is_number());
    assert!(body["percent_used"].is_number());
    assert!(body["source"].is_string());

    assert_eq!(body["account_id"], "test@gmail.com");
    assert_eq!(body["pool"], "claude_gpt");
    assert_eq!(body["percent_used"], 2.52);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Payload de lease tiene los campos requeridos por el servidor
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_lease_payload_tiene_campos_servidor() {
    let config = PollerConfig::new("http://localhost:3010", "pc-test");
    let account_id = "test@gmail.com";

    let body = serde_json::json!({
        "account_id": account_id,
        "device_id": config.device_id,
        "lease_seconds": config.lease_duration_secs,
    });

    assert_eq!(body["account_id"], "test@gmail.com");
    assert_eq!(body["device_id"], "pc-test");
    assert_eq!(body["lease_seconds"], 600);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test I-1 (integración): Ciclo completo con servidor real
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
#[ignore = "Requiere servidor VPS en 198.211.97.243:3010 y agy instalado"]
async fn test_ciclo_completo_con_servidor_real() {
    use ai_keyvault_poller::collector;

    let snapshots = collector::collect_all("pc-test-integracion");
    println!("Snapshots recolectados: {}", snapshots.len());

    for s in &snapshots {
        println!(
            "  {} | {} | {} | {:.2}% usado | {}",
            s.account_id, s.provider, s.pool, s.percent_used, s.source
        );
    }

    // Con servidor real, verificar que el lease funciona
    let config = PollerConfig::new("http://198.211.97.243:3010", "pc-test-integracion");
    let client = reqwest::Client::new();

    if !snapshots.is_empty() {
        let account_id = &snapshots[0].account_id;
        let acquired = ai_keyvault_poller::lease::acquire(&client, &config, account_id)
            .await
            .expect("Lease debe funcionar");
        println!("Lease adquirido: {acquired}");
    }
}

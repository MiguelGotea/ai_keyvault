//! Tests de integración del alerter.
//!
//! Verifican cooldown, múltiples pools y defaults de AlertConfig.

use ai_keyvault_alerter::cooldown::CooldownStore;
use ai_keyvault_alerter::detector::detect_alerts;
use ai_keyvault_alerter::models::{AlertConfig, AlertLevel, AlertReading};
use std::path::PathBuf;

// ── helpers ──────────────────────────────────────────────────────────────────

fn reading(account: &str, pool: &str, pct: f64) -> AlertReading {
    AlertReading {
        account_id: account.to_string(),
        provider: "agy".to_string(),
        pool: pool.to_string(),
        percent_used: pct,
        reset_time: None,
    }
}

/// Config fija para tests (no usa #[cfg(test)] del crate — los integration
/// tests compilan como crate separado y no ven ítems cfg(test) de la lib).
fn test_config() -> AlertConfig {
    AlertConfig {
        warn_threshold: 80.0,
        crit_threshold: 95.0,
        cooldown_minutes: 60,
        ntfy_topic: None,
        ntfy_url: "https://ntfy.sh".to_string(),
        toast_enabled: false,
        state_path: PathBuf::from("/tmp/akyv-test-alert-state.json"),
    }
}

// ── tests de cooldown ─────────────────────────────────────────────────────────

#[test]
fn cooldown_suprime_segunda_alerta_igual() {
    let mut store = CooldownStore::default();
    let key = "a@b.com:gemini-weekly:Warn";

    // Primera vez: no está en cooldown
    assert!(!store.is_cooling_down(key, 60));

    // Marcar como alertado
    store.mark_alerted(key);

    // Segunda vez: sí está en cooldown
    assert!(store.is_cooling_down(key, 60));
}

#[test]
fn cooldown_no_afecta_claves_distintas() {
    let mut store = CooldownStore::default();

    store.mark_alerted("a@b.com:gemini-weekly:Warn");

    // Clave diferente (pool distinto) no debe estar en cooldown
    assert!(!store.is_cooling_down("a@b.com:claude-gpt-weekly:Warn", 60));
}

#[test]
fn cooldown_cero_minutos_siempre_permite() {
    let mut store = CooldownStore::default();
    let key = "a@b.com:gemini-weekly:Critical";

    store.mark_alerted(key);

    // Con cooldown de 0 minutos no se suprime
    assert!(!store.is_cooling_down(key, 0));
}

// ── tests de detector ─────────────────────────────────────────────────────────

#[test]
fn dos_cuentas_distintas_generan_alertas_independientes() {
    let config = test_config();
    let readings = vec![
        reading("cuenta1@gmail.com", "claude-gpt-weekly", 85.0),
        reading("cuenta2@gmail.com", "claude-gpt-weekly", 85.0),
    ];
    let events = detect_alerts(&readings, &config);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.level == AlertLevel::Warn));
}

#[test]
fn umbral_warn_exacto_genera_alerta() {
    let config = test_config(); // warn_threshold = 80.0
    let readings = vec![reading("a@b.com", "gemini-weekly", 80.0)];
    let events = detect_alerts(&readings, &config);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, AlertLevel::Warn);
}

#[test]
fn umbral_crit_exacto_genera_critico() {
    let config = test_config(); // crit_threshold = 95.0
    let readings = vec![reading("a@b.com", "claude-gpt-5h", 95.0)];
    let events = detect_alerts(&readings, &config);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, AlertLevel::Critical);
}

#[test]
fn bajo_umbral_no_genera_alerta() {
    let config = test_config();
    let readings = vec![reading("a@b.com", "gemini-weekly", 79.9)];
    let events = detect_alerts(&readings, &config);
    assert!(events.is_empty());
}

// ── tests de AlertConfig defaults ────────────────────────────────────────────
//
// Las variables de entorno son globales al proceso — si estos tests corren en
// paralelo se pisan. Un Mutex estático los serializa sin dependencias extras.

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn alert_config_defaults_correctos() {
    let _guard = ENV_MUTEX.lock().unwrap();

    std::env::remove_var("AKYV_WARN_THRESHOLD");
    std::env::remove_var("AKYV_CRIT_THRESHOLD");
    std::env::remove_var("AKYV_COOLDOWN_MINUTES");
    std::env::remove_var("AKYV_NTFY_TOPIC");
    std::env::remove_var("AKYV_TOAST_ENABLED");

    let config = AlertConfig::from_env();

    assert_eq!(config.warn_threshold, 80.0);
    assert_eq!(config.crit_threshold, 95.0);
    assert_eq!(config.cooldown_minutes, 60);
    assert!(config.ntfy_topic.is_none());
    assert!(config.toast_enabled);
}

#[test]
fn alert_config_lee_variables_de_entorno() {
    let _guard = ENV_MUTEX.lock().unwrap();

    std::env::set_var("AKYV_WARN_THRESHOLD", "70");
    std::env::set_var("AKYV_CRIT_THRESHOLD", "90");
    std::env::set_var("AKYV_COOLDOWN_MINUTES", "30");
    std::env::set_var("AKYV_TOAST_ENABLED", "false");

    let config = AlertConfig::from_env();

    assert_eq!(config.warn_threshold, 70.0);
    assert_eq!(config.crit_threshold, 90.0);
    assert_eq!(config.cooldown_minutes, 30);
    assert!(!config.toast_enabled);

    // Limpiar siempre — incluso si falla una aserción
    std::env::remove_var("AKYV_WARN_THRESHOLD");
    std::env::remove_var("AKYV_CRIT_THRESHOLD");
    std::env::remove_var("AKYV_COOLDOWN_MINUTES");
    std::env::remove_var("AKYV_TOAST_ENABLED");
}

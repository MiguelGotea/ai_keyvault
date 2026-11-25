//! Detector de umbrales de cuota.

use crate::models::{AlertConfig, AlertEvent, AlertLevel, AlertReading};
use chrono::Utc;

/// Evalúa las lecturas y devuelve los eventos de alerta activos.
pub fn detect_alerts(readings: &[AlertReading], config: &AlertConfig) -> Vec<AlertEvent> {
    let mut events = Vec::new();

    for r in readings {
        if r.percent_used <= 0.0 {
            continue;
        }

        let level = if r.percent_used >= config.crit_threshold {
            AlertLevel::Critical
        } else if r.percent_used >= config.warn_threshold {
            AlertLevel::Warn
        } else {
            continue;
        };

        events.push(AlertEvent {
            account_id: r.account_id.clone(),
            provider: r.provider.clone(),
            pool: r.pool.clone(),
            level,
            percent_used: r.percent_used,
            reset_time: r.reset_time.clone(),
            detected_at: Utc::now(),
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_reading(account: &str, pool: &str, pct: f64) -> AlertReading {
        AlertReading {
            account_id: account.to_string(),
            provider: "agy".to_string(),
            pool: pool.to_string(),
            percent_used: pct,
            reset_time: None,
        }
    }

    #[test]
    fn test_sin_alerta_bajo_umbral() {
        let config = AlertConfig::for_test();
        let readings = vec![make_reading("a@b.com", "gemini-weekly", 50.0)];
        let events = detect_alerts(&readings, &config);
        assert!(events.is_empty());
    }

    #[test]
    fn test_alerta_warn_80_pct() {
        let config = AlertConfig::for_test();
        let readings = vec![make_reading("a@b.com", "claude-gpt-weekly", 82.0)];
        let events = detect_alerts(&readings, &config);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, AlertLevel::Warn);
    }

    #[test]
    fn test_alerta_critica_95_pct() {
        let config = AlertConfig::for_test();
        let readings = vec![make_reading("a@b.com", "claude-gpt-5h", 97.5)];
        let events = detect_alerts(&readings, &config);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, AlertLevel::Critical);
    }

    #[test]
    fn test_multiples_pools_alertas_independientes() {
        let config = AlertConfig::for_test();
        let readings = vec![
            make_reading("a@b.com", "gemini-weekly", 10.0),
            make_reading("a@b.com", "claude-gpt-weekly", 85.0),
            make_reading("a@b.com", "claude-gpt-5h", 96.0),
        ];
        let events = detect_alerts(&readings, &config);
        assert_eq!(events.len(), 2);
        assert!(events.iter().any(|e| e.level == AlertLevel::Warn));
        assert!(events.iter().any(|e| e.level == AlertLevel::Critical));
    }

    #[test]
    fn test_ignora_lecturas_sin_consumo() {
        let config = AlertConfig::for_test();
        let readings = vec![make_reading("a@b.com", "gemini-weekly", 0.0)];
        let events = detect_alerts(&readings, &config);
        assert!(events.is_empty());
    }
}

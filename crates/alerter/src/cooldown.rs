//! Store de cooldown — evita alertas duplicadas.
//!
//! Persiste en `~/.ai-keyvault/alert-state.json` la última vez
//! que se disparó cada tipo de alerta. Si la alerta es más reciente
//! que el cooldown configurado, se suprime.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

/// Mapa de `clave → último_disparo`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CooldownStore {
    /// Clave: `"{account_id}:{pool}:{level}"`. Valor: timestamp UTC del último disparo.
    pub last_alerted: HashMap<String, DateTime<Utc>>,
}

impl CooldownStore {
    /// Carga el store desde disco. Si falla, devuelve store vacío.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persiste el store en disco. Crea el directorio si no existe.
    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("No se pudo crear directorio de estado: {e}");
                return;
            }
        }
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(path, text) {
                    warn!("No se pudo guardar estado de alertas: {e}");
                }
            }
            Err(e) => warn!("No se pudo serializar estado de alertas: {e}"),
        }
    }

    /// Devuelve `true` si la clave está dentro del periodo de cooldown.
    pub fn is_cooling_down(&self, key: &str, cooldown_minutes: i64) -> bool {
        match self.last_alerted.get(key) {
            Some(last) => {
                let elapsed = Utc::now() - *last;
                elapsed < Duration::minutes(cooldown_minutes)
            }
            None => false,
        }
    }

    /// Registra que se disparó la alerta para esta clave ahora.
    pub fn mark_alerted(&mut self, key: &str) {
        self.last_alerted.insert(key.to_string(), Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nueva_clave_no_enfriando() {
        let store = CooldownStore::default();
        assert!(!store.is_cooling_down("a@b.com:gemini-weekly:Warn", 60));
    }

    #[test]
    fn test_clave_recien_marcada_enfriando() {
        let mut store = CooldownStore::default();
        store.mark_alerted("a@b.com:gemini-weekly:Warn");
        assert!(
            store.is_cooling_down("a@b.com:gemini-weekly:Warn", 60),
            "Debe estar en cooldown recién marcada"
        );
    }

    #[test]
    fn test_clave_expirada_no_enfriando() {
        let mut store = CooldownStore::default();
        // Insertar timestamp antiguo (hace 2 horas)
        let old = Utc::now() - Duration::minutes(120);
        store
            .last_alerted
            .insert("a@b.com:claude-gpt-weekly:Critical".to_string(), old);
        assert!(
            !store.is_cooling_down("a@b.com:claude-gpt-weekly:Critical", 60),
            "Cooldown de 1h debe haber expirado"
        );
    }

    #[test]
    fn test_claves_independientes() {
        let mut store = CooldownStore::default();
        store.mark_alerted("a@b.com:gemini-weekly:Warn");
        // Otra clave no debe estar en cooldown
        assert!(!store.is_cooling_down("a@b.com:claude-gpt-weekly:Warn", 60));
    }
}

//! Mapa local cifrado correo → UUID opaco de la bóveda (D-02).
//!
//! El poller lee la cuenta activa del sistema operativo (por correo),
//! pero el VPS solo debe recibir el UUID opaco de la bóveda.
//! Este módulo mantiene el mapa `"{provider}:{email}" -> uuid` cifrado
//! con `XChaCha20-Poly1305` para que el poller pueda resolverlo incluso
//! cuando la UI está bloqueada (`VaultKey` borrada de memoria).

use crate::crypto::{EncryptedField, VaultKey};
use crate::error::{Result, VaultError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const IDENTITY_MAP_AAD: &str = "identity-map:v1";

/// Mapa local `"{provider}:{email_lowercase}" -> account_uuid`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityMap {
    pub entries: HashMap<String, String>,
}

impl IdentityMap {
    /// Crea un mapa vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construye la clave normalizada `"{provider}:{email}"`.
    fn make_key(provider: &str, email: &str) -> String {
        format!(
            "{}:{}",
            provider.trim().to_lowercase(),
            email.trim().to_lowercase()
        )
    }

    /// Inserta o actualiza la relación `(provider, email) -> account_uuid`.
    pub fn insert(&mut self, provider: &str, email: &str, account_uuid: &str) {
        if email.trim().is_empty() || account_uuid.trim().is_empty() {
            return;
        }
        self.entries
            .insert(Self::make_key(provider, email), account_uuid.trim().to_string());
    }

    /// Resuelve el UUID de bóveda para un `(provider, email)`.
    pub fn resolve(&self, provider: &str, email: &str) -> Option<&str> {
        self.entries
            .get(&Self::make_key(provider, email))
            .map(String::as_str)
    }

    /// Ruta del archivo de clave local asociado a `map_path`.
    pub fn key_path_for(map_path: &Path) -> PathBuf {
        map_path.with_extension("key")
    }

    /// Guarda el mapa cifrado en `map_path` usando la clave en `key_path_for(map_path)`.
    pub fn save_encrypted(&self, map_path: &Path) -> Result<()> {
        if let Some(parent) = map_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VaultError::EncryptionFailed(format!(
                    "No se pudo crear directorio del mapa de identidad: {e}"
                ))
            })?;
        }

        let key_path = Self::key_path_for(map_path);
        let key = load_or_create_key(&key_path)?;

        let json = serde_json::to_vec(self)
            .map_err(|e| VaultError::EncryptionFailed(format!("Error serializando mapa: {e}")))?;
        let encrypted = key.encrypt_field(&json, IDENTITY_MAP_AAD)?;
        let out = serde_json::to_vec_pretty(&encrypted)
            .map_err(|e| VaultError::EncryptionFailed(format!("Error serializando cifrado: {e}")))?;

        std::fs::write(map_path, out).map_err(|e| {
            VaultError::EncryptionFailed(format!("No se pudo escribir mapa cifrado: {e}"))
        })?;

        Ok(())
    }

    /// Carga y descifra el mapa desde `map_path`.
    /// Si `map_path` no existe todavía, devuelve un mapa vacío.
    pub fn load_encrypted(map_path: &Path) -> Result<Self> {
        if !map_path.exists() {
            return Ok(Self::new());
        }

        let key_path = Self::key_path_for(map_path);
        let key = load_key(&key_path)?;

        let data = std::fs::read(map_path).map_err(|_| VaultError::DecryptionFailed)?;
        let encrypted: EncryptedField =
            serde_json::from_slice(&data).map_err(|_| VaultError::DecryptionFailed)?;
        let plaintext = key.decrypt_field(&encrypted)?;
        let map: IdentityMap =
            serde_json::from_slice(&plaintext).map_err(|_| VaultError::DecryptionFailed)?;

        Ok(map)
    }
}

/// Ruta por defecto del mapa de identidad cifrado.
/// Prioriza `AKYV_IDENTITY_MAP_PATH` si está definida.
pub fn default_identity_map_path() -> PathBuf {
    if let Ok(p) = std::env::var("AKYV_IDENTITY_MAP_PATH") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }

    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata)
            .join("com.miguelgotea.ai-keyvault")
            .join("identity-map.enc");
    }

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".ai-keyvault")
        .join("identity-map.enc")
}

fn load_key(key_path: &Path) -> Result<VaultKey> {
    let bytes = std::fs::read(key_path).map_err(|_| VaultError::DecryptionFailed)?;
    if bytes.len() != 32 {
        return Err(VaultError::DecryptionFailed);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(VaultKey(arr))
}

fn load_or_create_key(key_path: &Path) -> Result<VaultKey> {
    if key_path.exists() {
        return load_key(key_path);
    }
    let key = VaultKey::generate();
    std::fs::write(key_path, key.0).map_err(|e| {
        VaultError::EncryptionFailed(format!("No se pudo guardar clave del mapa: {e}"))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_cifrado_con_datos_falsos() {
        let dir = std::env::temp_dir().join(format!("akv-idmap-{}", uuid::Uuid::new_v4()));
        let map_path = dir.join("identity-map.enc");

        let mut map = IdentityMap::new();
        map.insert(
            "agy",
            "test-fake-user@example.com",
            "00000000-0000-0000-0000-000000000001",
        );
        map.insert(
            "codex",
            "test-fake-user@example.com",
            "00000000-0000-0000-0000-000000000002",
        );

        map.save_encrypted(&map_path).expect("save_encrypted");

        // Verificar que el archivo en disco no contiene el correo en claro
        let raw = std::fs::read_to_string(&map_path).expect("read raw");
        assert!(!raw.contains("test-fake-user@example.com"));
        assert!(!raw.contains("00000000-0000-0000-0000-000000000001"));

        let loaded = IdentityMap::load_encrypted(&map_path).expect("load_encrypted");
        assert_eq!(
            loaded.resolve("agy", "TEST-FAKE-USER@example.com"),
            Some("00000000-0000-0000-0000-000000000001")
        );
        assert_eq!(
            loaded.resolve("codex", "test-fake-user@example.com"),
            Some("00000000-0000-0000-0000-000000000002")
        );
        assert_eq!(loaded.resolve("agy", "otro@example.com"), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

use crate::error::{Result, VaultError};
use data_encoding::BASE32_NOPAD;
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Clave de respaldo de 32 bytes (256 bits)
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct RecoveryKey(pub [u8; 32]);

impl RecoveryKey {
    /// Genera una nueva clave de recuperación aleatoria segura
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    /// Formatea la clave como texto legible de estilo Windows / BitLocker
    /// Ejemplo: AKVR-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX
    pub fn to_formatted_string(&self) -> String {
        let encoded = BASE32_NOPAD.encode(&self.0); // 52-53 caracteres Base32
        let mut formatted = String::from("AKVR");
        for chunk in encoded.as_bytes().chunks(4) {
            formatted.push('-');
            formatted.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        }
        formatted
    }

    /// Parsea una cadena formateada de clave de recuperación
    pub fn from_formatted_string(s: &str) -> Result<Self> {
        let clean: String = s
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_uppercase();

        let raw_b32 = if let Some(stripped) = clean.strip_prefix("AKVR") {
            stripped
        } else {
            &clean
        };

        let decoded = BASE32_NOPAD
            .decode(raw_b32.as_bytes())
            .map_err(|e| VaultError::InvalidRecoveryKey(e.to_string()))?;

        if decoded.len() != 32 {
            return Err(VaultError::InvalidRecoveryKey(format!(
                "Longitud de clave incorrecta: esperado 32 bytes, obtenido {}",
                decoded.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        Ok(Self(key))
    }
}

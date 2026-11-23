use crate::error::{Result, VaultError};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Vault Key (VK) aleatoria de 32 bytes que cifra todos los registros de la bóveda
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultKey(pub [u8; 32]);

impl VaultKey {
    /// Genera una nueva Vault Key aleatoria segura
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    /// Embalar / Cifrar la Vault Key con una clave de envoltura (ej. MDK o Recovery Key)
    pub fn wrap(&self, wrapping_key: &[u8; 32]) -> Result<WrappedKey> {
        let cipher = XChaCha20Poly1305::new(wrapping_key.into());
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, self.0.as_ref())
            .map_err(|_| VaultError::EncryptionFailed("No se pudo envolver la Vault Key".into()))?;

        Ok(WrappedKey {
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Desenvolver / Descifrar una Vault Key usando la clave correspondiente
    pub fn unwrap(wrapped: &WrappedKey, wrapping_key: &[u8; 32]) -> Result<Self> {
        let cipher = XChaCha20Poly1305::new(wrapping_key.into());
        let nonce = XNonce::from_slice(&wrapped.nonce);

        let mut decrypted = cipher
            .decrypt(nonce, wrapped.ciphertext.as_ref())
            .map_err(|_| VaultError::DecryptionFailed)?;

        if decrypted.len() != 32 {
            decrypted.zeroize();
            return Err(VaultError::DecryptionFailed);
        }

        let mut vk = [0u8; 32];
        vk.copy_from_slice(&decrypted);
        decrypted.zeroize();

        Ok(Self(vk))
    }

    /// Cifra un campo con AAD (Additional Authenticated Data)
    pub fn encrypt_field(&self, plaintext: &[u8], aad: &str) -> Result<EncryptedField> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad: aad.as_bytes(),
        };

        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|_| VaultError::EncryptionFailed("Fallo al cifrar campo".into()))?;

        Ok(EncryptedField {
            nonce: nonce_bytes,
            ciphertext,
            aad: aad.to_string(),
        })
    }

    /// Descifra un campo verificando su AAD
    pub fn decrypt_field(&self, encrypted: &EncryptedField) -> Result<Vec<u8>> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());
        let nonce = XNonce::from_slice(&encrypted.nonce);

        let payload = Payload {
            msg: encrypted.ciphertext.as_ref(),
            aad: encrypted.aad.as_bytes(),
        };

        cipher
            .decrypt(nonce, payload)
            .map_err(|_| VaultError::DecryptionFailed)
    }
}

/// Clave empaquetada (cifrada con MDK o Recovery Key)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WrappedKey {
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
}

/// Registro cifrado individual con nonce y AAD
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedField {
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    pub aad: String,
}

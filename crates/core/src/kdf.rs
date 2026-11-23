use crate::error::{Result, VaultError};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Sal aleatoria de 32 bytes para la KDF
pub const KDF_SALT_LEN: usize = 32;

/// Clave de autenticación para el servidor (32 bytes)
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct AuthKey(pub [u8; 32]);

/// Clave de derivación maestra para desempaquetar la Vault Key (32 bytes)
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterDerivationKey(pub [u8; 32]);

/// Parámetros configurables de Argon2id según ADR-002
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        // Según ADR-002: 64MB, 3 iteraciones, paralelismo 4
        Self {
            memory_kib: 64 * 1024,
            iterations: 3,
            parallelism: 4,
        }
    }
}

/// Deriva (AuthKey, MasterDerivationKey) a partir de la contraseña maestra y el salt
pub fn derive_keys(
    password: &str,
    salt: &[u8],
    params: &KdfParams,
) -> Result<(AuthKey, MasterDerivationKey)> {
    if salt.len() < 16 {
        return Err(VaultError::KdfError(
            "El salt debe tener al menos 16 bytes".into(),
        ));
    }

    let argon_params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(64), // 32 bytes AuthKey + 32 bytes MasterDerivationKey
    )
    .map_err(|e| VaultError::KdfError(e.to_string()))?;

    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut output = [0u8; 64];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|e| VaultError::KdfError(e.to_string()))?;

    let mut auth_bytes = [0u8; 32];
    let mut mdk_bytes = [0u8; 32];

    auth_bytes.copy_from_slice(&output[0..32]);
    mdk_bytes.copy_from_slice(&output[32..64]);

    output.zeroize();

    Ok((AuthKey(auth_bytes), MasterDerivationKey(mdk_bytes)))
}

/// Genera un nuevo salt aleatorio seguro de 32 bytes
pub fn generate_salt() -> [u8; KDF_SALT_LEN] {
    let mut salt = [0u8; KDF_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

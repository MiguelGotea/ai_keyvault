use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Error de derivación KDF: {0}")]
    KdfError(String),

    #[error(
        "Fallo de autenticación o descifrado criptográfico (datos corruptos o clave incorrecta)"
    )]
    DecryptionFailed,

    #[error("Error de cifrado: {0}")]
    EncryptionFailed(String),

    #[error("Formato de clave de respaldo inválido: {0}")]
    InvalidRecoveryKey(String),

    #[error("La bóveda está bloqueada. Debe desbloquearse primero")]
    VaultLocked,

    #[error("Error de serialización JSON: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Error de entrada/salida: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VaultError>;

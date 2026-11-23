pub mod crypto;
pub mod error;
pub mod kdf;
pub mod recovery;
pub mod vault;

pub use crypto::{EncryptedField, VaultKey, WrappedKey};
pub use error::{Result, VaultError};
pub use kdf::{derive_keys, AuthKey, KdfParams, MasterDerivationKey};
pub use recovery::RecoveryKey;
pub use vault::{EncryptedVaultState, Vault, VaultAccount};

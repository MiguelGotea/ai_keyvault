pub mod crypto;
pub mod error;
pub mod identity_map;
pub mod kdf;
pub mod recovery;
pub mod vault;

pub use crypto::{EncryptedField, VaultKey, WrappedKey};
pub use error::{Result, VaultError};
pub use identity_map::{default_identity_map_path, IdentityMap};
pub use kdf::{derive_keys, AuthKey, KdfParams, MasterDerivationKey};
pub use recovery::RecoveryKey;
pub use vault::{EncryptedVaultState, Vault, VaultAccount};

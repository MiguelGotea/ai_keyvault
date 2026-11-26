use crate::crypto::{EncryptedField, VaultKey, WrappedKey};
use crate::error::{Result, VaultError};
use crate::kdf::{derive_keys, generate_salt, AuthKey, KdfParams};
use crate::recovery::RecoveryKey;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;

/// Cuenta y credenciales cifradas almacenadas en la bóveda
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultAccount {
    pub id: Uuid,
    pub provider: String,
    pub alias: String,
    pub encrypted_email: EncryptedField,
    pub encrypted_password: Option<EncryptedField>,
    pub encrypted_tokens: Option<EncryptedField>,
    pub encrypted_notes: Option<EncryptedField>,
    #[serde(default)]
    pub encrypted_details: Option<EncryptedField>,
    #[serde(default)]
    pub subscription_ends_at: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Estado persistible o sincronizable de la Bóveda Cifrada (Conocimiento Cero)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedVaultState {
    pub version: u32,
    pub salt: Vec<u8>,
    pub kdf_params: KdfParams,
    pub wrapped_vault_key_master: WrappedKey,
    pub wrapped_vault_key_recovery: WrappedKey,
    pub accounts: Vec<VaultAccount>,
    pub updated_at: DateTime<Utc>,
}

/// Bóveda en memoria (desbloqueada o bloqueada)
pub struct Vault {
    state: EncryptedVaultState,
    unlocked_key: Option<VaultKey>,
}

impl Vault {
    /// Inicializa una nueva bóveda con contraseña maestra
    /// Devuelve la bóveda y la RecoveryKey (que se debe mostrar una única vez al usuario)
    pub fn create_new(
        password: &str,
        kdf_params: Option<KdfParams>,
    ) -> Result<(Self, RecoveryKey)> {
        let params = kdf_params.unwrap_or_default();
        let salt = generate_salt();

        let (_auth_key, mdk) = derive_keys(password, &salt, &params)?;
        let recovery_key = RecoveryKey::generate();
        let vault_key = VaultKey::generate();

        let wrapped_master = vault_key.wrap(&mdk.0)?;
        let wrapped_recovery = vault_key.wrap(&recovery_key.0)?;

        let state = EncryptedVaultState {
            version: 1,
            salt: salt.to_vec(),
            kdf_params: params,
            wrapped_vault_key_master: wrapped_master,
            wrapped_vault_key_recovery: wrapped_recovery,
            accounts: Vec::new(),
            updated_at: Utc::now(),
        };

        let vault = Self {
            state,
            unlocked_key: Some(vault_key),
        };

        Ok((vault, recovery_key))
    }

    /// Carga la bóveda desde su estado serializado (inicia bloqueada)
    pub fn load_locked(state: EncryptedVaultState) -> Self {
        Self {
            state,
            unlocked_key: None,
        }
    }

    /// Retorna una referencia al estado cifrado (listo para persistir o sincronizar)
    pub fn state(&self) -> &EncryptedVaultState {
        &self.state
    }

    /// Retorna una referencia mutable al estado cifrado (para modificaciones internas)
    pub fn state_mut(&mut self) -> &mut EncryptedVaultState {
        &mut self.state
    }

    /// Indica si la bóveda está desbloqueada en memoria
    pub fn is_unlocked(&self) -> bool {
        self.unlocked_key.is_some()
    }

    /// Bloquea la bóveda y borra de memoria la Vault Key con zeroize
    pub fn lock(&mut self) {
        if let Some(mut key) = self.unlocked_key.take() {
            key.0.zeroize();
        }
    }

    /// Desbloquea la bóveda con la contraseña maestra
    pub fn unlock_with_password(&mut self, password: &str) -> Result<AuthKey> {
        let (auth_key, mdk) = derive_keys(password, &self.state.salt, &self.state.kdf_params)?;
        let vk = VaultKey::unwrap(&self.state.wrapped_vault_key_master, &mdk.0)?;
        self.unlocked_key = Some(vk);
        Ok(auth_key)
    }

    /// Desbloquea la bóveda y permite re-establecer contraseña usando la clave de recuperación
    pub fn recover_with_key(
        &mut self,
        recovery_key: &RecoveryKey,
        new_password: &str,
    ) -> Result<()> {
        let vk = VaultKey::unwrap(&self.state.wrapped_vault_key_recovery, &recovery_key.0)?;

        // Re-derivar con nueva contraseña y nuevo salt
        let new_salt = generate_salt();
        let (_new_auth, new_mdk) = derive_keys(new_password, &new_salt, &self.state.kdf_params)?;

        let new_wrapped_master = vk.wrap(&new_mdk.0)?;
        let new_recovery = RecoveryKey::generate();
        let new_wrapped_recovery = vk.wrap(&new_recovery.0)?;

        self.state.salt = new_salt.to_vec();
        self.state.wrapped_vault_key_master = new_wrapped_master;
        self.state.wrapped_vault_key_recovery = new_wrapped_recovery;
        self.state.updated_at = Utc::now();

        self.unlocked_key = Some(vk);
        Ok(())
    }

    /// Agrega o actualiza una cuenta con sus campos cifrados individualmente
    pub fn add_account(
        &mut self,
        provider: &str,
        alias: &str,
        email: &str,
        password: Option<&str>,
        tokens_json: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Uuid> {
        let vk = self.unlocked_key.as_ref().ok_or(VaultError::VaultLocked)?;
        let account_id = Uuid::new_v4();
        let now = Utc::now();

        let aad_email = format!("{}:1:email", account_id);
        let enc_email = vk.encrypt_field(email.as_bytes(), &aad_email)?;

        let enc_password = if let Some(p) = password {
            let aad_p = format!("{}:1:password", account_id);
            Some(vk.encrypt_field(p.as_bytes(), &aad_p)?)
        } else {
            None
        };

        let enc_tokens = if let Some(t) = tokens_json {
            let aad_t = format!("{}:1:tokens", account_id);
            Some(vk.encrypt_field(t.as_bytes(), &aad_t)?)
        } else {
            None
        };

        let enc_notes = if let Some(n) = notes {
            let aad_n = format!("{}:1:notes", account_id);
            Some(vk.encrypt_field(n.as_bytes(), &aad_n)?)
        } else {
            None
        };

        let account = VaultAccount {
            id: account_id,
            provider: provider.to_string(),
            alias: alias.to_string(),
            encrypted_email: enc_email,
            encrypted_password: enc_password,
            encrypted_tokens: enc_tokens,
            encrypted_notes: enc_notes,
            encrypted_details: None,
            subscription_ends_at: None,
            created_at: now,
            updated_at: now,
        };

        self.state.accounts.push(account);
        self.state.updated_at = now;
        Ok(account_id)
    }

    /// Descifra el correo de una cuenta
    pub fn decrypt_account_email(&self, account_id: Uuid) -> Result<String> {
        let vk = self.unlocked_key.as_ref().ok_or(VaultError::VaultLocked)?;
        let acc = self
            .state
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or(VaultError::DecryptionFailed)?;

        let bytes = vk.decrypt_field(&acc.encrypted_email)?;
        String::from_utf8(bytes).map_err(|_| VaultError::DecryptionFailed)
    }

    /// Descifra la contraseña de una cuenta si existe
    pub fn decrypt_account_password(&self, account_id: Uuid) -> Result<Option<String>> {
        let vk = self.unlocked_key.as_ref().ok_or(VaultError::VaultLocked)?;
        let acc = self
            .state
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or(VaultError::DecryptionFailed)?;

        if let Some(enc_p) = &acc.encrypted_password {
            let bytes = vk.decrypt_field(enc_p)?;
            let s = String::from_utf8(bytes).map_err(|_| VaultError::DecryptionFailed)?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Descifra el JSON de tokens de una cuenta (el blob completo del Credential Manager).
    /// Devuelve None si la cuenta no tiene tokens guardados.
    pub fn decrypt_account_tokens(&self, account_id: Uuid) -> Result<Option<String>> {
        let vk = self.unlocked_key.as_ref().ok_or(VaultError::VaultLocked)?;
        let acc = self
            .state
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or(VaultError::DecryptionFailed)?;

        if let Some(enc_t) = &acc.encrypted_tokens {
            let bytes = vk.decrypt_field(enc_t)?;
            let s = String::from_utf8(bytes).map_err(|_| VaultError::DecryptionFailed)?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Descifra las notas de una cuenta si existen.
    pub fn decrypt_account_notes(&self, account_id: Uuid) -> Result<Option<String>> {
        self.decrypt_optional_field(account_id, |a| &a.encrypted_notes)
    }

    /// Descifra los detalles adicionales de una cuenta si existen.
    pub fn decrypt_account_details(&self, account_id: Uuid) -> Result<Option<String>> {
        self.decrypt_optional_field(account_id, |a| &a.encrypted_details)
    }

    fn decrypt_optional_field<F>(&self, account_id: Uuid, selector: F) -> Result<Option<String>>
    where
        F: FnOnce(&VaultAccount) -> &Option<EncryptedField>,
    {
        let vk = self.unlocked_key.as_ref().ok_or(VaultError::VaultLocked)?;
        let acc = self
            .state
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or(VaultError::DecryptionFailed)?;

        if let Some(enc) = selector(acc) {
            let bytes = vk.decrypt_field(enc)?;
            let s = String::from_utf8(bytes).map_err(|_| VaultError::DecryptionFailed)?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Actualiza alias, correo, contraseña, notas y detalles cifrados de una cuenta existente.
    /// Conserva `encrypted_tokens` intacto.
    pub fn update_account(
        &mut self,
        account_id: Uuid,
        alias: &str,
        email: &str,
        password: Option<&str>,
        notes: Option<&str>,
        details: Option<&str>,
        subscription_ends_at: Option<&str>,
    ) -> Result<()> {
        let vk = self.unlocked_key.as_ref().ok_or(VaultError::VaultLocked)?;
        let now = Utc::now();

        let acc = self
            .state
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
            .ok_or(VaultError::DecryptionFailed)?;

        let aad_email = format!("{}:1:email", account_id);
        acc.encrypted_email = vk.encrypt_field(email.as_bytes(), &aad_email)?;

        acc.encrypted_password = match password.filter(|p| !p.is_empty()) {
            Some(p) => Some(vk.encrypt_field(p.as_bytes(), &format!("{}:1:password", account_id))?),
            None => None,
        };

        acc.encrypted_notes = match notes.filter(|n| !n.trim().is_empty()) {
            Some(n) => Some(vk.encrypt_field(n.as_bytes(), &format!("{}:1:notes", account_id))?),
            None => None,
        };

        acc.encrypted_details = match details.filter(|d| !d.trim().is_empty()) {
            Some(d) => Some(vk.encrypt_field(d.as_bytes(), &format!("{}:1:details", account_id))?),
            None => None,
        };

        acc.alias = alias.to_string();
        acc.subscription_ends_at = subscription_ends_at
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        acc.updated_at = now;
        self.state.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editar_y_descifrar_credenciales_con_datos_falsos() {
        let (mut vault, _) = Vault::create_new("FakeMasterPass123!", None).expect("create");
        let id = vault
            .add_account(
                "agy",
                "cuenta-falsa",
                "test-fake-user@example.com",
                None,
                Some("{\"fake\":true}"),
                None,
            )
            .expect("add");

        vault
            .update_account(
                id,
                "cuenta-editada",
                "test-fake-user2@example.com",
                Some("FakePass123!"),
                Some("Nota falsa"),
                Some("Detalle falso"),
                Some("2026-12-31"),
            )
            .expect("update");

        // Bloquear y volver a desbloquear para confirmar persistencia en state cifrado
        let serialized = serde_json::to_string(vault.state()).expect("serialize");
        assert!(!serialized.contains("FakePass123!"));
        assert!(!serialized.contains("Nota falsa"));
        assert!(!serialized.contains("Detalle falso"));

        let loaded_state: EncryptedVaultState =
            serde_json::from_str(&serialized).expect("deserialize");
        let mut reopened = Vault::load_locked(loaded_state);
        reopened
            .unlock_with_password("FakeMasterPass123!")
            .expect("unlock");

        assert_eq!(
            reopened.decrypt_account_email(id).unwrap(),
            "test-fake-user2@example.com"
        );
        assert_eq!(
            reopened.decrypt_account_password(id).unwrap().as_deref(),
            Some("FakePass123!")
        );
        assert_eq!(
            reopened.decrypt_account_notes(id).unwrap().as_deref(),
            Some("Nota falsa")
        );
        assert_eq!(
            reopened.decrypt_account_details(id).unwrap().as_deref(),
            Some("Detalle falso")
        );
        assert_eq!(
            reopened.decrypt_account_tokens(id).unwrap().as_deref(),
            Some("{\"fake\":true}")
        );
    }
}

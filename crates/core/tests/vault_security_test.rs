use ai_keyvault_core::kdf::KdfParams;
use ai_keyvault_core::recovery::RecoveryKey;
use ai_keyvault_core::vault::Vault;

#[test]
fn test_vault_create_lock_unlock() {
    let fast_params = KdfParams {
        memory_kib: 1024,
        iterations: 1,
        parallelism: 1,
    };

    let password = "FakePassword123!Safe";
    let (mut vault, recovery_key) =
        Vault::create_new(password, Some(fast_params.clone())).expect("Error al crear bóveda");

    assert!(vault.is_unlocked());

    let acc_id = vault
        .add_account(
            "agy",
            "fake-test-account-1",
            "fake-user@example.com",
            Some("fake-pass-xyz"),
            Some(r#"{"access_token":"fake_token_123"}"#),
            Some("Nota de prueba"),
        )
        .expect("Error al agregar cuenta");

    // Verificar descifrado mientras está desbloqueada
    let email = vault.decrypt_account_email(acc_id).unwrap();
    assert_eq!(email, "fake-user@example.com");

    let pass = vault.decrypt_account_password(acc_id).unwrap();
    assert_eq!(pass.as_deref(), Some("fake-pass-xyz"));

    // Bloquear la bóveda
    vault.lock();
    assert!(!vault.is_unlocked());

    // Intentar descifrar bloqueada debe fallar
    assert!(vault.decrypt_account_email(acc_id).is_err());

    // Desbloquear con contraseña incorrecta debe fallar
    let bad_unlock = vault.unlock_with_password("WrongPassword");
    assert!(bad_unlock.is_err());
    assert!(!vault.is_unlocked());

    // Desbloquear con contraseña correcta
    let _auth_key = vault.unlock_with_password(password).unwrap();
    assert!(vault.is_unlocked());

    let email_after = vault.decrypt_account_email(acc_id).unwrap();
    assert_eq!(email_after, "fake-user@example.com");

    // Probar recuperación con RecoveryKey
    let recovery_str = recovery_key.to_formatted_string();
    assert!(recovery_str.starts_with("AKVR-"));

    let parsed_rec_key = RecoveryKey::from_formatted_string(&recovery_str).unwrap();

    let new_password = "NewFakePassword456!Changed";
    vault
        .recover_with_key(&parsed_rec_key, new_password)
        .unwrap();

    // Bloquear y desbloquear con la nueva contraseña
    vault.lock();
    vault.unlock_with_password(new_password).unwrap();
    assert_eq!(
        vault.decrypt_account_email(acc_id).unwrap(),
        "fake-user@example.com"
    );
}

#[test]
fn test_recovery_key_format_roundtrip() {
    let key = RecoveryKey::generate();
    let formatted = key.to_formatted_string();
    let parsed = RecoveryKey::from_formatted_string(&formatted).expect("Parse falló");
    assert_eq!(key.0, parsed.0);
}

#[test]
fn test_ciphertext_contains_no_cleartext() {
    let fast_params = KdfParams {
        memory_kib: 1024,
        iterations: 1,
        parallelism: 1,
    };

    let sensitive_email = "super-secret-cleartext-string-12345@example.com";
    let (mut vault, _) = Vault::create_new("MasterPass", Some(fast_params)).unwrap();
    let _acc_id = vault
        .add_account("codex", "alias1", sensitive_email, None, None, None)
        .unwrap();

    // Serializar el estado de la bóveda
    let serialized = serde_json::to_string(vault.state()).unwrap();

    // Ninguna parte del estado debe contener el string en texto plano
    assert!(!serialized.contains(sensitive_email));
    assert!(!serialized.contains("super-secret-cleartext"));
}

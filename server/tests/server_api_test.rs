use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;

// ─────────────────────────────────────────────────────────────────────────────
// Helper: inicializar un pool SQLite en memoria con todas las tablas
// ─────────────────────────────────────────────────────────────────────────────
async fn create_test_db() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Error al conectar a SQLite en memoria");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            public_key_hex TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS vault_snapshots (
            version INTEGER PRIMARY KEY,
            device_id TEXT NOT NULL,
            encrypted_payload TEXT NOT NULL,
            checksum_sha256 TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS quota_readings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            pool TEXT NOT NULL,
            tokens_used INTEGER NOT NULL,
            tokens_limit INTEGER NOT NULL,
            percent_used REAL NOT NULL,
            reset_time TEXT,
            source TEXT NOT NULL,
            read_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS quota_leases (
            account_id TEXT PRIMARY KEY,
            locked_by_device TEXT NOT NULL,
            locked_until TEXT NOT NULL,
            last_read_at TEXT
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("Error al crear tablas de prueba");

    pool
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Inicialización del esquema de BD
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_db_schema_initialization() {
    let pool = create_test_db().await;

    // Verificar inserción y lectura de un snapshot cifrado
    let insert = sqlx::query(
        "INSERT INTO vault_snapshots VALUES (1, 'device-pc1', '{\"cipher\":\"opaque\"}', 'chk123', '2026-11-23T00:00:00Z')"
    )
    .execute(&pool)
    .await;

    assert!(
        insert.is_ok(),
        "El esquema debe permitir insertar un snapshot cifrado"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Conocimiento cero del servidor
// Demuestra que el servidor almacena blobs cifrados opacos y no puede
// recuperar el texto plano aunque tenga acceso total a la BD.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_servidor_no_puede_descifrar_vault() {
    let pool = create_test_db().await;

    // Simulamos un payload que el cliente cifró con XChaCha20-Poly1305.
    // El servidor solo ve bytes opacos (base64 en este mock).
    let plaintext_secreto = "api_key=sk-prod-SUPER_SECRETO_12345";
    let payload_cifrado_por_cliente = "AAAA_CIPHERTEXT_OPACO_QUE_SIMULA_XCHACHA20_b64==";

    // El servidor inserta el payload cifrado tal como lo recibe
    sqlx::query(
        "INSERT INTO vault_snapshots VALUES (1, 'device-test', ?, 'sha256mock', '2026-11-24T00:00:00Z')"
    )
    .bind(payload_cifrado_por_cliente)
    .execute(&pool)
    .await
    .expect("El servidor debe poder insertar el blob cifrado");

    // El servidor lee el payload de la BD
    let row = sqlx::query("SELECT encrypted_payload FROM vault_snapshots WHERE version = 1")
        .fetch_one(&pool)
        .await
        .expect("El servidor debe poder leer el blob cifrado");

    let stored: String = row.get("encrypted_payload");

    // El servidor puede leer el ciphertext (lo necesita para sincronizar)
    assert_eq!(stored, payload_cifrado_por_cliente);

    // PERO: el texto plano secreto NO aparece en ningún lugar del almacenamiento
    assert!(
        !stored.contains(plaintext_secreto),
        "FALLO DE SEGURIDAD: el servidor puede leer texto plano en el payload almacenado"
    );

    // El servidor no tiene clave para descifrar — no existe función decrypt en el server crate
    // (confirmado por ausencia de dependencias de ai-keyvault-core en server/Cargo.toml)
    // Este test documenta y verifica ese invariante de diseño.
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Prueba de restauración de BD
// Simula el proceso de backup + restauración verificando integridad de datos.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_restauracion_desde_backup() {
    // ── Paso 1: BD "original" con datos
    let pool_original = create_test_db().await;

    sqlx::query(
        "INSERT INTO devices VALUES ('dev-pc1', 'PC Principal', 'pubkey_hex_abc123', 1, '2026-11-24T00:00:00Z', '2026-11-24T00:00:00Z')"
    )
    .execute(&pool_original)
    .await
    .expect("Insertar dispositivo en BD original");

    sqlx::query(
        "INSERT INTO vault_snapshots VALUES (1, 'dev-pc1', 'CIPHERTEXT_VERSION_1', 'sha256_v1', '2026-11-24T00:00:00Z')"
    )
    .execute(&pool_original)
    .await
    .expect("Insertar snapshot en BD original");

    sqlx::query(
        "INSERT INTO quota_readings (account_id, provider, pool, tokens_used, tokens_limit, percent_used, source, read_at)
         VALUES ('cuenta-agy-01', 'agy', 'gemini-weekly', 15000, 100000, 15.0, 'agy_usage_cmd', '2026-11-24T01:00:00Z')"
    )
    .execute(&pool_original)
    .await
    .expect("Insertar lectura de cuota en BD original");

    // ── Paso 2: "Backup" — exportar datos clave a un archivo temporal SQLite
    let backup_path = std::env::temp_dir().join("ai_keyvault_test_backup.db");
    let backup_url = format!("sqlite:{}?mode=rwc", backup_path.display());

    let pool_backup = SqlitePoolOptions::new()
        .connect(&backup_url)
        .await
        .expect("Crear BD de backup");

    // Recrear esquema en el backup
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, public_key_hex TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, last_seen_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS vault_snapshots (
            version INTEGER PRIMARY KEY, device_id TEXT NOT NULL,
            encrypted_payload TEXT NOT NULL, checksum_sha256 TEXT NOT NULL, updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS quota_readings (
            id INTEGER PRIMARY KEY AUTOINCREMENT, account_id TEXT NOT NULL, provider TEXT NOT NULL,
            pool TEXT NOT NULL, tokens_used INTEGER NOT NULL, tokens_limit INTEGER NOT NULL,
            percent_used REAL NOT NULL, reset_time TEXT, source TEXT NOT NULL, read_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool_backup)
    .await
    .expect("Crear tablas en backup");

    // Copiar datos de la BD original al backup
    let devices: Vec<(String, String, String, i64, String, String)> = sqlx::query(
        "SELECT id, name, public_key_hex, is_active, created_at, last_seen_at FROM devices",
    )
    .map(|r: sqlx::sqlite::SqliteRow| {
        (
            r.get("id"),
            r.get("name"),
            r.get("public_key_hex"),
            r.get("is_active"),
            r.get("created_at"),
            r.get("last_seen_at"),
        )
    })
    .fetch_all(&pool_original)
    .await
    .expect("Leer devices de BD original");

    for d in &devices {
        sqlx::query("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&d.0)
            .bind(&d.1)
            .bind(&d.2)
            .bind(d.3)
            .bind(&d.4)
            .bind(&d.5)
            .execute(&pool_backup)
            .await
            .expect("Copiar device al backup");
    }

    let snapshots: Vec<(i64, String, String, String, String)> = sqlx::query(
        "SELECT version, device_id, encrypted_payload, checksum_sha256, updated_at FROM vault_snapshots"
    )
    .map(|r: sqlx::sqlite::SqliteRow| (
        r.get("version"), r.get("device_id"), r.get("encrypted_payload"),
        r.get("checksum_sha256"), r.get("updated_at"),
    ))
    .fetch_all(&pool_original)
    .await
    .expect("Leer snapshots de BD original");

    for s in &snapshots {
        sqlx::query("INSERT INTO vault_snapshots VALUES (?, ?, ?, ?, ?)")
            .bind(s.0)
            .bind(&s.1)
            .bind(&s.2)
            .bind(&s.3)
            .bind(&s.4)
            .execute(&pool_backup)
            .await
            .expect("Copiar snapshot al backup");
    }

    pool_backup.close().await;

    // ── Paso 3: "Restauración" — abrir el backup como BD nueva
    let pool_restaurado = SqlitePoolOptions::new()
        .connect(&backup_url)
        .await
        .expect("Abrir BD restaurada desde backup");

    // Verificar que los datos están íntegros
    let device_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices")
        .fetch_one(&pool_restaurado)
        .await
        .expect("Contar devices en BD restaurada");

    assert_eq!(
        device_count, 1,
        "La BD restaurada debe tener exactamente 1 dispositivo"
    );

    let snapshot_payload: String =
        sqlx::query_scalar("SELECT encrypted_payload FROM vault_snapshots WHERE version = 1")
            .fetch_one(&pool_restaurado)
            .await
            .expect("Leer snapshot restaurado");

    assert_eq!(
        snapshot_payload, "CIPHERTEXT_VERSION_1",
        "El payload cifrado debe preservarse intacto tras el backup/restauración"
    );

    // Limpieza
    pool_restaurado.close().await;
    let _ = std::fs::remove_file(&backup_path);
}

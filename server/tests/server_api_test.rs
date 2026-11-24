use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn test_db_schema_initialization() {
    // Probar BD SQLite en memoria
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Error al conectar a SQLite en memoria");

    // Ejecutar migraciones / inicialización
    let init_res = sqlx::query(
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
    .await;

    assert!(init_res.is_ok(), "Las tablas deben crearse correctamente");

    // Verificar inserción y lectura de un snapshot cifrado
    let insert = sqlx::query(
        "INSERT INTO vault_snapshots VALUES (1, 'device-pc1', '{\"cipher\":\"opaque\"}', 'chk123', '2026-11-23T00:00:00Z')"
    )
    .execute(&pool)
    .await;

    assert!(insert.is_ok());
}

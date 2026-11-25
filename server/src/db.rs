use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 1. Tabla de dispositivos registrados (autenticación Ed25519)
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
        "#,
    )
    .execute(pool)
    .await?;

    // 2. Tabla del blob cifrado de la bóveda (Conocimiento Cero)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS vault_snapshots (
            version INTEGER PRIMARY KEY,
            device_id TEXT NOT NULL,
            encrypted_payload TEXT NOT NULL,
            checksum_sha256 TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (device_id) REFERENCES devices(id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 3. Tabla de lecturas de cuota (serie temporal multi-dispositivo)
    sqlx::query(
        r#"
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
        "#,
    )
    .execute(pool)
    .await?;

    // 4. Tabla de leases anti-duplicados para consultas
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS quota_leases (
            account_id TEXT PRIMARY KEY,
            locked_by_device TEXT NOT NULL,
            locked_until TEXT NOT NULL,
            last_read_at TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 5. Tabla de sesiones de chat de agy (Fase 6)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id          TEXT NOT NULL,
            account_id  TEXT NOT NULL,
            provider    TEXT NOT NULL DEFAULT 'agy',
            title       TEXT,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            device_id   TEXT NOT NULL,
            archived    INTEGER NOT NULL DEFAULT 0,
            synced_at   TEXT NOT NULL,
            PRIMARY KEY (id, account_id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

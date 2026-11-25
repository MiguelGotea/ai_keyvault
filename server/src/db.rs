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

    // 5. Tabla de sesiones de chat (agy Fase 6, Codex Fase A)
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

    // 6. Tabla de pings (Fase 8)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ping_log (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id   TEXT NOT NULL,
            provider     TEXT NOT NULL DEFAULT 'agy',
            device_id    TEXT NOT NULL,
            success      INTEGER NOT NULL DEFAULT 0,
            error        TEXT,
            duration_ms  INTEGER NOT NULL DEFAULT 0,
            message_sent TEXT NOT NULL DEFAULT 'k',
            is_manual    INTEGER NOT NULL DEFAULT 1,
            pinged_at    TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 7. Tabla de historial de switches de cuenta (Fase E)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS switch_log (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            to_account_id   TEXT NOT NULL,
            from_account_id TEXT,
            provider        TEXT NOT NULL DEFAULT 'agy',
            device_id       TEXT NOT NULL,
            success         INTEGER NOT NULL DEFAULT 1,
            reason          TEXT,
            switched_at     TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 8. Tabla de historial de alertas disparadas (Fase D)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS alert_log (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id      TEXT NOT NULL,
            provider        TEXT NOT NULL DEFAULT 'agy',
            pool            TEXT NOT NULL,
            level           TEXT NOT NULL,
            percent_used    REAL NOT NULL,
            reset_time      TEXT,
            notified_toast  INTEGER NOT NULL DEFAULT 0,
            notified_ntfy   INTEGER NOT NULL DEFAULT 0,
            detected_at     TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

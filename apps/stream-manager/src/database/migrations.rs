use sqlx::SqlitePool;
use super::Result;

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Create initial schema",
        sql: r#"
            CREATE TABLE IF NOT EXISTS streams (
                id TEXT PRIMARY KEY,
                uri TEXT NOT NULL,
                config TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_streams_status ON streams(status);
            CREATE INDEX IF NOT EXISTS idx_streams_created_at ON streams(created_at);

            CREATE TABLE IF NOT EXISTS recordings (
                id TEXT PRIMARY KEY,
                stream_id TEXT NOT NULL,
                path TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                size_bytes INTEGER,
                duration_ms INTEGER,
                status TEXT NOT NULL,
                metadata TEXT,
                FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_recordings_stream_id ON recordings(stream_id);
            CREATE INDEX IF NOT EXISTS idx_recordings_start_time ON recordings(start_time);
            CREATE INDEX IF NOT EXISTS idx_recordings_end_time ON recordings(end_time);
            CREATE INDEX IF NOT EXISTS idx_recordings_status ON recordings(status);
            CREATE INDEX IF NOT EXISTS idx_recordings_stream_start ON recordings(stream_id, start_time);
            CREATE INDEX IF NOT EXISTS idx_recordings_stream_end ON recordings(stream_id, end_time);
            CREATE INDEX IF NOT EXISTS idx_recordings_path ON recordings(path);

            CREATE TABLE IF NOT EXISTS state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS migration_history (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );
        "#,
    },
];

pub struct Migration {
    pub version: i64,
    pub description: &'static str,
    pub sql: &'static str,
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Create migration history table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS migration_history (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        )
        "#
    )
    .execute(pool)
    .await?;

    for migration in MIGRATIONS {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM migration_history WHERE version = ?1)"
        )
        .bind(migration.version)
        .fetch_one(pool)
        .await?;

        if !exists {
            tracing::info!(
                "Applying migration {}: {}",
                migration.version,
                migration.description
            );

            // Run the migration
            sqlx::query(migration.sql)
                .execute(pool)
                .await?;

            // Record the migration
            let now = chrono::Utc::now().timestamp();
            sqlx::query(
                "INSERT INTO migration_history (version, description, applied_at) VALUES (?1, ?2, ?3)"
            )
            .bind(migration.version)
            .bind(migration.description)
            .bind(now)
            .execute(pool)
            .await?;

            tracing::info!("Migration {} applied successfully", migration.version);
        }
    }

    Ok(())
}

pub async fn get_current_version(pool: &SqlitePool) -> Result<Option<i64>> {
    let version: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(version) FROM migration_history"
    )
    .fetch_one(pool)
    .await?;

    Ok(version)
}
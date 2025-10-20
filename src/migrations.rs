use nostr_database::DatabaseError;

pub async fn run_migrations(pool: &deadpool_postgres::Pool) -> Result<(), DatabaseError> {
    run_query(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id BYTEA PRIMARY KEY NOT NULL,
            pubkey BYTEA NOT NULL,
            created_at BIGINT NOT NULL,
            kind BIGINT NOT NULL,
            payload BYTEA NOT NULL,
            deleted BOOLEAN NOT NULL
        );
    "#,
    )
    .await?;

    run_query(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS event_tags (
            tag TEXT NOT NULL,
            tag_value TEXT NOT NULL,
            event_id BYTEA NOT NULL
            REFERENCES events (id)
            ON DELETE CASCADE
            ON UPDATE CASCADE
        );
    "#,
    )
    .await?;

    run_query(
        pool,
        r#"
        CREATE INDEX IF NOT EXISTS event_pubkey ON events (pubkey);
    "#,
    )
    .await?;
    run_query(
        pool,
        r#"
        CREATE INDEX IF NOT EXISTS event_date ON events (created_at);
    "#,
    )
    .await?;
    run_query(
        pool,
        r#"
        CREATE INDEX IF NOT EXISTS event_kind ON events (kind);
    "#,
    )
    .await?;
    run_query(
        pool,
        r#"
        CREATE INDEX IF NOT EXISTS event_deleted ON events (deleted);
    "#,
    )
    .await?;
    run_query(
        pool,
        r#"
        CREATE INDEX IF NOT EXISTS event_tags_tag ON event_tags (tag, tag_value, event_id);
    "#,
    )
    .await?;

    Ok(())
}

async fn run_query(pool: &deadpool_postgres::Pool, query: &str) -> Result<(), DatabaseError> {
    pool.get()
        .await
        .map_err(DatabaseError::backend)?
        .execute(query, &[])
        .await
        .map_err(DatabaseError::backend)?;
    Ok(())
}

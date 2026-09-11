// Copyright (c) 2025 Protom
// Distributed under the MIT software license

use std::time::Duration;

use nostr_postgres_db::NostrPostgres;
use nostr_sdk::prelude::*;

// Your database URL
const DB_URL: &str = "postgres://postgres:password@localhost:5432";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create a nostr db instance and run pending db migrations if any
    let db = NostrPostgres::new(DB_URL).await?;

    // Create a local relay backed by Postgres
    let relay = LocalRelay::builder().database(db).build();
    relay.run().await?;
    println!("Url: {}", relay.url().await);

    // Keep up the program
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

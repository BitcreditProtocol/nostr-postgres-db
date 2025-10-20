// Common utilities for integration tests
use nostr::prelude::*;
use nostr_database::prelude::*;
use nostr_postgres_db::NostrPostgres;
use std::env;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;

pub struct TestDatabase {
    pub db: NostrPostgres,
    #[allow(dead_code)]
    container: Option<ContainerAsync<Postgres>>,
}

/// Setup a test database connection using testcontainers
/// 
/// If DATABASE_URL environment variable is set, uses that instead of starting a container.
/// This allows for CI environments or manual testing with existing databases.
pub async fn setup_test_db() -> TestDatabase {
    // Check if DATABASE_URL is set (for CI or manual testing)
    if let Ok(db_url) = env::var("DATABASE_URL") {
        let db = NostrPostgres::new(&db_url)
            .await
            .expect("Failed to connect to test database");
        return TestDatabase {
            db,
            container: None,
        };
    }

    // Start PostgreSQL container using testcontainers
    let container = Postgres::default()
        .start()
        .await
        .expect("Failed to start PostgreSQL container");
    
    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get container port");
    
    // Build connection string
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        host_port
    );
    
    // Create database connection
    let db = NostrPostgres::new(&db_url)
        .await
        .expect("Failed to connect to test database");
    
    TestDatabase {
        db,
        container: Some(container),
    }
}

/// Clean up test data (deletes all events)
pub async fn cleanup_test_db(test_db: &TestDatabase) {
    // Query all events and delete them
    let filter = Filter::new();
    let _ = test_db.db.delete(filter).await;
}

/// Create a test event with given content
pub fn create_test_event(keys: &Keys, content: &str) -> Event {
    EventBuilder::text_note(content)
        .sign_with_keys(keys)
        .unwrap()
}

/// Create multiple test events
pub fn create_test_events(keys: &Keys, count: usize) -> Vec<Event> {
    (0..count)
        .map(|i| create_test_event(keys, &format!("Test event {}", i)))
        .collect()
}

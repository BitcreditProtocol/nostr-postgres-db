// Common utilities for integration tests
use nostr::prelude::*;
use nostr_database::prelude::*;
use nostr_postgres_db::NostrPostgres;
use std::env;

/// Setup a test database connection
pub async fn setup_test_db() -> NostrPostgres {
    // Get database URL from environment or use default
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/nostr_test".to_string());
    
    NostrPostgres::new(&db_url)
        .await
        .expect("Failed to connect to test database")
}

/// Clean up test data (deletes all events)
pub async fn cleanup_test_db(db: &NostrPostgres) {
    // Query all events and delete them
    let filter = Filter::new();
    let _ = db.delete(filter).await;
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

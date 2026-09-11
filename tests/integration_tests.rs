// Integration tests for NostrPostgres database operations
use nostr::prelude::*;
use nostr_database::prelude::*;

mod common;
use common::*;

#[tokio::test]
async fn test_save_and_retrieve_event() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    let event = EventBuilder::new(Kind::TextNote, "Hello Nostr!")
        .finalize(&keys)
        .unwrap();

    // Save event
    let status = test_db.db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Success);

    // Retrieve event
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, event.id);
    assert_eq!(retrieved.content, event.content);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_save_duplicate_event() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    let event = EventBuilder::new(Kind::TextNote, "Test duplicate")
        .finalize(&keys)
        .unwrap();

    // Save event first time
    let status = test_db.db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Success);

    // Try to save same event again
    let status = test_db.db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Rejected(RejectedReason::Duplicate));

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_check_id_saved() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    let event = EventBuilder::new(Kind::TextNote, "Test check ID")
        .finalize(&keys)
        .unwrap();

    // Check status before saving
    let status = test_db.db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::NotExistent);

    // Save event
    test_db.db.save_event(&event).await.unwrap();

    // Check status after saving
    let status = test_db.db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::Saved);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_check_id_deleted() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    let event = EventBuilder::new(Kind::TextNote, "Test delete")
        .finalize(&keys)
        .unwrap();

    // Save and delete event
    test_db.db.save_event(&event).await.unwrap();
    let filter = Filter::new().id(event.id);
    test_db.db.delete(filter).await.unwrap();

    // Check status after deletion
    let status = test_db.db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::Deleted);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_by_author() {
    let test_db = setup_test_db().await;
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();

    // Create events from two different authors
    let event1 = EventBuilder::new(Kind::TextNote, "From author 1")
        .finalize(&keys1)
        .unwrap();
    let event2 = EventBuilder::new(Kind::TextNote, "From author 2")
        .finalize(&keys2)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2).await.unwrap();

    // Query by first author
    let filter = Filter::new().author(keys1.public_key());
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().pubkey, keys1.public_key());

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_by_kinds() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create events of different kinds
    let event1 = EventBuilder::new(Kind::TextNote, "Text note")
        .finalize(&keys)
        .unwrap();
    let event2 = EventBuilder::new(Kind::Metadata, "{}")
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2).await.unwrap();

    // Query for text notes only
    let filter = Filter::new().kind(Kind::TextNote);
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().kind, Kind::TextNote);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_by_ids() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create multiple events
    let event1 = EventBuilder::new(Kind::TextNote, "Event 1")
        .finalize(&keys)
        .unwrap();
    let event2 = EventBuilder::new(Kind::TextNote, "Event 2")
        .finalize(&keys)
        .unwrap();
    let event3 = EventBuilder::new(Kind::TextNote, "Event 3")
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2).await.unwrap();
    test_db.db.save_event(&event3).await.unwrap();

    // Query specific IDs
    let filter = Filter::new().ids([event1.id, event3.id]);
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 2);
    let ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
    assert!(ids.contains(&event1.id));
    assert!(ids.contains(&event3.id));
    assert!(!ids.contains(&event2.id));

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_by_since() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create event with specific timestamp
    let old_event = EventBuilder::new(Kind::TextNote, "Old event")
        .custom_created_at(Timestamp::from(1000000))
        .finalize(&keys)
        .unwrap();
    let new_event = EventBuilder::new(Kind::TextNote, "New event")
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&old_event).await.unwrap();
    test_db.db.save_event(&new_event).await.unwrap();

    // Query events since a certain time
    let filter = Filter::new()
        .author(keys.public_key())
        .since(Timestamp::from(2000000));
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, new_event.id);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_by_until() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create event with specific timestamp
    let old_event = EventBuilder::new(Kind::TextNote, "Old event")
        .custom_created_at(Timestamp::from(1000000))
        .finalize(&keys)
        .unwrap();
    let new_event = EventBuilder::new(Kind::TextNote, "New event")
        .custom_created_at(Timestamp::from(3000000))
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&old_event).await.unwrap();
    test_db.db.save_event(&new_event).await.unwrap();

    // Query events until a certain time
    let filter = Filter::new()
        .author(keys.public_key())
        .until(Timestamp::from(2000000));
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, old_event.id);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_with_limit() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create multiple events
    for i in 0..10 {
        let event = EventBuilder::new(Kind::TextNote, format!("Event {}", i))
            .finalize(&keys)
            .unwrap();
        test_db.db.save_event(&event).await.unwrap();
    }

    // Query with limit
    let filter = Filter::new().author(keys.public_key()).limit(5);
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 5);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_by_tags() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();
    let tagged_pubkey = Keys::generate().public_key();

    // Create event with tag
    let event_with_tag = EventBuilder::new(Kind::TextNote, "Tagged event")
        .tags([Tag::public_key(tagged_pubkey)])
        .finalize(&keys)
        .unwrap();
    let event_without_tag = EventBuilder::new(Kind::TextNote, "Untagged event")
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&event_with_tag).await.unwrap();
    test_db.db.save_event(&event_without_tag).await.unwrap();

    // Query by tag
    let filter = Filter::new()
        .author(keys.public_key())
        .pubkey(tagged_pubkey);
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, event_with_tag.id);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_count_events() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Initially no events
    let count = test_db
        .db
        .count(Filter::new().author(keys.public_key()))
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Add some events
    for i in 0..5 {
        let event = EventBuilder::new(Kind::TextNote, format!("Event {}", i))
            .finalize(&keys)
            .unwrap();
        test_db.db.save_event(&event).await.unwrap();
    }

    // Count should match
    let count = test_db
        .db
        .count(Filter::new().author(keys.public_key()))
        .await
        .unwrap();
    assert_eq!(count, 5);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_delete_events() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create and save event
    let event = EventBuilder::new(Kind::TextNote, "To be deleted")
        .finalize(&keys)
        .unwrap();
    test_db.db.save_event(&event).await.unwrap();

    // Verify it exists
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());

    // Delete it
    let filter = Filter::new().id(event.id);
    test_db.db.delete(filter).await.unwrap();

    // Verify it's deleted (returns None)
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_none());

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_delete_by_author() {
    let test_db = setup_test_db().await;
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();

    // Create events from two authors
    let event1 = EventBuilder::new(Kind::TextNote, "Author 1")
        .finalize(&keys1)
        .unwrap();
    let event2 = EventBuilder::new(Kind::TextNote, "Author 2")
        .finalize(&keys2)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2).await.unwrap();

    // Delete events from author 1
    let filter = Filter::new().author(keys1.public_key());
    test_db.db.delete(filter).await.unwrap();

    // Verify author 1's event is deleted
    let retrieved = test_db.db.event_by_id(&event1.id).await.unwrap();
    assert!(retrieved.is_none());

    // Verify author 2's event still exists
    let retrieved = test_db.db.event_by_id(&event2.id).await.unwrap();
    assert!(retrieved.is_some());

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_complex_query() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create various events
    let event1 = EventBuilder::new(Kind::TextNote, "Match")
        .custom_created_at(Timestamp::from(2000000))
        .finalize(&keys)
        .unwrap();
    // For event2, we'll use metadata kind to differentiate
    let event2_metadata = EventBuilder::new(Kind::Metadata, "{}")
        .custom_created_at(Timestamp::from(2000000))
        .finalize(&keys)
        .unwrap();
    let event3 = EventBuilder::new(Kind::TextNote, "No match - wrong time")
        .custom_created_at(Timestamp::from(1000000))
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2_metadata).await.unwrap();
    test_db.db.save_event(&event3).await.unwrap();

    // Complex query
    let filter = Filter::new()
        .author(keys.public_key())
        .kind(Kind::TextNote)
        .since(Timestamp::from(1500000));
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, event1.id);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_backend_method() {
    let test_db = setup_test_db().await;
    assert_eq!(test_db.db.backend(), "postgres");

    let features = test_db.db.features();
    assert!(features.persistent);
    assert!(!features.event_expiration);
    assert!(!features.full_text_search);
    assert!(!features.request_to_vanish);
    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_wipe_not_supported() {
    let test_db = setup_test_db().await;
    let err = test_db
        .db
        .wipe()
        .await
        .expect_err("wipe must not be supported");
    assert_eq!(err.kind(), nostr_database::error::ErrorKind::Unsupported);
    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_event_with_multiple_tags() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();
    let other_pubkey1 = Keys::generate().public_key();
    let other_pubkey2 = Keys::generate().public_key();
    let event_id = EventId::from_byte_array([0; 32]);

    // Create event with multiple tags
    let event = EventBuilder::new(Kind::TextNote, "Multi-tagged")
        .tags([
            Tag::public_key(other_pubkey1),
            Tag::public_key(other_pubkey2),
            Tag::event(event_id),
            Tag::hashtag("nostr"),
            Tag::hashtag("test"),
        ])
        .finalize(&keys)
        .unwrap();

    test_db.db.save_event(&event).await.unwrap();

    // Retrieve and verify
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.tags.len(), event.tags.len());

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_from_pool_constructor() {
    use nostr_postgres_db::{NostrPostgres, postgres_connection_pool};
    use std::env;

    // Use DATABASE_URL when set (CI or manual testing), otherwise start a container
    let (_container, db_url) = match env::var("DATABASE_URL") {
        Ok(db_url) => (None, db_url),
        Err(_) => {
            let (container, db_url) = start_postgres().await;
            (Some(container), db_url)
        }
    };

    // Create a connection pool
    let pool = postgres_connection_pool(&db_url)
        .await
        .expect("Failed to create connection pool");

    // Create NostrPostgres from pool
    let db = NostrPostgres::from_pool(pool)
        .await
        .expect("Failed to create NostrPostgres from pool");

    // Test basic functionality
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::TextNote, "Test from_pool")
        .finalize(&keys)
        .unwrap();

    let status = db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Success);

    let retrieved = db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, event.id);

    // Cleanup
    let filter = Filter::new();
    let _ = db.delete(filter).await;
}

// Integration tests for NostrPostgres database operations
use nostr::prelude::*;
use nostr_database::prelude::*;
use nostr_postgres_db::NostrPostgres;

mod common;
use common::*;

#[tokio::test]
async fn test_save_and_retrieve_event() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    let event = EventBuilder::text_note("Hello Nostr!")
        .sign_with_keys(&keys)
        .unwrap();
    
    // Save event
    let status = db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Success);
    
    // Retrieve event
    let retrieved = db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, event.id);
    assert_eq!(retrieved.content, event.content);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_save_duplicate_event() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    let event = EventBuilder::text_note("Test duplicate")
        .sign_with_keys(&keys)
        .unwrap();
    
    // Save event first time
    let status = db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Success);
    
    // Try to save same event again
    let status = db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Rejected(RejectedReason::Duplicate));
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_check_id_saved() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    let event = EventBuilder::text_note("Test check ID")
        .sign_with_keys(&keys)
        .unwrap();
    
    // Check status before saving
    let status = db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::NotExistent);
    
    // Save event
    db.save_event(&event).await.unwrap();
    
    // Check status after saving
    let status = db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::Saved);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_check_id_deleted() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    let event = EventBuilder::text_note("Test delete")
        .sign_with_keys(&keys)
        .unwrap();
    
    // Save and delete event
    db.save_event(&event).await.unwrap();
    let filter = Filter::new().id(event.id);
    db.delete(filter).await.unwrap();
    
    // Check status after deletion
    let status = db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::Deleted);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_by_author() {
    let db = setup_test_db().await;
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();
    
    // Create events from two different authors
    let event1 = EventBuilder::text_note("From author 1")
        .sign_with_keys(&keys1)
        .unwrap();
    let event2 = EventBuilder::text_note("From author 2")
        .sign_with_keys(&keys2)
        .unwrap();
    
    db.save_event(&event1).await.unwrap();
    db.save_event(&event2).await.unwrap();
    
    // Query by first author
    let filter = Filter::new().author(keys1.public_key());
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().pubkey, keys1.public_key());
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_by_kinds() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create events of different kinds
    let event1 = EventBuilder::text_note("Text note")
        .sign_with_keys(&keys)
        .unwrap();
    let event2 = EventBuilder::metadata(&Metadata::new())
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&event1).await.unwrap();
    db.save_event(&event2).await.unwrap();
    
    // Query for text notes only
    let filter = Filter::new().kind(Kind::TextNote);
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().kind, Kind::TextNote);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_by_ids() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create multiple events
    let event1 = EventBuilder::text_note("Event 1")
        .sign_with_keys(&keys)
        .unwrap();
    let event2 = EventBuilder::text_note("Event 2")
        .sign_with_keys(&keys)
        .unwrap();
    let event3 = EventBuilder::text_note("Event 3")
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&event1).await.unwrap();
    db.save_event(&event2).await.unwrap();
    db.save_event(&event3).await.unwrap();
    
    // Query specific IDs
    let filter = Filter::new().ids([event1.id, event3.id]);
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 2);
    let ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
    assert!(ids.contains(&event1.id));
    assert!(ids.contains(&event3.id));
    assert!(!ids.contains(&event2.id));
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_by_since() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create event with specific timestamp
    let old_event = EventBuilder::text_note("Old event")
        .custom_created_at(Timestamp::from(1000000))
        .sign_with_keys(&keys)
        .unwrap();
    let new_event = EventBuilder::text_note("New event")
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&old_event).await.unwrap();
    db.save_event(&new_event).await.unwrap();
    
    // Query events since a certain time
    let filter = Filter::new()
        .author(keys.public_key())
        .since(Timestamp::from(2000000));
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, new_event.id);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_by_until() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create event with specific timestamp
    let old_event = EventBuilder::text_note("Old event")
        .custom_created_at(Timestamp::from(1000000))
        .sign_with_keys(&keys)
        .unwrap();
    let new_event = EventBuilder::text_note("New event")
        .custom_created_at(Timestamp::from(3000000))
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&old_event).await.unwrap();
    db.save_event(&new_event).await.unwrap();
    
    // Query events until a certain time
    let filter = Filter::new()
        .author(keys.public_key())
        .until(Timestamp::from(2000000));
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, old_event.id);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_with_limit() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create multiple events
    for i in 0..10 {
        let event = EventBuilder::text_note(format!("Event {}", i))
            .sign_with_keys(&keys)
            .unwrap();
        db.save_event(&event).await.unwrap();
    }
    
    // Query with limit
    let filter = Filter::new().author(keys.public_key()).limit(5);
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 5);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_query_by_tags() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    let tagged_pubkey = Keys::generate().public_key();
    
    // Create event with tag
    let event_with_tag = EventBuilder::text_note("Tagged event")
        .tags([Tag::public_key(tagged_pubkey)])
        .sign_with_keys(&keys)
        .unwrap();
    let event_without_tag = EventBuilder::text_note("Untagged event")
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&event_with_tag).await.unwrap();
    db.save_event(&event_without_tag).await.unwrap();
    
    // Query by tag
    let filter = Filter::new()
        .author(keys.public_key())
        .pubkey(tagged_pubkey);
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, event_with_tag.id);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_count_events() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Initially no events
    let count = db.count(Filter::new().author(keys.public_key())).await.unwrap();
    assert_eq!(count, 0);
    
    // Add some events
    for i in 0..5 {
        let event = EventBuilder::text_note(format!("Event {}", i))
            .sign_with_keys(&keys)
            .unwrap();
        db.save_event(&event).await.unwrap();
    }
    
    // Count should match
    let count = db.count(Filter::new().author(keys.public_key())).await.unwrap();
    assert_eq!(count, 5);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_delete_events() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create and save event
    let event = EventBuilder::text_note("To be deleted")
        .sign_with_keys(&keys)
        .unwrap();
    db.save_event(&event).await.unwrap();
    
    // Verify it exists
    let retrieved = db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    
    // Delete it
    let filter = Filter::new().id(event.id);
    db.delete(filter).await.unwrap();
    
    // Verify it's deleted (returns None)
    let retrieved = db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_none());
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_delete_by_author() {
    let db = setup_test_db().await;
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();
    
    // Create events from two authors
    let event1 = EventBuilder::text_note("Author 1")
        .sign_with_keys(&keys1)
        .unwrap();
    let event2 = EventBuilder::text_note("Author 2")
        .sign_with_keys(&keys2)
        .unwrap();
    
    db.save_event(&event1).await.unwrap();
    db.save_event(&event2).await.unwrap();
    
    // Delete events from author 1
    let filter = Filter::new().author(keys1.public_key());
    db.delete(filter).await.unwrap();
    
    // Verify author 1's event is deleted
    let retrieved = db.event_by_id(&event1.id).await.unwrap();
    assert!(retrieved.is_none());
    
    // Verify author 2's event still exists
    let retrieved = db.event_by_id(&event2.id).await.unwrap();
    assert!(retrieved.is_some());
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_complex_query() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    
    // Create various events
    let event1 = EventBuilder::text_note("Match")
        .custom_created_at(Timestamp::from(2000000))
        .sign_with_keys(&keys)
        .unwrap();
    let event2 = EventBuilder::text_note("No match - wrong kind")
        .custom_created_at(Timestamp::from(2000000))
        .sign_with_keys(&keys)
        .unwrap();
    let event3 = EventBuilder::text_note("No match - wrong time")
        .custom_created_at(Timestamp::from(1000000))
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&event1).await.unwrap();
    // For event2, we'll use metadata kind to differentiate
    let event2_metadata = EventBuilder::metadata(&Metadata::new())
        .custom_created_at(Timestamp::from(2000000))
        .sign_with_keys(&keys)
        .unwrap();
    db.save_event(&event2_metadata).await.unwrap();
    db.save_event(&event3).await.unwrap();
    
    // Complex query
    let filter = Filter::new()
        .author(keys.public_key())
        .kind(Kind::TextNote)
        .since(Timestamp::from(1500000));
    let events = db.query(filter).await.unwrap();
    
    assert_eq!(events.len(), 1);
    assert_eq!(events.iter().next().unwrap().id, event1.id);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_backend_method() {
    let db = setup_test_db().await;
    let backend = db.backend();
    assert_eq!(backend, Backend::Custom("Postgres".to_string()));
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wipe_not_supported() {
    let db = setup_test_db().await;
    let result = db.wipe().await;
    assert!(matches!(result, Err(DatabaseError::NotSupported)));
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_event_with_multiple_tags() {
    let db = setup_test_db().await;
    let keys = Keys::generate();
    let other_pubkey1 = Keys::generate().public_key();
    let other_pubkey2 = Keys::generate().public_key();
    let event_id = EventId::all_zeros();
    
    // Create event with multiple tags
    let event = EventBuilder::text_note("Multi-tagged")
        .tags([
            Tag::public_key(other_pubkey1),
            Tag::public_key(other_pubkey2),
            Tag::event(event_id),
            Tag::hashtag("nostr"),
            Tag::hashtag("test"),
        ])
        .sign_with_keys(&keys)
        .unwrap();
    
    db.save_event(&event).await.unwrap();
    
    // Retrieve and verify
    let retrieved = db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.tags.len(), event.tags.len());
    
    cleanup_test_db(&db).await;
}

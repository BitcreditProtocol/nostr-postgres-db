// Additional integration tests for edge cases and error scenarios
use nostr::prelude::*;
use nostr_database::prelude::*;

mod common;
use common::*;

#[tokio::test]
async fn test_empty_database_query() {
    let test_db = setup_test_db().await;

    // Query empty database
    let filter = Filter::new();
    let events = test_db.db.query(filter).await.unwrap();
    assert_eq!(events.len(), 0);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_empty_database_count() {
    let test_db = setup_test_db().await;

    // Count in empty database
    let count = test_db.db.count(Filter::new()).await.unwrap();
    assert_eq!(count, 0);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_nonexistent_id() {
    let test_db = setup_test_db().await;

    // Query for non-existent event
    let nonexistent_id = EventId::all_zeros();
    let result = test_db.db.event_by_id(&nonexistent_id).await.unwrap();
    assert!(result.is_none());

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_nonexistent_author() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Query for non-existent author
    let filter = Filter::new().author(keys.public_key());
    let events = test_db.db.query(filter).await.unwrap();
    assert_eq!(events.len(), 0);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_large_batch_save() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Save many events
    let event_count = 100;
    for i in 0..event_count {
        let event = EventBuilder::text_note(format!("Batch event {}", i))
            .sign_with_keys(&keys)
            .unwrap();
        let status = test_db.db.save_event(&event).await.unwrap();
        assert_eq!(status, SaveEventStatus::Success);
    }

    // Verify count
    let count = test_db
        .db
        .count(Filter::new().author(keys.public_key()))
        .await
        .unwrap();
    assert_eq!(count, event_count);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_with_very_large_limit() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create a few events
    for i in 0..5 {
        let event = EventBuilder::text_note(format!("Event {}", i))
            .sign_with_keys(&keys)
            .unwrap();
        test_db.db.save_event(&event).await.unwrap();
    }

    // Query with large limit
    let filter = Filter::new().author(keys.public_key()).limit(10000);
    let events = test_db.db.query(filter).await.unwrap();

    // Should only return actual count
    assert_eq!(events.len(), 5);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_timestamp_edge_cases() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Event at timestamp 0
    let event1 = EventBuilder::text_note("At zero")
        .custom_created_at(Timestamp::from(0))
        .sign_with_keys(&keys)
        .unwrap();

    // Event at max timestamp
    let event2 = EventBuilder::text_note("At max")
        .custom_created_at(Timestamp::from(i64::MAX as u64))
        .sign_with_keys(&keys)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2).await.unwrap();

    // Query with timestamp filter
    let filter = Filter::new()
        .since(Timestamp::from(0))
        .until(Timestamp::from(u64::MAX));
    let events = test_db.db.query(filter).await.unwrap();
    assert_eq!(events.len(), 2);
    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_event_with_empty_content() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Event with empty content
    let event = EventBuilder::text_note("").sign_with_keys(&keys).unwrap();

    let status = test_db.db.save_event(&event).await.unwrap();
    assert_eq!(status, SaveEventStatus::Success);

    // Retrieve and verify
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content, "");

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_event_with_unicode_content() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Event with various unicode characters
    let unicode_content = "Hello 世界 🌍 Привет مرحبا";
    let event = EventBuilder::text_note(unicode_content)
        .sign_with_keys(&keys)
        .unwrap();

    test_db.db.save_event(&event).await.unwrap();

    // Retrieve and verify unicode is preserved
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content, unicode_content);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_multiple_deletes_same_event() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create and save event
    let event = EventBuilder::text_note("To be deleted multiple times")
        .sign_with_keys(&keys)
        .unwrap();
    test_db.db.save_event(&event).await.unwrap();

    // Delete once
    let filter = Filter::new().id(event.id);
    test_db
        .db
        .delete(filter.clone())
        .await
        .expect("Failed to delete event");

    // Delete again (should not error)
    test_db
        .db
        .delete(filter)
        .await
        .expect("Failed to delete event again");

    // Verify still deleted
    let status = test_db.db.check_id(&event.id).await.unwrap();
    assert_eq!(status, DatabaseEventStatus::Deleted);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_query_ordering() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create events with specific timestamps
    let event1 = EventBuilder::text_note("First")
        .custom_created_at(Timestamp::from(1000))
        .sign_with_keys(&keys)
        .unwrap();
    let event2 = EventBuilder::text_note("Second")
        .custom_created_at(Timestamp::from(2000))
        .sign_with_keys(&keys)
        .unwrap();
    let event3 = EventBuilder::text_note("Third")
        .custom_created_at(Timestamp::from(3000))
        .sign_with_keys(&keys)
        .unwrap();

    // Save in random order
    test_db.db.save_event(&event2).await.unwrap();
    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event3).await.unwrap();

    // Query should return in descending timestamp order
    let filter = Filter::new().author(keys.public_key());
    let events = test_db.db.query(filter).await.unwrap();

    let timestamps: Vec<u64> = events.iter().map(|e| e.created_at.as_secs()).collect();
    assert_eq!(timestamps, vec![3000, 2000, 1000]);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_event_with_special_kinds() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Test various event kinds
    let kinds_to_test = vec![
        Kind::Metadata,
        Kind::TextNote,
        Kind::RecommendRelay,
        Kind::ContactList,
        Kind::EncryptedDirectMessage,
        Kind::Custom(10000),
        Kind::Custom(30000),
    ];

    for kind in kinds_to_test {
        let event = EventBuilder::new(kind, "").sign_with_keys(&keys).unwrap();
        let status = test_db.db.save_event(&event).await.unwrap();
        assert_eq!(status, SaveEventStatus::Success);
    }

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_concurrent_operations() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create multiple events concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let db_clone = test_db.db.clone();
        let keys_clone = keys.clone();
        let handle = tokio::spawn(async move {
            let event = EventBuilder::text_note(format!("Concurrent event {}", i))
                .sign_with_keys(&keys_clone)
                .unwrap();
            db_clone.save_event(&event).await.unwrap()
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, SaveEventStatus::Success);
    }

    // Verify all were saved
    let count = test_db
        .db
        .count(Filter::new().author(keys.public_key()))
        .await
        .unwrap();
    assert_eq!(count, 10);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_tag_with_empty_value() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create event with custom tag with empty value
    let event = EventBuilder::text_note("Event with empty tag")
        .tags([Tag::hashtag("")])
        .sign_with_keys(&keys)
        .unwrap();

    test_db.db.save_event(&event).await.unwrap();

    // Should be retrievable
    let retrieved = test_db.db.event_by_id(&event.id).await.unwrap();
    assert!(retrieved.is_some());

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_multiple_authors_same_content() {
    let test_db = setup_test_db().await;
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();

    // Same content from different authors
    let content = "Identical content";
    let event1 = EventBuilder::text_note(content)
        .sign_with_keys(&keys1)
        .unwrap();
    let event2 = EventBuilder::text_note(content)
        .sign_with_keys(&keys2)
        .unwrap();

    test_db.db.save_event(&event1).await.unwrap();
    test_db.db.save_event(&event2).await.unwrap();

    // Both should be saved (different IDs)
    let retrieved1 = test_db.db.event_by_id(&event1.id).await.unwrap();
    let retrieved2 = test_db.db.event_by_id(&event2.id).await.unwrap();

    assert!(retrieved1.is_some());
    assert!(retrieved2.is_some());
    assert_ne!(event1.id, event2.id);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_limit_zero() {
    let test_db = setup_test_db().await;
    let keys = Keys::generate();

    // Create some events
    for i in 0..5 {
        let event = EventBuilder::text_note(format!("Event {}", i))
            .sign_with_keys(&keys)
            .unwrap();
        test_db.db.save_event(&event).await.unwrap();
    }

    // Query with limit 0
    let filter = Filter::new().author(keys.public_key()).limit(0);
    let events = test_db.db.query(filter).await.unwrap();

    // Should return 0 results
    assert_eq!(events.len(), 0);

    cleanup_test_db(&test_db).await;
}

#[tokio::test]
async fn test_filter_combination_no_matches() {
    let test_db = setup_test_db().await;
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();

    // Create event from keys1
    let event = EventBuilder::text_note("Test")
        .sign_with_keys(&keys1)
        .unwrap();
    test_db.db.save_event(&event).await.unwrap();

    // Query for different author with same kind
    let filter = Filter::new()
        .author(keys2.public_key())
        .kind(Kind::TextNote);
    let events = test_db.db.query(filter).await.unwrap();

    assert_eq!(events.len(), 0);

    cleanup_test_db(&test_db).await;
}

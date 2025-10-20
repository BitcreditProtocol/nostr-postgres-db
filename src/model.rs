use std::sync::{Mutex, OnceLock};

use nostr::event::Event;
use nostr_database::{DatabaseError, FlatBufferBuilder, FlatBufferEncode};
use tokio_postgres::Row;

/// DB representation of [`Event`]
#[derive(Debug, Clone)]
pub struct EventDb {
    pub id: Vec<u8>,
    pub pubkey: Vec<u8>,
    pub created_at: i64,
    pub kind: i64,
    pub payload: Vec<u8>,
    pub deleted: bool,
}

impl From<Row> for EventDb {
    fn from(row: Row) -> Self {
        Self {
            id: row.get(0),
            pubkey: row.get(1),
            created_at: row.get(2),
            kind: row.get(3),
            payload: row.get(4),
            deleted: row.get(5),
        }
    }
}

/// DB representation of [`EventTag`]
#[derive(Debug, Clone)]
pub struct EventTagDb {
    pub tag: String,
    pub tag_value: String,
    pub event_id: Vec<u8>,
}

impl From<Row> for EventTagDb {
    fn from(row: Row) -> Self {
        Self {
            tag: row.get(0),
            tag_value: row.get(1),
            event_id: row.get(2),
        }
    }
}

/// A data container for extracting data from [`Event`] and its tags
#[derive(Debug, Clone)]
pub struct EventDataDb {
    pub event: EventDb,
    pub tags: Vec<EventTagDb>,
}

impl TryFrom<&Event> for EventDataDb {
    type Error = DatabaseError;
    fn try_from(value: &Event) -> Result<Self, Self::Error> {
        Ok(Self {
            event: EventDb {
                id: value.id.as_bytes().to_vec(),
                pubkey: value.pubkey.as_bytes().to_vec(),
                created_at: value.created_at.as_u64() as i64,
                kind: value.kind.as_u16() as i64,
                payload: encode_payload(value),
                deleted: false,
            },
            tags: extract_tags(value),
        })
    }
}

fn encode_payload(value: &Event) -> Vec<u8> {
    static FB_BUILDER: OnceLock<Mutex<FlatBufferBuilder>> = OnceLock::new();
    match FB_BUILDER
        .get_or_init(|| Mutex::new(FlatBufferBuilder::new()))
        .lock()
    {
        Ok(mut fb_builder) => value.encode(&mut fb_builder).to_vec(),
        Err(_) => value.encode(&mut FlatBufferBuilder::new()).to_vec(),
    }
}

fn extract_tags(event: &Event) -> Vec<EventTagDb> {
    event
        .tags
        .iter()
        .filter_map(|tag| {
            if let (kind, Some(content)) = (tag.kind(), tag.content()) {
                Some(EventTagDb {
                    tag: kind.to_string(),
                    tag_value: content.to_string(),
                    event_id: event.id.as_bytes().to_vec(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::prelude::*;
    use nostr_database::FlatBufferDecode;

    #[test]
    fn test_event_data_db_try_from_event() {
        let keys = Keys::generate();
        let event = EventBuilder::text_note("test content")
            .sign_with_keys(&keys)
            .unwrap();

        let event_data = EventDataDb::try_from(&event).unwrap();

        assert_eq!(event_data.event.id, event.id.as_bytes().to_vec());
        assert_eq!(event_data.event.pubkey, event.pubkey.as_bytes().to_vec());
        assert_eq!(
            event_data.event.created_at,
            event.created_at.as_u64() as i64
        );
        assert_eq!(event_data.event.kind, event.kind.as_u16() as i64);
        assert!(!event_data.event.deleted);
        assert!(!event_data.event.payload.is_empty());
    }

    #[test]
    fn test_event_data_db_with_tags() {
        let keys = Keys::generate();
        let other_pubkey = Keys::generate().public_key();
        let event = EventBuilder::text_note("test content")
            .tags([
                Tag::public_key(other_pubkey),
                Tag::event(EventId::all_zeros()),
                Tag::hashtag("nostr"),
            ])
            .sign_with_keys(&keys)
            .unwrap();

        let event_data = EventDataDb::try_from(&event).unwrap();

        assert_eq!(event_data.tags.len(), 3);

        // Check that tags are properly extracted
        let tag_types: Vec<String> = event_data.tags.iter().map(|t| t.tag.clone()).collect();
        assert!(tag_types.contains(&"p".to_string()));
        assert!(tag_types.contains(&"e".to_string()));
        assert!(tag_types.contains(&"t".to_string()));

        // All tags should reference the same event
        for tag in &event_data.tags {
            assert_eq!(tag.event_id, event.id.as_bytes().to_vec());
        }
    }

    #[test]
    fn test_extract_tags_empty() {
        let keys = Keys::generate();
        let event = EventBuilder::text_note("no tags")
            .sign_with_keys(&keys)
            .unwrap();

        let tags = extract_tags(&event);
        assert_eq!(tags.len(), 0);
    }

    #[test]
    fn test_extract_tags_with_various_types() {
        let keys = Keys::generate();
        let other_pubkey = Keys::generate().public_key();
        let event_id = EventId::all_zeros();

        let event = EventBuilder::text_note("test")
            .tags([
                Tag::public_key(other_pubkey),
                Tag::event(event_id),
                Tag::hashtag("rust"),
                Tag::identifier("test-id"),
            ])
            .sign_with_keys(&keys)
            .unwrap();

        let tags = extract_tags(&event);

        // Should have at least p, e, t, and d tags
        assert!(tags.len() >= 4);

        let tag_types: Vec<String> = tags.iter().map(|t| t.tag.clone()).collect();
        assert!(tag_types.contains(&"p".to_string()));
        assert!(tag_types.contains(&"e".to_string()));
        assert!(tag_types.contains(&"t".to_string()));
        assert!(tag_types.contains(&"d".to_string()));
    }

    #[test]
    fn test_encode_payload_produces_valid_flatbuffer() {
        let keys = Keys::generate();
        let event = EventBuilder::text_note("test content")
            .sign_with_keys(&keys)
            .unwrap();

        let payload = encode_payload(&event);

        // Should produce a non-empty payload
        assert!(!payload.is_empty());

        // Should be able to decode the payload back
        let decoded = Event::decode(&payload).unwrap();
        assert_eq!(decoded.id, event.id);
        assert_eq!(decoded.content, event.content);
    }

    #[test]
    fn test_event_db_fields() {
        let event_db = EventDb {
            id: vec![1, 2, 3],
            pubkey: vec![4, 5, 6],
            created_at: 1234567890,
            kind: 1,
            payload: vec![7, 8, 9],
            deleted: false,
        };

        assert_eq!(event_db.id, vec![1, 2, 3]);
        assert_eq!(event_db.pubkey, vec![4, 5, 6]);
        assert_eq!(event_db.created_at, 1234567890);
        assert_eq!(event_db.kind, 1);
        assert_eq!(event_db.payload, vec![7, 8, 9]);
        assert!(!event_db.deleted);
    }

    #[test]
    fn test_event_tag_db_fields() {
        let tag_db = EventTagDb {
            tag: "e".to_string(),
            tag_value: "test_value".to_string(),
            event_id: vec![1, 2, 3],
        };

        assert_eq!(tag_db.tag, "e");
        assert_eq!(tag_db.tag_value, "test_value");
        assert_eq!(tag_db.event_id, vec![1, 2, 3]);
    }
}

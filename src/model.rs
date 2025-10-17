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

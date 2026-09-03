// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! FlatBuffers codec for the `events.payload` column.
//!
//! `nostr-database` shipped this schema and codec behind its `flatbuf` feature up to
//! 0.44 and removed both in 0.45. They are vendored here so the stored format stays
//! the same: payloads written by earlier versions of this crate decode unchanged.
//!
//! `event_generated.rs` is produced by `flatc` from `event.fbs`; do not edit it by hand.

use std::fmt;

pub use flatbuffers::FlatBufferBuilder;
use flatbuffers::InvalidFlatbuffer;
use nostr::event::{Event, EventId, Kind, Signature, Tag};
use nostr::key::PublicKey;
use nostr::types::Timestamp;

#[allow(
    unused_imports,
    dead_code,
    clippy::all,
    unsafe_code,
    missing_docs,
    unsafe_op_in_unsafe_fn
)]
mod event_generated;

use self::event_generated::event_fbs;

/// FlatBuffers codec error
#[derive(Debug)]
pub enum Error {
    /// The buffer is not a valid event FlatBuffer
    FlatBuffer(InvalidFlatbuffer),
    /// A field could not be turned back into its nostr type (tag, signature, ...)
    Nostr(nostr::error::Error),
    /// A required field is missing from the buffer
    MissingField(&'static str),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FlatBuffer(e) => Some(e),
            Self::Nostr(e) => Some(e),
            Self::MissingField(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FlatBuffer(e) => write!(f, "invalid flatbuffer: {e}"),
            Self::Nostr(e) => write!(f, "{e}"),
            Self::MissingField(field) => write!(f, "missing field: {field}"),
        }
    }
}

impl From<InvalidFlatbuffer> for Error {
    fn from(e: InvalidFlatbuffer) -> Self {
        Self::FlatBuffer(e)
    }
}

impl From<nostr::error::Error> for Error {
    fn from(e: nostr::error::Error) -> Self {
        Self::Nostr(e)
    }
}

/// Encode `event` with `fbb` and return the finished buffer
///
/// The builder is reset first, so it can be reused across calls.
pub fn encode_event<'a>(event: &Event, fbb: &'a mut FlatBufferBuilder) -> &'a [u8] {
    fbb.reset();

    let id = event_fbs::Fixed32Bytes::new(event.id.as_bytes());
    let pubkey = event_fbs::Fixed32Bytes::new(event.pubkey.as_bytes());
    let sig = event_fbs::Fixed64Bytes::new(event.sig.as_bytes());
    let tags = event
        .tags
        .iter()
        .map(|tag| {
            let values = tag
                .as_slice()
                .iter()
                .map(|value| fbb.create_string(value))
                .collect::<Vec<_>>();
            let args = event_fbs::StringVectorArgs {
                data: Some(fbb.create_vector(&values)),
            };
            event_fbs::StringVector::create(fbb, &args)
        })
        .collect::<Vec<_>>();
    let args = event_fbs::EventArgs {
        id: Some(&id),
        pubkey: Some(&pubkey),
        created_at: event.created_at.as_secs(),
        kind: event.kind.as_u16() as u64,
        tags: Some(fbb.create_vector(&tags)),
        content: Some(fbb.create_string(&event.content)),
        sig: Some(&sig),
    };

    let offset = event_fbs::Event::create(fbb, &args);
    event_fbs::finish_event_buffer(fbb, offset);

    fbb.finished_data()
}

/// Decode an event previously written by [`encode_event`]
pub fn decode_event(buf: &[u8]) -> Result<Event, Error> {
    let ev = event_fbs::root_as_event(buf)?;
    let tags = ev
        .tags()
        .ok_or(Error::MissingField("tags"))?
        .into_iter()
        .filter_map(|tag| tag.data().map(Tag::parse))
        .collect::<Result<Vec<Tag>, _>>()?;

    Ok(Event::new(
        EventId::from_byte_array(ev.id().ok_or(Error::MissingField("id"))?.0),
        PublicKey::from_byte_array(ev.pubkey().ok_or(Error::MissingField("pubkey"))?.0),
        Timestamp::from_secs(ev.created_at()),
        Kind::from_u16(ev.kind() as u16),
        tags,
        ev.content().ok_or(Error::MissingField("content"))?,
        Signature::from_slice(&ev.sig().ok_or(Error::MissingField("sig"))?.0)?,
    ))
}

#[cfg(test)]
mod tests {
    use nostr::event::{EventBuilder, FinalizeEvent};
    use nostr::key::Keys;

    use super::*;

    #[test]
    fn round_trip_preserves_every_field() {
        let keys = Keys::generate();
        let event = EventBuilder::new(Kind::Custom(30023), "hello 世界 🌍")
            .tags([
                Tag::public_key(Keys::generate().public_key()),
                Tag::event(EventId::from_byte_array([0; 32])),
                Tag::hashtag("nostr"),
                Tag::identifier("article-1"),
                Tag::parse(["x", "a", "b", "c"]).unwrap(),
            ])
            .custom_created_at(Timestamp::from_secs(1_700_000_000))
            .finalize(&keys)
            .unwrap();

        let mut fbb = FlatBufferBuilder::new();
        let payload = encode_event(&event, &mut fbb).to_vec();
        let decoded = decode_event(&payload).unwrap();

        assert_eq!(decoded, event);
        assert_eq!(decoded.tags.len(), 5);
        assert!(decoded.verify().is_ok());
    }

    #[test]
    fn builder_is_reusable() {
        let keys = Keys::generate();
        let first = EventBuilder::new(Kind::TextNote, "first")
            .finalize(&keys)
            .unwrap();
        let second = EventBuilder::new(Kind::TextNote, "second")
            .finalize(&keys)
            .unwrap();

        let mut fbb = FlatBufferBuilder::new();
        let first_payload = encode_event(&first, &mut fbb).to_vec();
        let second_payload = encode_event(&second, &mut fbb).to_vec();

        assert_eq!(decode_event(&first_payload).unwrap(), first);
        assert_eq!(decode_event(&second_payload).unwrap(), second);
    }

    /// Payload written by `nostr-database` 0.44 with its `flatbuf` feature, i.e. the
    /// codec this crate used before vendoring the schema. Stored rows must keep decoding.
    const LEGACY_PAYLOAD_HEX: &str = "1c00000000000000000012009c00040024008c009400440048004c001200000040a6f374a85d2dc1e5a8f1dfa95d27851754eb1dd493a7c593605511a02f2a0979be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f817987000000054000000efe03c35eaa2c683ba26175d83ffdbfb04d6065d2c5da777a3520c20d4a92d2ea336b94034f04300af8a46467f472084f989d66890bdc1464da757b46aa4410700f15365000000000100000000000000130000006c6567616379207061796c6f616420f09f8c8d00030000006c0000003800000004000000a6ffffff04000000030000001c0000001000000004000000010000006200000001000000610000000100000078000000d6ffffff04000000020000001800000004000000080000006c65676163792d31000000000100000064000600080004000600000004000000020000001400000004000000050000006e6f7374720000000100000074000000";

    fn unhex(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn decodes_payloads_written_by_nostr_database_0_44() {
        let event = decode_event(&unhex(LEGACY_PAYLOAD_HEX)).unwrap();

        assert_eq!(
            event.id.to_hex(),
            "40a6f374a85d2dc1e5a8f1dfa95d27851754eb1dd493a7c593605511a02f2a09"
        );
        assert_eq!(
            event.pubkey.to_hex(),
            "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
        );
        assert_eq!(event.created_at, Timestamp::from_secs(1_700_000_000));
        assert_eq!(event.kind, Kind::TextNote);
        assert_eq!(event.content, "legacy payload 🌍");
        let tags: Vec<&[String]> = event.tags.iter().map(|t| t.as_slice()).collect();
        assert_eq!(
            tags,
            [&["t", "nostr"][..], &["d", "legacy-1"], &["x", "a", "b"]]
        );
        assert!(event.verify().is_ok());

        // and re-encoding with the vendored codec is byte-for-byte identical
        let mut fbb = FlatBufferBuilder::new();
        assert_eq!(encode_event(&event, &mut fbb), unhex(LEGACY_PAYLOAD_HEX));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(matches!(
            decode_event(b"definitely not a flatbuffer"),
            Err(Error::FlatBuffer(_))
        ));
    }
}

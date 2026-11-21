//! InitProducerId (API key 22), versions 0..=4.
//!
//! The Java client's idempotent producer (`enable.idempotence=true`, the default since Kafka 3.0)
//! calls InitProducerId right after connecting, before the first Produce, to obtain a
//! `(producer_id, producer_epoch)` pair. Without this API the broker answered with an
//! `ApiVersions` response and the client failed with `UnsupportedVersionException: The node does
//! not support INIT_PRODUCER_ID`.
//!
//! Versions: v0-v1 use the classic encoding, v2+ the flexible one (compact strings, tag buffers,
//! response header v1). v3 added `producer_id` / `producer_epoch` to the request so a producer can
//! ask for an epoch bump of its existing id (KIP-360). Transactions themselves
//! (AddPartitionsToTxn, EndTxn, ...) are not implemented: a `transactional_id` just gets a stable
//! id whose epoch is bumped on every re-initialisation, which is what a fenced-producer test needs.

use std::collections::HashMap;

use crate::produce_codec::{
    push_unsigned_varint, read_compact_nullable_string, read_i16, read_i32, read_i64,
    skip_tag_buffer, take,
};

/// First flexible version of InitProducerId.
pub const INIT_PRODUCER_ID_FIRST_FLEXIBLE_VERSION: i16 = 2;
/// Lowest supported version.
pub const INIT_PRODUCER_ID_MIN_VERSION: i16 = 0;
/// Highest supported version.
pub const INIT_PRODUCER_ID_MAX_VERSION: i16 = 4;

/// Parsed InitProducerId request body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitProducerIdRequest {
    /// `None` (or empty) for a plain idempotent producer.
    pub transactional_id: Option<String>,
    pub transaction_timeout_ms: i32,
    /// `-1` unless the client asks for an epoch bump of an existing id (v3+).
    pub producer_id: i64,
    /// `-1` unless the client asks for an epoch bump of an existing id (v3+).
    pub producer_epoch: i16,
}

/// Response body values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitProducerIdResponse {
    pub error_code: i16,
    pub producer_id: i64,
    pub producer_epoch: i16,
}

/// Parse an InitProducerId request body (everything after the request header).
pub fn parse_init_producer_id(version: i16, body: &[u8]) -> Result<InitProducerIdRequest, String> {
    if !(INIT_PRODUCER_ID_MIN_VERSION..=INIT_PRODUCER_ID_MAX_VERSION).contains(&version) {
        return Err(format!("unsupported InitProducerId version {version}"));
    }
    let mut buf = body;
    let flexible = version >= INIT_PRODUCER_ID_FIRST_FLEXIBLE_VERSION;

    let transactional_id = if flexible {
        read_compact_nullable_string(&mut buf)?
    } else {
        let len = read_i16(&mut buf)?;
        if len < 0 {
            None
        } else {
            let bytes = take(&mut buf, len as usize)?;
            Some(String::from_utf8(bytes.to_vec()).map_err(|e| format!("invalid utf8: {e}"))?)
        }
    };
    let transaction_timeout_ms = read_i32(&mut buf)?;
    let (producer_id, producer_epoch) = if version >= 3 {
        (read_i64(&mut buf)?, read_i16(&mut buf)?)
    } else {
        (-1, -1)
    };
    if flexible {
        skip_tag_buffer(&mut buf)?;
    }
    Ok(InitProducerIdRequest {
        transactional_id: transactional_id.filter(|id| !id.is_empty()),
        transaction_timeout_ms,
        producer_id,
        producer_epoch,
    })
}

/// Serialize the full response, including the response header (correlation id, plus a tag buffer
/// for flexible versions), as `KafkaResponse::Preserialized` expects.
pub fn serialize_init_producer_id_response(
    version: i16,
    correlation_id: i32,
    response: &InitProducerIdResponse,
) -> Vec<u8> {
    let flexible = version >= INIT_PRODUCER_ID_FIRST_FLEXIBLE_VERSION;
    let mut out = Vec::with_capacity(24);
    out.extend_from_slice(&correlation_id.to_be_bytes());
    if flexible {
        push_unsigned_varint(&mut out, 0); // response header v1 tag buffer
    }
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&response.error_code.to_be_bytes());
    out.extend_from_slice(&response.producer_id.to_be_bytes());
    out.extend_from_slice(&response.producer_epoch.to_be_bytes());
    if flexible {
        push_unsigned_varint(&mut out, 0); // body tag buffer
    }
    out
}

/// Hands out producer ids and tracks their epochs.
#[derive(Debug)]
pub struct ProducerIdAllocator {
    next_id: i64,
    /// `transactional_id` -> (producer_id, current epoch)
    transactional: HashMap<String, (i64, i16)>,
}

impl Default for ProducerIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProducerIdAllocator {
    pub fn new() -> Self {
        // Start above 0 so a zero-initialised id is never mistaken for a real one.
        Self {
            next_id: 1000,
            transactional: HashMap::new(),
        }
    }

    fn fresh_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Allocate (or re-initialise) a producer id for `request`.
    ///
    /// * New plain producer: fresh id, epoch 0.
    /// * Known `transactional_id`: same id, epoch + 1 (fences the previous instance).
    /// * v3+ request carrying an existing `producer_id`: same id, epoch + 1.
    /// * An epoch at `i16::MAX` cannot be bumped: a fresh id with epoch 0 is returned instead.
    pub fn init(&mut self, request: &InitProducerIdRequest) -> InitProducerIdResponse {
        let existing = match &request.transactional_id {
            Some(tid) => self.transactional.get(tid).copied(),
            None if request.producer_id >= 0 => {
                Some((request.producer_id, request.producer_epoch.max(0)))
            }
            None => None,
        };
        let (producer_id, producer_epoch) = match existing {
            Some((id, epoch)) if epoch < i16::MAX => (id, epoch + 1),
            _ => (self.fresh_id(), 0),
        };
        if let Some(tid) = &request.transactional_id {
            self.transactional.insert(tid.clone(), (producer_id, producer_epoch));
        }
        InitProducerIdResponse {
            error_code: 0,
            producer_id,
            producer_epoch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> InitProducerIdRequest {
        InitProducerIdRequest {
            transactional_id: None,
            transaction_timeout_ms: 60_000,
            producer_id: -1,
            producer_epoch: -1,
        }
    }

    #[test]
    fn parses_v1_null_transactional_id() {
        // int16 -1 (null), int32 timeout
        let mut body = (-1i16).to_be_bytes().to_vec();
        body.extend_from_slice(&60_000i32.to_be_bytes());
        assert_eq!(
            parse_init_producer_id(1, &body).unwrap(),
            InitProducerIdRequest {
                producer_id: -1,
                producer_epoch: -1,
                ..plain()
            }
        );
    }

    #[test]
    fn parses_v1_transactional_id() {
        let mut body = 3i16.to_be_bytes().to_vec();
        body.extend_from_slice(b"txn");
        body.extend_from_slice(&5000i32.to_be_bytes());
        let req = parse_init_producer_id(0, &body).unwrap();
        assert_eq!(req.transactional_id.as_deref(), Some("txn"));
        assert_eq!(req.transaction_timeout_ms, 5000);
    }

    #[test]
    fn parses_v4_flexible_with_existing_producer() {
        // compact null string (0), timeout, producer_id, epoch, empty tag buffer
        let mut body = vec![0u8];
        body.extend_from_slice(&60_000i32.to_be_bytes());
        body.extend_from_slice(&1234i64.to_be_bytes());
        body.extend_from_slice(&7i16.to_be_bytes());
        body.push(0);
        let req = parse_init_producer_id(4, &body).unwrap();
        assert_eq!(req.transactional_id, None);
        assert_eq!((req.producer_id, req.producer_epoch), (1234, 7));
    }

    #[test]
    fn parses_v2_flexible_compact_transactional_id() {
        let mut body = vec![4u8]; // len 3 + 1
        body.extend_from_slice(b"txn");
        body.extend_from_slice(&1i32.to_be_bytes());
        body.push(0);
        let req = parse_init_producer_id(2, &body).unwrap();
        assert_eq!(req.transactional_id.as_deref(), Some("txn"));
        assert_eq!(req.producer_id, -1);
    }

    #[test]
    fn rejects_truncated_body_and_unsupported_version() {
        assert!(parse_init_producer_id(1, &[0, 0]).is_err());
        assert!(parse_init_producer_id(9, &[]).is_err());
    }

    #[test]
    fn serializes_classic_and_flexible_responses() {
        let resp = InitProducerIdResponse {
            error_code: 0,
            producer_id: 1000,
            producer_epoch: 0,
        };
        let v1 = serialize_init_producer_id_response(1, 42, &resp);
        // correlation(4) throttle(4) error(2) id(8) epoch(2)
        assert_eq!(v1.len(), 20);
        assert_eq!(&v1[..4], &42i32.to_be_bytes());
        assert_eq!(&v1[10..18], &1000i64.to_be_bytes());

        let v4 = serialize_init_producer_id_response(4, 42, &resp);
        // + header tag buffer + body tag buffer
        assert_eq!(v4.len(), 22);
        assert_eq!(v4[4], 0, "header tag buffer");
        assert_eq!(*v4.last().unwrap(), 0, "body tag buffer");
    }

    #[test]
    fn allocates_distinct_ids_for_plain_producers() {
        let mut a = ProducerIdAllocator::new();
        let first = a.init(&plain());
        let second = a.init(&plain());
        assert_ne!(first.producer_id, second.producer_id);
        assert_eq!((first.producer_epoch, second.producer_epoch), (0, 0));
        assert_eq!(first.error_code, 0);
    }

    #[test]
    fn bumps_epoch_for_known_transactional_id() {
        let mut a = ProducerIdAllocator::new();
        let req = InitProducerIdRequest {
            transactional_id: Some("t".into()),
            ..plain()
        };
        let first = a.init(&req);
        let second = a.init(&req);
        assert_eq!(second.producer_id, first.producer_id);
        assert_eq!(second.producer_epoch, first.producer_epoch + 1);
    }

    #[test]
    fn bumps_epoch_when_client_presents_its_producer_id() {
        let mut a = ProducerIdAllocator::new();
        let first = a.init(&plain());
        let bump = a.init(&InitProducerIdRequest {
            producer_id: first.producer_id,
            producer_epoch: first.producer_epoch,
            ..plain()
        });
        assert_eq!(bump.producer_id, first.producer_id);
        assert_eq!(bump.producer_epoch, 1);
    }

    #[test]
    fn exhausted_epoch_gets_a_fresh_id() {
        let mut a = ProducerIdAllocator::new();
        let resp = a.init(&InitProducerIdRequest {
            producer_id: 5,
            producer_epoch: i16::MAX,
            ..plain()
        });
        assert_ne!(resp.producer_id, 5);
        assert_eq!(resp.producer_epoch, 0);
    }
}

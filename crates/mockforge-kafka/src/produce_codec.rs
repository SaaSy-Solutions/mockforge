//! Produce v9 wire-format codec.
//!
//! Produce v9 is the first "flexible" version of the Produce API: request
//! and response bodies use compact arrays (unsigned-varint length+1),
//! compact strings (unsigned-varint length+1 + bytes), and tag buffers after
//! every struct. Records are carried as a `RecordBatch` v2 blob inside a
//! compact bytes field.
//!
//! The job of this module is:
//!
//!   1. Parse a produce v9 request body into topics -> partitions -> records.
//!   2. Parse each partition's RecordBatch v2 into individual records
//!      (ZigZag signed varints for record deltas/lengths).
//!   3. Serialize a produce v9 response body describing the offsets each
//!      topic/partition ended up at.
//!
//! The broker is responsible for actually writing records into topic storage.
//! This module only decodes and encodes.

/// A record extracted from a RecordBatch v2.
///
/// Offsets are not set here — the broker assigns them when calling
/// `Topic::produce()`.
#[derive(Debug, Clone)]
pub struct DecodedRecord {
    pub timestamp_ms: i64,
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: Vec<(String, Vec<u8>)>,
}

/// One partition's worth of records within a Produce request.
#[derive(Debug, Clone)]
pub struct PartitionProduceData {
    pub partition_index: i32,
    pub records: Vec<DecodedRecord>,
    /// Low 3 bits of the incoming batch's attributes field — i.e. the
    /// compression codec as it arrived on the wire. `records` is always
    /// the uncompressed form regardless of this value; unknown codecs
    /// (bits 5-7) surface as a parse error upstream instead of reaching
    /// this struct. Kept around for observability and for any future
    /// policy that wants to honor a "respond with same compression" rule.
    pub compression_codec: i8,
}

/// One topic's worth of produce data.
#[derive(Debug, Clone)]
pub struct TopicProduceData {
    pub name: String,
    pub partitions: Vec<PartitionProduceData>,
}

/// Parsed Produce v9 request body.
#[derive(Debug, Clone)]
pub struct ProduceRequestV9 {
    pub transactional_id: Option<String>,
    pub acks: i16,
    pub timeout_ms: i32,
    pub topics: Vec<TopicProduceData>,
}

/// Per-partition result the broker feeds back to build a response.
#[derive(Debug, Clone, Copy)]
pub struct PartitionProduceResult {
    pub partition_index: i32,
    /// Kafka error code; 0 = success.
    pub error_code: i16,
    /// Offset of the first record written — undefined when error_code != 0.
    pub base_offset: i64,
    /// Wall-clock ms when the broker appended the batch. -1 when N/A.
    pub log_append_time_ms: i64,
    /// Lowest available offset for the partition. 0 in our mock since we
    /// never expire segments.
    pub log_start_offset: i64,
}

/// Per-topic aggregation of partition results.
#[derive(Debug, Clone)]
pub struct TopicProduceResult {
    pub name: String,
    pub partitions: Vec<PartitionProduceResult>,
}

// =========================================================================
// Wire-format primitives (cursor-style)
// =========================================================================

pub(crate) fn take<'a>(buf: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
    if buf.len() < n {
        return Err(format!("short read: wanted {n}, have {}", buf.len()));
    }
    let (head, tail) = buf.split_at(n);
    *buf = tail;
    Ok(head)
}

pub(crate) fn read_i8(buf: &mut &[u8]) -> Result<i8, String> {
    Ok(take(buf, 1)?[0] as i8)
}

pub(crate) fn read_i16(buf: &mut &[u8]) -> Result<i16, String> {
    let b = take(buf, 2)?;
    Ok(i16::from_be_bytes([b[0], b[1]]))
}

pub(crate) fn read_i32(buf: &mut &[u8]) -> Result<i32, String> {
    let b = take(buf, 4)?;
    Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn read_i64(buf: &mut &[u8]) -> Result<i64, String> {
    let b = take(buf, 8)?;
    Ok(i64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// Kafka "unsigned varint" — little-endian 7 bits per byte, continuation in
/// high bit. Used for compact array/string lengths and tag buffer counts.
pub fn read_unsigned_varint(buf: &mut &[u8]) -> Result<u32, String> {
    let mut value: u32 = 0;
    let mut shift: u32 = 0;
    loop {
        if buf.is_empty() {
            return Err("truncated unsigned varint".into());
        }
        let byte = buf[0];
        *buf = &buf[1..];
        value |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            return Ok(value);
        }
        shift += 7;
        if shift >= 32 {
            return Err("unsigned varint overflow".into());
        }
    }
}

/// ZigZag-encoded signed varint. Used inside RecordBatch v2 for every
/// record-level length/delta field (NOT the same as the unsigned varint
/// used in compact arrays at the batch/request level).
pub fn read_signed_varint(buf: &mut &[u8]) -> Result<i64, String> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if buf.is_empty() {
            return Err("truncated signed varint".into());
        }
        let byte = buf[0];
        *buf = &buf[1..];
        value |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            // Unzigzag
            let signed = ((value >> 1) as i64) ^ -((value & 1) as i64);
            return Ok(signed);
        }
        shift += 7;
        if shift >= 64 {
            return Err("signed varint overflow".into());
        }
    }
}

/// Read a compact string. `unsigned_varint(len + 1)` then bytes.
/// A leading 0 means null; 1 means empty string.
pub(crate) fn read_compact_nullable_string(buf: &mut &[u8]) -> Result<Option<String>, String> {
    let len_plus_one = read_unsigned_varint(buf)?;
    if len_plus_one == 0 {
        return Ok(None);
    }
    let len = (len_plus_one - 1) as usize;
    let bytes = take(buf, len)?;
    String::from_utf8(bytes.to_vec())
        .map(Some)
        .map_err(|e| format!("invalid utf8: {e}"))
}

pub(crate) fn read_compact_string(buf: &mut &[u8]) -> Result<String, String> {
    read_compact_nullable_string(buf)?.ok_or_else(|| "expected non-null compact string".into())
}

/// Read a compact bytes field. A leading 0 means null.
pub(crate) fn read_compact_nullable_bytes<'a>(
    buf: &mut &'a [u8],
) -> Result<Option<&'a [u8]>, String> {
    let len_plus_one = read_unsigned_varint(buf)?;
    if len_plus_one == 0 {
        return Ok(None);
    }
    let len = (len_plus_one - 1) as usize;
    let bytes = take(buf, len)?;
    Ok(Some(bytes))
}

/// Tag buffer: read the count, then for each tag, skip its length+1 bytes.
/// We don't act on any tagged fields today — this just advances the cursor
/// past them.
pub(crate) fn skip_tag_buffer(buf: &mut &[u8]) -> Result<(), String> {
    let count = read_unsigned_varint(buf)?;
    for _ in 0..count {
        let _tag_id = read_unsigned_varint(buf)?;
        let len = read_unsigned_varint(buf)? as usize;
        take(buf, len)?;
    }
    Ok(())
}

// =========================================================================
// Output helpers
// =========================================================================

pub(crate) fn push_unsigned_varint(buf: &mut Vec<u8>, mut value: u32) {
    while (value & !0x7F) != 0 {
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buf.push(value as u8);
}

pub(crate) fn push_signed_varint(buf: &mut Vec<u8>, value: i64) {
    let zz = ((value << 1) ^ (value >> 63)) as u64;
    let mut v = zz;
    while (v & !0x7F) != 0 {
        buf.push(((v & 0x7F) | 0x80) as u8);
        v >>= 7;
    }
    buf.push(v as u8);
}

pub(crate) fn push_compact_string(buf: &mut Vec<u8>, s: &str) {
    push_unsigned_varint(buf, (s.len() as u32) + 1);
    buf.extend_from_slice(s.as_bytes());
}

pub(crate) fn push_empty_tag_buffer(buf: &mut Vec<u8>) {
    buf.push(0);
}

// =========================================================================
// Record batch v2 parser
// =========================================================================

/// Parse one RecordBatch v2 out of `batch_bytes` and return the decoded
/// records. Supports the 4 standard compression codecs (gzip / snappy /
/// lz4 / zstd) — the records blob is decompressed inline before the
/// framed-record iteration, so callers don't need to distinguish.
/// Unknown codecs (attributes bits 5–7) are surfaced as a parse error so
/// the broker can respond with `UNSUPPORTED_COMPRESSION_TYPE` (74).
///
/// Returns `Ok((records, attributes))`. The caller reads the compression
/// codec from `attributes & 0x7` if it wants to, but the records vector
/// is already uncompressed either way.
pub fn parse_record_batch(batch_bytes: &[u8]) -> Result<(Vec<DecodedRecord>, i16), String> {
    use crate::record_compression::{decompress, CompressionCodec};

    let mut cur = batch_bytes;
    // Fixed-width batch header (before records)
    let _base_offset = read_i64(&mut cur)?;
    let _batch_length = read_i32(&mut cur)?;
    let _partition_leader_epoch = read_i32(&mut cur)?;
    let magic = read_i8(&mut cur)?;
    if magic != 2 {
        return Err(format!("unsupported RecordBatch magic: {magic}"));
    }
    let _crc = read_i32(&mut cur)?;
    let attributes = read_i16(&mut cur)?;
    let codec = CompressionCodec::from_attributes_bits((attributes & 0x7) as i8)
        .ok_or_else(|| format!("unknown compression codec: {}", attributes & 0x7))?;
    let _last_offset_delta = read_i32(&mut cur)?;
    let base_timestamp = read_i64(&mut cur)?;
    let _max_timestamp = read_i64(&mut cur)?;
    let _producer_id = read_i64(&mut cur)?;
    let _producer_epoch = read_i16(&mut cur)?;
    let _base_sequence = read_i32(&mut cur)?;
    let records_count = read_i32(&mut cur)?;
    if records_count < 0 {
        return Err(format!("negative records count: {records_count}"));
    }

    // For compressed batches, the remaining bytes are the compressed
    // records blob. Decompress into an owned Vec and iterate against
    // that slice; for `None`, fall through with the original borrow.
    let decompressed_blob: Option<Vec<u8>> = if codec == CompressionCodec::None {
        None
    } else {
        Some(decompress(codec, cur)?)
    };
    let mut records_cur: &[u8] = match &decompressed_blob {
        Some(v) => v.as_slice(),
        None => cur,
    };

    let mut records = Vec::with_capacity(records_count as usize);
    for _ in 0..records_count {
        let record_len = read_signed_varint(&mut records_cur)?;
        if record_len < 0 {
            return Err(format!("negative record length: {record_len}"));
        }
        // Bound the record body so a bogus length can't read past the batch.
        if (record_len as usize) > records_cur.len() {
            return Err("record length overruns batch".into());
        }
        let mut body = &records_cur[..record_len as usize];
        records_cur = &records_cur[record_len as usize..];

        let _attributes = read_i8(&mut body)?;
        let timestamp_delta = read_signed_varint(&mut body)?;
        let _offset_delta = read_signed_varint(&mut body)?;
        let key_len = read_signed_varint(&mut body)?;
        let key = if key_len < 0 {
            None
        } else {
            Some(take(&mut body, key_len as usize)?.to_vec())
        };
        let value_len = read_signed_varint(&mut body)?;
        let value = if value_len < 0 {
            Vec::new()
        } else {
            take(&mut body, value_len as usize)?.to_vec()
        };
        let headers_len = read_signed_varint(&mut body)?;
        if headers_len < 0 {
            return Err(format!("negative headers count: {headers_len}"));
        }
        let mut headers = Vec::with_capacity(headers_len as usize);
        for _ in 0..headers_len {
            let hk_len = read_signed_varint(&mut body)?;
            if hk_len < 0 {
                return Err("negative header key length".into());
            }
            let hk_bytes = take(&mut body, hk_len as usize)?;
            let hk = String::from_utf8(hk_bytes.to_vec())
                .map_err(|e| format!("invalid header key utf8: {e}"))?;
            let hv_len = read_signed_varint(&mut body)?;
            let hv = if hv_len < 0 {
                Vec::new()
            } else {
                take(&mut body, hv_len as usize)?.to_vec()
            };
            headers.push((hk, hv));
        }

        records.push(DecodedRecord {
            timestamp_ms: base_timestamp.saturating_add(timestamp_delta),
            key,
            value,
            headers,
        });
    }

    Ok((records, attributes))
}

// =========================================================================
// Produce v9 request parser
// =========================================================================

/// Parse a Produce v9 (flexible) request body. `body` starts immediately
/// after the request header's tag buffer.
pub fn parse_produce_v9(body: &[u8]) -> Result<ProduceRequestV9, String> {
    let mut cur = body;

    let transactional_id = read_compact_nullable_string(&mut cur)?;
    let acks = read_i16(&mut cur)?;
    let timeout_ms = read_i32(&mut cur)?;

    let topics_len_plus_one = read_unsigned_varint(&mut cur)?;
    if topics_len_plus_one == 0 {
        return Err("produce request topic array is null".into());
    }
    let topics_len = (topics_len_plus_one - 1) as usize;
    let mut topics = Vec::with_capacity(topics_len);

    for _ in 0..topics_len {
        let name = read_compact_string(&mut cur)?;

        let parts_len_plus_one = read_unsigned_varint(&mut cur)?;
        if parts_len_plus_one == 0 {
            return Err(format!("topic {name} partition array is null"));
        }
        let parts_len = (parts_len_plus_one - 1) as usize;
        let mut parts = Vec::with_capacity(parts_len);

        for _ in 0..parts_len {
            let partition_index = read_i32(&mut cur)?;
            let records_bytes = read_compact_nullable_bytes(&mut cur)?;
            let (records, attributes) = match records_bytes {
                None => (Vec::new(), 0i16),
                Some(bytes) => parse_record_batch(bytes)?,
            };
            skip_tag_buffer(&mut cur)?;
            parts.push(PartitionProduceData {
                partition_index,
                records,
                compression_codec: (attributes & 0x7) as i8,
            });
        }
        skip_tag_buffer(&mut cur)?;
        topics.push(TopicProduceData {
            name,
            partitions: parts,
        });
    }

    skip_tag_buffer(&mut cur)?;
    Ok(ProduceRequestV9 {
        transactional_id,
        acks,
        timeout_ms,
        topics,
    })
}

// =========================================================================
// Produce v9 response serializer
// =========================================================================

/// Serialize a full Produce v9 response including the flexible response
/// header (correlation_id + empty tag buffer).
pub fn serialize_produce_v9_response(
    correlation_id: i32,
    results: &[TopicProduceResult],
) -> Vec<u8> {
    let mut out = Vec::new();
    // Response header v1 (flexible): correlation_id + empty tag buffer
    out.extend_from_slice(&correlation_id.to_be_bytes());
    push_empty_tag_buffer(&mut out);

    // Body
    push_unsigned_varint(&mut out, (results.len() as u32) + 1);
    for topic in results {
        push_compact_string(&mut out, &topic.name);
        push_unsigned_varint(&mut out, (topic.partitions.len() as u32) + 1);
        for p in &topic.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&p.error_code.to_be_bytes());
            out.extend_from_slice(&p.base_offset.to_be_bytes());
            out.extend_from_slice(&p.log_append_time_ms.to_be_bytes());
            out.extend_from_slice(&p.log_start_offset.to_be_bytes());
            // record_errors: empty compact array
            push_unsigned_varint(&mut out, 1);
            // error_message: null compact string
            push_unsigned_varint(&mut out, 0);
            push_empty_tag_buffer(&mut out);
        }
        push_empty_tag_buffer(&mut out);
    }
    // throttle_time_ms
    out.extend_from_slice(&0i32.to_be_bytes());
    push_empty_tag_buffer(&mut out);
    out
}

/// Build a single-record RecordBatch v2 for tests in other modules.
/// `crc` is written as 0 — our parser doesn't validate it — so this is
/// suitable for driving `parse_produce_v9` end-to-end but not for sending
/// to a real broker.
#[cfg(test)]
pub(crate) fn one_record_batch_for_testing(key: Option<&[u8]>, value: &[u8]) -> Vec<u8> {
    let mut record = Vec::new();
    record.push(0); // attributes
    record.push(0); // timestamp_delta (zigzag 0)
    record.push(0); // offset_delta (zigzag 0)
    match key {
        None => record.push(1), // zigzag(-1) = 1
        Some(k) => {
            let zz = ((k.len() as i64) << 1) as u64;
            let mut v = zz;
            while (v & !0x7F) != 0 {
                record.push(((v & 0x7F) | 0x80) as u8);
                v >>= 7;
            }
            record.push(v as u8);
            record.extend_from_slice(k);
        }
    }
    let zz = ((value.len() as i64) << 1) as u64;
    let mut v = zz;
    while (v & !0x7F) != 0 {
        record.push(((v & 0x7F) | 0x80) as u8);
        v >>= 7;
    }
    record.push(v as u8);
    record.extend_from_slice(value);
    record.push(0); // headers_count

    let mut record_framed = Vec::new();
    let zz = ((record.len() as i64) << 1) as u64;
    let mut v = zz;
    while (v & !0x7F) != 0 {
        record_framed.push(((v & 0x7F) | 0x80) as u8);
        v >>= 7;
    }
    record_framed.push(v as u8);
    record_framed.extend_from_slice(&record);

    let mut batch = Vec::new();
    batch.extend_from_slice(&0i64.to_be_bytes()); // base_offset
    batch.extend_from_slice(&0i32.to_be_bytes()); // batch_length (not validated)
    batch.extend_from_slice(&(-1i32).to_be_bytes()); // partition_leader_epoch
    batch.push(2); // magic
    batch.extend_from_slice(&0i32.to_be_bytes()); // crc (not validated)
    batch.extend_from_slice(&0i16.to_be_bytes()); // attributes
    batch.extend_from_slice(&0i32.to_be_bytes()); // last_offset_delta
    batch.extend_from_slice(&1_000i64.to_be_bytes()); // base_timestamp
    batch.extend_from_slice(&1_000i64.to_be_bytes()); // max_timestamp
    batch.extend_from_slice(&(-1i64).to_be_bytes()); // producer_id
    batch.extend_from_slice(&(-1i16).to_be_bytes()); // producer_epoch
    batch.extend_from_slice(&(-1i32).to_be_bytes()); // base_sequence
    batch.extend_from_slice(&1i32.to_be_bytes()); // records_count
    batch.extend_from_slice(&record_framed);
    batch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_varint_roundtrip() {
        for v in [0u32, 1, 127, 128, 300, 16_383, 16_384, u32::MAX] {
            let mut buf = Vec::new();
            push_unsigned_varint(&mut buf, v);
            let mut slice = buf.as_slice();
            let decoded = read_unsigned_varint(&mut slice).unwrap();
            assert_eq!(decoded, v, "mismatch on {v}");
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn signed_varint_zigzag() {
        // ZigZag: 0 -> 0, -1 -> 1, 1 -> 2, -2 -> 3, 2 -> 4
        fn encode(n: i64) -> Vec<u8> {
            let zz = ((n << 1) ^ (n >> 63)) as u64;
            let mut buf = Vec::new();
            let mut v = zz;
            while (v & !0x7F) != 0 {
                buf.push(((v & 0x7F) | 0x80) as u8);
                v >>= 7;
            }
            buf.push(v as u8);
            buf
        }
        for n in [-1i64, 0, 1, -2, 2, -63, 64, 2147483647, -2147483648] {
            let enc = encode(n);
            let mut slice = enc.as_slice();
            assert_eq!(read_signed_varint(&mut slice).unwrap(), n);
            assert!(slice.is_empty());
        }
    }

    use super::one_record_batch_for_testing as one_record_batch;

    #[test]
    fn parses_single_record_batch() {
        let batch = one_record_batch(Some(b"k"), b"hello");
        let (records, attrs) = parse_record_batch(&batch).unwrap();
        assert_eq!(attrs & 0x7, 0);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key.as_deref(), Some(b"k".as_ref()));
        assert_eq!(records[0].value, b"hello");
        assert_eq!(records[0].timestamp_ms, 1_000);
        assert!(records[0].headers.is_empty());
    }

    #[test]
    fn rejects_unknown_compression_codec() {
        let mut batch = one_record_batch(None, b"x");
        // Attributes are an i16 at offset 21 (after the 17-byte fixed
        // prefix: base_offset(8) + batch_length(4) + leader_epoch(4) +
        // magic(1) + crc(4)). Codec 5 is unassigned.
        batch[21] = 0;
        batch[22] = 5;
        let err = parse_record_batch(&batch).unwrap_err();
        assert!(err.contains("unknown compression codec"), "unexpected error: {err}");
    }

    fn swap_records_for_compressed(
        batch: &[u8],
        codec: crate::record_compression::CompressionCodec,
    ) -> Vec<u8> {
        // Uncompressed batch layout:
        //   [0..21]: base_offset(8), batch_length(4), leader_epoch(4), magic(1), crc(4)
        //   [21..23]: attributes(2)
        //   [23..27]: last_offset_delta(4)
        //   [27..35]: base_timestamp(8)
        //   [35..43]: max_timestamp(8)
        //   [43..51]: producer_id(8)
        //   [51..53]: producer_epoch(2)
        //   [53..57]: base_sequence(4)
        //   [57..61]: records_count(4)
        //   [61..]:  raw records blob
        //
        // Rebuild the batch with the blob compressed + attributes bits set.
        assert!(batch.len() >= 61);
        let records_blob = &batch[61..];
        let compressed = crate::record_compression::compress(codec, records_blob).unwrap();

        let mut out = Vec::with_capacity(61 + compressed.len());
        out.extend_from_slice(&batch[..21]);
        let attributes = codec.attributes_bits();
        out.extend_from_slice(&attributes.to_be_bytes());
        out.extend_from_slice(&batch[23..61]);
        out.extend_from_slice(&compressed);
        out
    }

    #[test]
    fn decompresses_gzip_and_yields_records() {
        let uncompressed = one_record_batch(Some(b"k"), b"gzipped-hello");
        let batch = swap_records_for_compressed(
            &uncompressed,
            crate::record_compression::CompressionCodec::Gzip,
        );
        let (records, attrs) = parse_record_batch(&batch).unwrap();
        assert_eq!(attrs & 0x7, 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value, b"gzipped-hello");
        assert_eq!(records[0].key.as_deref(), Some(b"k".as_ref()));
    }

    #[test]
    fn decompresses_snappy_and_yields_records() {
        let uncompressed = one_record_batch(None, b"snappy-hello");
        let batch = swap_records_for_compressed(
            &uncompressed,
            crate::record_compression::CompressionCodec::Snappy,
        );
        let (records, attrs) = parse_record_batch(&batch).unwrap();
        assert_eq!(attrs & 0x7, 2);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value, b"snappy-hello");
    }

    #[test]
    fn decompresses_lz4_and_yields_records() {
        let uncompressed = one_record_batch(None, b"lz4-hello");
        let batch = swap_records_for_compressed(
            &uncompressed,
            crate::record_compression::CompressionCodec::Lz4,
        );
        let (records, attrs) = parse_record_batch(&batch).unwrap();
        assert_eq!(attrs & 0x7, 3);
        assert_eq!(records[0].value, b"lz4-hello");
    }

    #[test]
    fn decompresses_zstd_and_yields_records() {
        let uncompressed = one_record_batch(None, b"zstd-hello");
        let batch = swap_records_for_compressed(
            &uncompressed,
            crate::record_compression::CompressionCodec::Zstd,
        );
        let (records, attrs) = parse_record_batch(&batch).unwrap();
        assert_eq!(attrs & 0x7, 4);
        assert_eq!(records[0].value, b"zstd-hello");
    }

    #[test]
    fn parses_produce_v9_single_topic() {
        // Craft a minimal Produce v9 body:
        //   transactional_id = null (varint 0)
        //   acks = -1
        //   timeout_ms = 30000
        //   topics = [{ name="t", partitions=[{ index=0, records=batch, tags }], tags }]
        //   tags
        let batch = one_record_batch(None, b"v");

        let mut body = Vec::new();
        body.push(0); // transactional_id null
        body.extend_from_slice(&(-1i16).to_be_bytes()); // acks
        body.extend_from_slice(&30_000i32.to_be_bytes());
        // topics compact array length: 1+1=2
        push_unsigned_varint(&mut body, 2);
        // topic name "t" (len=1, compact = 2)
        push_unsigned_varint(&mut body, 2);
        body.push(b't');
        // partitions compact array: 1+1=2
        push_unsigned_varint(&mut body, 2);
        // partition index
        body.extend_from_slice(&0i32.to_be_bytes());
        // records compact bytes: len+1
        push_unsigned_varint(&mut body, (batch.len() as u32) + 1);
        body.extend_from_slice(&batch);
        // partition tag buffer
        body.push(0);
        // topic tag buffer
        body.push(0);
        // request tag buffer
        body.push(0);

        let req = parse_produce_v9(&body).unwrap();
        assert_eq!(req.acks, -1);
        assert_eq!(req.timeout_ms, 30_000);
        assert_eq!(req.topics.len(), 1);
        assert_eq!(req.topics[0].name, "t");
        assert_eq!(req.topics[0].partitions.len(), 1);
        assert_eq!(req.topics[0].partitions[0].partition_index, 0);
        assert_eq!(req.topics[0].partitions[0].records.len(), 1);
        assert_eq!(req.topics[0].partitions[0].records[0].value, b"v");
    }

    #[test]
    fn response_shape_matches_spec() {
        let results = vec![TopicProduceResult {
            name: "orders".to_string(),
            partitions: vec![PartitionProduceResult {
                partition_index: 2,
                error_code: 0,
                base_offset: 41,
                log_append_time_ms: -1,
                log_start_offset: 0,
            }],
        }];
        let data = serialize_produce_v9_response(99, &results);

        // correlation_id
        assert_eq!(&data[0..4], &99i32.to_be_bytes());
        // response header tag buffer
        assert_eq!(data[4], 0);
        // topics compact array length (1+1=2)
        assert_eq!(data[5], 2);
        // topic name "orders" (len=6, compact=7)
        assert_eq!(data[6], 7);
        assert_eq!(&data[7..13], b"orders");
        // partitions compact array length (1+1=2)
        assert_eq!(data[13], 2);
        // partition_index=2, error_code=0, base_offset=41
        assert_eq!(&data[14..18], &2i32.to_be_bytes());
        assert_eq!(&data[18..20], &0i16.to_be_bytes());
        assert_eq!(&data[20..28], &41i64.to_be_bytes());
        // log_append_time_ms=-1, log_start_offset=0
        assert_eq!(&data[28..36], &(-1i64).to_be_bytes());
        assert_eq!(&data[36..44], &0i64.to_be_bytes());
        // record_errors compact array = 0+1=1
        assert_eq!(data[44], 1);
        // error_message null
        assert_eq!(data[45], 0);
        // partition tag buffer, topic tag buffer
        assert_eq!(data[46], 0);
        assert_eq!(data[47], 0);
        // throttle_time_ms + top-level tag buffer
        assert_eq!(&data[48..52], &0i32.to_be_bytes());
        assert_eq!(data[52], 0);
        assert_eq!(data.len(), 53);
    }
}

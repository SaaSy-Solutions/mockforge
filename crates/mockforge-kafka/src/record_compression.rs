//! Compression codecs for RecordBatch v2.
//!
//! The bottom three bits of a RecordBatch v2 `attributes` field carry the
//! compression codec:
//!
//!   0 = none, 1 = gzip, 2 = snappy, 3 = lz4, 4 = zstd.
//!
//! Anything else is unknown and surfaces as
//! `UNSUPPORTED_COMPRESSION_TYPE` (74) to the client.
//!
//! The compressed/decompressed payload is the concatenation of framed
//! records (what the caller would otherwise iterate over as records). The
//! batch header stays raw either way.
//!
//! ## Snappy framing note
//!
//! Kafka uses *plain snappy* for the `snappy` codec — NOT the snappy
//! framing format. `snap::raw::{Encoder, Decoder}` is what we want;
//! `snap::read::FrameDecoder` would desynchronize. librdkafka is also
//! permissive here: if the first four bytes look like the Xerial
//! "SNAPPY\x00\x00" header it strips it, otherwise it feeds the whole
//! payload to plain snappy. We emit plain-only on the way out and accept
//! plain-only on the way in; Xerial framing has been deprecated for years.
//!
//! ## LZ4 framing note
//!
//! Kafka uses the LZ4 FRAME format (not the raw block format). `lz4_flex`
//! exposes `frame::FrameEncoder`/`FrameDecoder` for this — we use those
//! on both sides.

use std::io::{Read, Write};

/// Compression codec value encoded in the low 3 bits of RecordBatch v2
/// attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionCodec {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

impl CompressionCodec {
    /// Decode the low 3 bits of a batch's attributes field.
    ///
    /// Returns `None` for unknown codecs (5, 6, 7) so the caller can
    /// respond with `UNSUPPORTED_COMPRESSION_TYPE` rather than silently
    /// misinterpreting the payload.
    pub fn from_attributes_bits(bits: i8) -> Option<Self> {
        match bits & 0x7 {
            0 => Some(Self::None),
            1 => Some(Self::Gzip),
            2 => Some(Self::Snappy),
            3 => Some(Self::Lz4),
            4 => Some(Self::Zstd),
            _ => None,
        }
    }

    /// Bit pattern to OR into the attributes field when serializing.
    pub fn attributes_bits(self) -> i16 {
        match self {
            Self::None => 0,
            Self::Gzip => 1,
            Self::Snappy => 2,
            Self::Lz4 => 3,
            Self::Zstd => 4,
        }
    }
}

/// Decompress a records blob according to its codec.
///
/// Returns `Err(String)` on malformed payloads. The caller still owns the
/// decision of what to do with the bytes — this just handles the wire
/// codec unpacking.
pub fn decompress(codec: CompressionCodec, payload: &[u8]) -> Result<Vec<u8>, String> {
    match codec {
        CompressionCodec::None => Ok(payload.to_vec()),
        CompressionCodec::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(payload);
            let mut out = Vec::with_capacity(payload.len() * 2);
            decoder.read_to_end(&mut out).map_err(|e| format!("gzip decode: {e}"))?;
            Ok(out)
        }
        CompressionCodec::Snappy => {
            let mut dec = snap::raw::Decoder::new();
            dec.decompress_vec(payload).map_err(|e| format!("snappy decode: {e}"))
        }
        CompressionCodec::Lz4 => {
            let mut decoder = lz4_flex::frame::FrameDecoder::new(payload);
            let mut out = Vec::with_capacity(payload.len() * 2);
            decoder.read_to_end(&mut out).map_err(|e| format!("lz4 decode: {e}"))?;
            Ok(out)
        }
        CompressionCodec::Zstd => {
            zstd::decode_all(payload).map_err(|e| format!("zstd decode: {e}"))
        }
    }
}

/// Compress a records blob with the given codec. For `None` this just
/// clones the input.
pub fn compress(codec: CompressionCodec, payload: &[u8]) -> Result<Vec<u8>, String> {
    match codec {
        CompressionCodec::None => Ok(payload.to_vec()),
        CompressionCodec::Gzip => {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(payload).map_err(|e| format!("gzip encode: {e}"))?;
            encoder.finish().map_err(|e| format!("gzip finish: {e}"))
        }
        CompressionCodec::Snappy => {
            let mut enc = snap::raw::Encoder::new();
            enc.compress_vec(payload).map_err(|e| format!("snappy encode: {e}"))
        }
        CompressionCodec::Lz4 => {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
            encoder.write_all(payload).map_err(|e| format!("lz4 encode: {e}"))?;
            encoder.finish().map_err(|e| format!("lz4 finish: {e}"))
        }
        CompressionCodec::Zstd => {
            zstd::encode_all(payload, 3).map_err(|e| format!("zstd encode: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"the quick brown fox jumps over the lazy dog; \
         mockforge kafka RecordBatch v2 compression roundtrip test payload";

    #[test]
    fn codec_mapping_matches_spec() {
        assert_eq!(CompressionCodec::from_attributes_bits(0), Some(CompressionCodec::None));
        assert_eq!(CompressionCodec::from_attributes_bits(1), Some(CompressionCodec::Gzip));
        assert_eq!(CompressionCodec::from_attributes_bits(2), Some(CompressionCodec::Snappy));
        assert_eq!(CompressionCodec::from_attributes_bits(3), Some(CompressionCodec::Lz4));
        assert_eq!(CompressionCodec::from_attributes_bits(4), Some(CompressionCodec::Zstd));
        assert_eq!(CompressionCodec::from_attributes_bits(5), None);
        assert_eq!(CompressionCodec::from_attributes_bits(7), None);

        assert_eq!(CompressionCodec::None.attributes_bits(), 0);
        assert_eq!(CompressionCodec::Gzip.attributes_bits(), 1);
        assert_eq!(CompressionCodec::Snappy.attributes_bits(), 2);
        assert_eq!(CompressionCodec::Lz4.attributes_bits(), 3);
        assert_eq!(CompressionCodec::Zstd.attributes_bits(), 4);
    }

    #[test]
    fn roundtrip_gzip() {
        let compressed = compress(CompressionCodec::Gzip, SAMPLE).unwrap();
        assert_ne!(compressed.as_slice(), SAMPLE);
        let decompressed = decompress(CompressionCodec::Gzip, &compressed).unwrap();
        assert_eq!(decompressed.as_slice(), SAMPLE);
    }

    #[test]
    fn roundtrip_snappy() {
        let compressed = compress(CompressionCodec::Snappy, SAMPLE).unwrap();
        let decompressed = decompress(CompressionCodec::Snappy, &compressed).unwrap();
        assert_eq!(decompressed.as_slice(), SAMPLE);
    }

    #[test]
    fn roundtrip_lz4() {
        let compressed = compress(CompressionCodec::Lz4, SAMPLE).unwrap();
        let decompressed = decompress(CompressionCodec::Lz4, &compressed).unwrap();
        assert_eq!(decompressed.as_slice(), SAMPLE);
    }

    #[test]
    fn roundtrip_zstd() {
        let compressed = compress(CompressionCodec::Zstd, SAMPLE).unwrap();
        let decompressed = decompress(CompressionCodec::Zstd, &compressed).unwrap();
        assert_eq!(decompressed.as_slice(), SAMPLE);
    }

    #[test]
    fn none_codec_is_passthrough() {
        assert_eq!(compress(CompressionCodec::None, SAMPLE).unwrap(), SAMPLE);
        assert_eq!(decompress(CompressionCodec::None, SAMPLE).unwrap(), SAMPLE);
    }

    #[test]
    fn decompress_rejects_garbage() {
        assert!(decompress(CompressionCodec::Gzip, b"not-gzip").is_err());
        assert!(decompress(CompressionCodec::Zstd, b"not-zstd").is_err());
    }
}

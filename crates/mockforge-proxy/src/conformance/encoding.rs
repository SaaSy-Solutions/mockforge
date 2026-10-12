//! Bounded decoding for validation copies; forwarded bytes are never changed.

const MAX_DECODED_JSON_BYTES: usize = 16 * 1024 * 1024;

pub(super) async fn decode_body(body: &[u8], encoding: Option<&str>) -> Result<Vec<u8>, String> {
    use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder};
    use tokio::io::{AsyncRead, AsyncReadExt};

    let encodings: Vec<_> = encoding
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("identity"))
        .collect();
    if encodings.len() > 4 {
        return Err("body has more than four content-encoding layers".to_string());
    }
    if body.len() > MAX_DECODED_JSON_BYTES {
        return Err("body exceeds the 16 MiB JSON validation limit".to_string());
    }
    let mut bytes = body.to_vec();
    for encoding in encodings.into_iter().rev() {
        let reader: Box<dyn AsyncRead + Unpin + Send + '_> =
            match encoding.to_ascii_lowercase().as_str() {
                "gzip" | "x-gzip" => Box::new(GzipDecoder::new(bytes.as_slice())),
                "deflate" => Box::new(ZlibDecoder::new(bytes.as_slice())),
                "br" => Box::new(BrotliDecoder::new(bytes.as_slice())),
                "zstd" => Box::new(ZstdDecoder::new(bytes.as_slice())),
                _ => return Err(format!("unsupported body content-encoding: {encoding}")),
            };
        let mut decoded = Vec::new();
        reader
            .take((MAX_DECODED_JSON_BYTES + 1) as u64)
            .read_to_end(&mut decoded)
            .await
            .map_err(|_| "body content-encoding cannot be decoded".to_string())?;
        if decoded.len() > MAX_DECODED_JSON_BYTES {
            return Err("decoded body exceeds the 16 MiB JSON validation limit".to_string());
        }
        bytes = decoded;
    }
    if bytes.len() > MAX_DECODED_JSON_BYTES {
        return Err("body exceeds the 16 MiB JSON validation limit".to_string());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compressed_validation_decodes_all_supported_codecs_and_bounds_expansion() {
        use async_compression::tokio::bufread::{
            BrotliEncoder, GzipEncoder, ZlibEncoder, ZstdEncoder,
        };
        use tokio::io::{AsyncRead, AsyncReadExt};
        let input = br#"{"created":true}"#;
        let cases: Vec<(&str, Box<dyn AsyncRead + Unpin + Send>)> = vec![
            ("gzip", Box::new(GzipEncoder::new(input.as_slice()))),
            ("deflate", Box::new(ZlibEncoder::new(input.as_slice()))),
            ("br", Box::new(BrotliEncoder::new(input.as_slice()))),
            ("zstd", Box::new(ZstdEncoder::new(input.as_slice()))),
        ];
        for (encoding, mut encoder) in cases {
            let mut wire = Vec::new();
            encoder.read_to_end(&mut wire).await.unwrap();
            assert_eq!(decode_body(&wire, Some(encoding)).await.unwrap(), input);
        }
        assert!(decode_body(b"not gzip", Some("gzip")).await.is_err());
        assert!(decode_body(input, Some("unsupported")).await.is_err());
        assert!(decode_body(input, Some("gzip,gzip,gzip,gzip,gzip")).await.is_err());
        let expanded = vec![b' '; MAX_DECODED_JSON_BYTES + 1];
        let mut encoder = GzipEncoder::new(expanded.as_slice());
        let mut wire = Vec::new();
        encoder.read_to_end(&mut wire).await.unwrap();
        assert!(decode_body(&wire, Some("gzip")).await.unwrap_err().contains("16 MiB"));
    }
}

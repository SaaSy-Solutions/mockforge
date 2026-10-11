//! Chunk lengths, payload terminators, and bounded trailers.

use super::{drain_bytes, invalid_response, read_line, read_more};
use std::sync::atomic::AtomicU64;
use tokio::io::AsyncRead;

pub(super) async fn read_chunked(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    bytes_received: &AtomicU64,
) -> std::io::Result<()> {
    loop {
        let line = read_line(stream, buf, bytes_received).await?;
        let size = std::str::from_utf8(&line)
            .ok()
            .and_then(|line| line.split(';').next())
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .and_then(|value| u64::from_str_radix(value, 16).ok())
            .ok_or_else(|| invalid_response("invalid chunk size"))?;
        if size == 0 {
            let mut trailer_bytes = 0;
            loop {
                let trailer = read_line(stream, buf, bytes_received).await?;
                trailer_bytes += trailer.len() + 2;
                if trailer_bytes > 128 * 1024 {
                    return Err(invalid_response("response trailers exceed 128 KiB"));
                }
                if trailer.is_empty() {
                    break;
                }
                if !trailer.contains(&b':') {
                    return Err(invalid_response("malformed response trailer"));
                }
            }
            break;
        }
        drain_bytes(stream, buf, bytes_received, size).await?;
        // Validate the payload terminator without accepting arbitrary bytes.
        while buf.len() < 2 {
            read_more(stream, buf, bytes_received).await?;
        }
        if &buf[..2] != b"\r\n" {
            return Err(invalid_response("invalid chunk terminator"));
        }
        buf.drain(..2);
    }
    Ok(())
}

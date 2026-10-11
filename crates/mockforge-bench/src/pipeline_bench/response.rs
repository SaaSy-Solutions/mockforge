//! HTTP response framing for ordered pipelined replies.

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Read a final HTTP response, retaining bytes belonging to the next response.
pub(super) async fn read_response(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    bytes_received: &AtomicU64,
    head_request: bool,
) -> std::io::Result<(u16, u64)> {
    let before = bytes_received.load(Ordering::Relaxed);
    let (status, head) = loop {
        let end = loop {
            if let Some(end) = find_head_end(buf) {
                if end > 128 * 1024 {
                    return Err(invalid_response("response head exceeds 128 KiB"));
                }
                break end;
            }
            if buf.len() > 128 * 1024 {
                return Err(invalid_response("response head exceeds 128 KiB"));
            }
            read_more(stream, buf, bytes_received).await?;
        };
        let head = std::str::from_utf8(&buf[..end])
            .map_err(|_| invalid_response("invalid response head"))?
            .to_owned();
        buf.drain(..end + 4);
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| (100..=599).contains(value))
            .ok_or_else(|| invalid_response("malformed status line"))?;
        if status == 101 {
            return Err(invalid_response("protocol upgrades are not supported"));
        }
        if status >= 200 {
            break (status, head);
        }
        // Informational responses do not complete the pipelined request.
    };
    if head_request || status == 204 || status == 304 {
        return Ok((status, bytes_received.load(Ordering::Relaxed) - before));
    }
    let mut length = None;
    let mut transfer = Vec::new();
    for line in head.lines().skip(1) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid_response("malformed response header"))?;
        if name.eq_ignore_ascii_case("content-length") {
            for value in value.split(',') {
                let parsed = value
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| invalid_response("invalid content length"))?;
                if length.is_some_and(|previous| previous != parsed) {
                    return Err(invalid_response("conflicting content lengths"));
                }
                length = Some(parsed);
            }
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            transfer.extend(value.split(',').map(|value| value.trim().to_ascii_lowercase()));
        }
    }
    if transfer.last().is_some_and(|coding| coding == "chunked") {
        loop {
            let line = read_line(stream, buf, bytes_received).await?;
            let size = std::str::from_utf8(&line)
                .ok()
                .and_then(|line| line.split(';').next())
                .filter(|value| {
                    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
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
    } else if transfer.is_empty() && length.is_some() {
        drain_bytes(stream, buf, bytes_received, length.unwrap_or_default()).await?;
    } else {
        // A close-delimited response completes only at EOF. Its bytes cannot
        // be interpreted as another pipelined status line.
        buf.clear();
        let mut chunk = [0u8; 16384];
        loop {
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            bytes_received.fetch_add(read as u64, Ordering::Relaxed);
        }
    }
    Ok((status, bytes_received.load(Ordering::Relaxed) - before))
}

fn invalid_response(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

async fn read_more(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    counter: &AtomicU64,
) -> std::io::Result<()> {
    let mut chunk = [0u8; 8192];
    let read = stream.read(&mut chunk).await?;
    if read == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "incomplete response"));
    }
    counter.fetch_add(read as u64, Ordering::Relaxed);
    buf.extend_from_slice(&chunk[..read]);
    Ok(())
}

async fn read_line(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    counter: &AtomicU64,
) -> std::io::Result<Vec<u8>> {
    loop {
        if let Some(end) = buf.windows(2).position(|window| window == b"\r\n") {
            if end > 128 * 1024 {
                return Err(invalid_response("response line exceeds 128 KiB"));
            }
            let line = buf.drain(..end).collect();
            buf.drain(..2);
            return Ok(line);
        }
        if buf.len() > 128 * 1024 {
            return Err(invalid_response("response line exceeds 128 KiB"));
        }
        read_more(stream, buf, counter).await?;
    }
}

async fn drain_bytes(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    counter: &AtomicU64,
    mut remaining: u64,
) -> std::io::Result<()> {
    while remaining > 0 {
        if buf.is_empty() {
            read_more(stream, buf, counter).await?;
        }
        let take = remaining.min(buf.len() as u64) as usize;
        buf.drain(..take);
        remaining -= take as u64;
    }
    Ok(())
}

/// Find the `\r\n\r\n` terminator in `buf`, returning the offset of its
/// first byte.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

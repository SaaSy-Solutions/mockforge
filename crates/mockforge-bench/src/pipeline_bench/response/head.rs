//! Bounded HTTP response heads and informational-response handling.

use super::{invalid_response, read_more};
use std::sync::atomic::AtomicU64;
use tokio::io::AsyncRead;

pub(super) async fn read_final_head(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    bytes_received: &AtomicU64,
) -> std::io::Result<(u16, String)> {
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
        // Field values can contain opaque non-UTF-8 octets (RFC 9110 §5.5).
        // Only the ASCII status and framing headers are interpreted here.
        let head = String::from_utf8_lossy(&buf[..end]).into_owned();
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
    Ok((status, head))
}

/// Find the `\r\n\r\n` terminator in `buf`, returning the offset of its
/// first byte.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

pub(super) fn parse_framing_headers(head: &str) -> std::io::Result<(Option<u64>, Vec<String>)> {
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
    Ok((length, transfer))
}

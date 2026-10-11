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
    Ok((status, head))
}

/// Find the `\r\n\r\n` terminator in `buf`, returning the offset of its
/// first byte.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

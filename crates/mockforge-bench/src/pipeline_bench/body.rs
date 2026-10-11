//! Bounded-memory synthetic request body generation.

use super::BodyKind;

/// Fixed head + patterned middle + fixed tail, totalling exactly `size`
/// bytes. The middle streams in 64 KiB patterned chunks so a 900 GB body
/// costs O(64 KiB) memory.
pub(super) struct StreamBody {
    kind: BodyKind,
    prefix: Vec<u8>,
    suffix: Vec<u8>,
    fill_total: u64,
    written_fill: u64,
    phase: u8,
    done_prefix: bool,
    done_fill: bool,
    done_suffix: bool,
}

impl StreamBody {
    const CHUNK: usize = 64 * 1024;

    /// Build a well-formed body of exactly `size` bytes for `kind`.
    /// Returns an error when `size` is too small for the frame.
    #[cfg(test)]
    pub(super) fn new(kind: BodyKind, size: u64) -> Result<Self, String> {
        Self::with_multipart_content_type(kind, size, "application/octet-stream")
    }

    pub(super) fn with_multipart_content_type(
        kind: BodyKind,
        size: u64,
        file_type: &str,
    ) -> Result<Self, String> {
        use BodyKind::*;
        let (prefix, suffix): (Vec<u8>, Vec<u8>) = match kind {
            Json => (b"{\"data\":\"".to_vec(), b"\"}".to_vec()),
            Xml => (b"<root><data>".to_vec(), b"</data></root>".to_vec()),
            UrlEncoded => (b"data=".to_vec(), Vec::new()),
            // One synthetic file part, framed by the multipart boundary
            // declared in BodyKind::content_type().
            Multipart => (
                format!("--mockforge-pipeline\r\nContent-Disposition: form-data; name=\"file\"; filename=\"synthetic.bin\"\r\nContent-Type: {file_type}\r\n\r\n").into_bytes(),
                b"\r\n--mockforge-pipeline--\r\n".to_vec(),
            ),
        };
        let overhead = (prefix.len() + suffix.len()) as u64;
        if size < overhead {
            return Err(format!(
                "body size {size} is smaller than the {kind:?} framing overhead ({overhead} bytes)"
            ));
        }
        Ok(Self {
            kind,
            prefix,
            suffix,
            fill_total: size - overhead,
            written_fill: 0,
            phase: b'a',
            done_prefix: false,
            done_fill: false,
            done_suffix: false,
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn exhausted(&self) -> bool {
        self.done_prefix && self.done_fill && self.done_suffix
    }

    /// Next chunk (up to 64 KiB) of the exact-sized body.
    pub(super) fn next_chunk(&mut self) -> Option<Vec<u8>> {
        if !self.done_prefix {
            self.done_prefix = true;
            return Some(std::mem::take(&mut self.prefix));
        }
        if !self.done_fill && self.fill_total > 0 {
            let remaining = self.fill_total - self.written_fill;
            let take = remaining.min(Self::CHUNK as u64) as usize;
            let mut buf = vec![0u8; take];
            for b in buf.iter_mut() {
                // Patterned filler keeps compression proxies honest and
                // makes truncation visible in captured bodies.
                *b = self.phase;
                self.phase = if self.phase >= b'z' {
                    b'a'
                } else {
                    self.phase + 1
                };
            }
            self.written_fill += take as u64;
            if self.written_fill == self.fill_total {
                self.done_fill = true;
            }
            return Some(buf);
        }
        if !self.done_fill && self.fill_total == 0 {
            self.done_fill = true;
        }
        if !self.done_suffix {
            self.done_suffix = true;
            return Some(std::mem::take(&mut self.suffix));
        }
        None
    }
}

impl StreamBody {
    pub(super) fn content_type(&self) -> &'static str {
        self.kind.content_type()
    }

    pub(super) fn total_len(&self) -> u64 {
        self.prefix.len() as u64 + self.fill_total + self.suffix.len() as u64
    }
}

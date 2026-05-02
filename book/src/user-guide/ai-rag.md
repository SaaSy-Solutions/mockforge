# AI / RAG-Driven Mock Data

MockForge ships two distinct AI-related capabilities; this chapter covers
both because they're easy to confuse:

| Feature | What it does | Where it runs |
|---|---|---|
| **MockAI** (response generation) | Produces realistic responses from natural-language prompts at request time | In the request path |
| **RAG-driven data synthesis** (`mockforge data ...`) | Generates static fixture files at build time using LLM + retrieval | One-shot CLI |

Use **MockAI** when you want responses to vary intelligently per request.
Use **RAG synthesis** when you want a deterministic fixture file you can
check into git.

For MockAI request-time response generation, see the existing
[MockAI chapter](./mockai.md). This chapter focuses on the **`mockforge
data` CLI** — the build-time fixture generator.

## What `mockforge data` is for

You have an OpenAPI spec or a JSON schema. You want sample data to test
against. Three options, in increasing intelligence:

1. **Static fixtures** you write by hand. Tedious; doesn't scale; gets
   stale.
2. **`faker`-based generation** — `{{faker.name}}`, `{{faker.email}}` etc.
   Realistic individual fields, but the rows don't relate to each other
   (a `User` named "Alice Smith" with email `bob@example.com`).
3. **RAG-driven generation** — an LLM looks at field names + schema +
   relationships and produces semantically coherent rows. A `User` named
   "Alice Smith" gets `alice.smith@example.com` and a plausible `bio`.

Option 3 is what `mockforge data` provides.

## Quick start

Generate 100 realistic users using OpenAI:

```bash
export MOCKFORGE_RAG_API_KEY=sk-...
mockforge data template user --rows 100 --rag --rag-provider openai \
  --output users.json
```

Generate from your own schema:

```bash
mockforge data schema my-schema.json --rows 50 \
  --format jsonl --output data.jsonl
```

Generate from an OpenAPI spec (per-schema):

```bash
mockforge data mock-openapi api.yaml --rows 25 --realistic \
  --output mock-data.json
```

Spin up a mock server backed by AI-generated data:

```bash
mockforge data mock-server api.yaml --port 3000 --rows 100
```

## Subcommand reference

### `data template <NAME>`

Built-in templates: `user`, `product`, `order`. Each has an internal
schema with realistic field relationships baked in.

```bash
mockforge data template user --rows 10 --format json
mockforge data template product --rows 50 --output products.csv --format csv
mockforge data template order --rows 20 --rag --rag-provider openai \
  --output orders.json
```

Common flags:

- `-r, --rows <N>` — row count (default `10`)
- `-f, --format <FMT>` — `json` | `csv` | `jsonl`
- `-o, --output <PATH>` — output file (stdout if omitted)
- `--rag` — enable RAG enhancement (otherwise pure faker)
- `--rag-provider <P>` — `openai` | `anthropic` | `ollama` | `openai_compatible`
- `--rag-model <M>` — model name (provider default if omitted)
- `--rag-endpoint <URL>` — custom endpoint (e.g. self-hosted Ollama)
- `--rag-timeout <SECS>`, `--rag-max-retries <N>` — request tuning

### `data schema <SCHEMA>`

Generate from a JSON Schema document.

```bash
mockforge data schema schemas/order.json --rows 100 \
  --format jsonl --output orders.jsonl
```

Same `--rag*` flags apply.

### `data mock-openapi <SPEC>`

Walk every schema in an OpenAPI spec and generate sample data per schema.

```bash
mockforge data mock-openapi api-spec.yaml --rows 50 --realistic \
  --output mock-data.json

mockforge data mock-openapi api-spec.json --validate --include-optional \
  --output mock-data.json
```

Flags:
- `--realistic` — use RAG (defaults to faker-only)
- `--validate` — verify generated rows against the schema before writing
- `--include-optional` — generate optional fields too (otherwise required-only)

### `data mock-server <SPEC>`

End-to-end shortcut: spin up a HTTP server backed by AI-generated data
without writing config files.

```bash
mockforge data mock-server api.yaml --port 3000 --rows 100
```

This is what you reach for when you want to demo a third-party API to your
team in 30 seconds. For long-term mocks, write a config file and use
`mockforge serve`.

### `data rag-config`

Validate and print the resolved RAG config (provider, model, endpoint,
key presence) MockForge would use for the current environment. Useful
when an LLM call is failing and you want to confirm what was actually
loaded.

```bash
mockforge data rag-config
```

## Provider setup

### OpenAI

```bash
export MOCKFORGE_RAG_PROVIDER=openai
export MOCKFORGE_RAG_API_KEY=sk-...
export MOCKFORGE_RAG_MODEL=gpt-4o-mini   # cheaper than gpt-4
mockforge data template user --rows 100 --rag
```

Cost tip: `gpt-4o-mini` and `gpt-3.5-turbo` are sufficient for fixture
generation. The model only sees a schema + field names, not your
production data; you don't need a frontier model.

### Anthropic

```bash
export MOCKFORGE_RAG_PROVIDER=anthropic
export MOCKFORGE_RAG_API_KEY=sk-ant-...
export MOCKFORGE_RAG_MODEL=claude-3-5-haiku-20241022
mockforge data template user --rows 100 --rag
```

### Ollama (local, free)

For development and CI where you don't want to spend on API calls:

```bash
# In one terminal
ollama serve
ollama pull llama3

# In another
export MOCKFORGE_RAG_PROVIDER=ollama
export MOCKFORGE_RAG_MODEL=llama3
export MOCKFORGE_RAG_API_ENDPOINT=http://localhost:11434/api/generate
mockforge data template user --rows 100 --rag
```

This is the recommended path for **CI generation** — no network calls
out, no spend per build.

### OpenAI-compatible (LM Studio, vLLM, Ollama OpenAI mode, etc.)

```bash
export MOCKFORGE_RAG_PROVIDER=openai_compatible
export MOCKFORGE_RAG_API_ENDPOINT=http://localhost:8000/v1/chat/completions
export MOCKFORGE_RAG_MODEL=local-model-name
mockforge data template user --rows 100 --rag
```

## Embedding (semantic search)

For schemas with cross-references (e.g. `Order.user_id` → `User.id`), RAG
optionally uses embeddings to keep references coherent. Configure with:

```bash
export MOCKFORGE_EMBEDDING_PROVIDER=openai      # or local | ollama
export MOCKFORGE_EMBEDDING_MODEL=text-embedding-3-small
export MOCKFORGE_EMBEDDING_ENDPOINT=http://localhost:11434  # for local
export MOCKFORGE_SIMILARITY_THRESHOLD=0.75      # 0.0-1.0
export MOCKFORGE_SEMANTIC_SEARCH=true
```

Without embeddings, foreign-key fields get random valid IDs. With
embeddings, MockForge picks IDs whose adjacent fields are semantically
related to the row being generated.

## Cost / latency tradeoffs

| Setup | Cost per 1K rows | Latency |
|---|---|---|
| Faker only (no `--rag`) | $0 | < 1 s |
| Ollama local | $0 | 30-120 s (CPU) / 5-15 s (GPU) |
| OpenAI gpt-4o-mini | ~$0.01 | 10-30 s |
| OpenAI gpt-4 | ~$0.30 | 30-90 s |
| Anthropic claude-3-5-haiku | ~$0.05 | 15-30 s |

Caching: results are cached by `(schema, prompt, model)` tuple, so
repeated runs of the same `mockforge data template ...` are free after
the first.

## CI patterns

**Pattern 1: Generate fixtures, check them in.**

```yaml
# .github/workflows/refresh-fixtures.yml
on:
  schedule:
    - cron: '0 0 * * 0'   # weekly
jobs:
  refresh:
    steps:
      - uses: actions/checkout@v5
      - run: |
          mockforge data mock-openapi api.yaml \
            --rows 50 --realistic --validate \
            --output fixtures/mock-data.json
      - uses: peter-evans/create-pull-request@v6
        with:
          title: "chore: refresh AI-generated fixtures"
```

**Pattern 2: Generate at test-suite startup.**

```bash
# In CI before tests
mockforge data mock-server api.yaml --port 3000 --rows 100 &
export MOCK_URL=http://localhost:3000
npm test
```

Use Ollama for both patterns to keep CI free.

## When NOT to use RAG generation

- **High-volume hot paths** — generating 1M rows live per request is
  expensive. Pre-generate to a file and serve from disk.
- **Sensitive schemas** — the LLM sees field names. If your schema is
  itself a secret (rare), use `mockforge data schema` with `--rag` off,
  or use Ollama locally so nothing leaves your network.
- **Tight determinism** — LLMs are stochastic by default. For
  byte-reproducible outputs, set `--rag-temperature 0` and seed faker;
  for full reproducibility, generate once and check the file in.

## Where to go next

- [MockAI (request-time response generation)](./mockai.md) — the in-band
  cousin of `mockforge data`
- [Generative Schema Mode](./generative-schema.md) — auto-generate
  schemas from sample requests
- [Configuration env vars: AI/RAG](../configuration/environment.md#rag--llm-provider)
  — full env-var reference
- [Plugin System](./plugins.md) — extend the data generator with custom
  field synthesizers

# Bake AI responses for offline CI

**Availability:** this command is unreleased. It is being developed for #852,
alongside the AI generation rollout in #842.

`mockforge ai bake` calls a provider at authoring time and writes the returned
JSON value into a static MockForge serve configuration. Commit that file and
serve it in CI without a model, provider connection or API key.

## Generate once

Save the desired response description in `users-prompt.txt`, then run:

```bash
mockforge ai bake \
  --provider ollama --model llama3.2 --seed 42 \
  --prompt-file users-prompt.txt \
  --method GET --path /users --output fixtures/users.yaml
```

For an OpenAI-compatible local server, use `--provider openai-compatible` and
`--endpoint http://127.0.0.1:1234/v1/chat/completions`. The endpoint must be the
complete provider API URL. `--provider openai` and `--provider anthropic` use
their standard endpoints unless overridden.

OpenAI and Anthropic read their usual API-key environment variables. Compatible
servers optionally use `OPENAI_API_KEY`. Use `--api-key-env VARIABLE_NAME` to
select another variable; pass its name, never its value. Provider credentials,
endpoint URLs and prompt text are not written into the baked configuration.

The provider must return one valid JSON value. Objects, arrays, strings, numbers,
booleans and `null` are supported. Invalid JSON and provider errors fail the
command without publishing a fixture. `--timeout` bounds the authoring request
(default 60 seconds). Prompt and fixture files are limited to 8 MiB.

The output extension selects JSON (`.json`) or YAML (`.yaml` / `.yml`). The
default response status is 200; `--status` changes it. This JSON-body workflow
requires a body-bearing method/status: HEAD and 204/205/304 are rejected.
Paths support literal segments, `{id}` parameters and final `{*rest}` wildcards.

## Reuse and deliberate changes

Running the same command against an existing verified output reuses its bytes
without initializing the provider or reading its credentials. The request
identity includes the prompt, provider/model/endpoint, effective seed, sampling
settings, method, path and status. Credential rotation and output-file location
do not change it.

A different request or a changed configuration fails the integrity check instead
of overwriting the existing fixture. To generate a new version, choose a new
output path, review the resulting configuration, and commit the intended change.
Concurrent writers cannot replace an already published fixture.

## Serve committed data in CI

```bash
mockforge serve --config fixtures/users.yaml --http-port 8080
```

The generated configuration binds HTTP to loopback and disables live MockAI,
response template expansion, automatic CORS preflight handling, and the other
protocol listeners. OPTIONS therefore returns its baked JSON body too. A literal
`{{uuid}}` or `{{now}}` in generated data stays literal. Repeated calls to
`GET /users`, including after restarting the server, return the same JSON body
bytes while the committed file stays unchanged. CI does not run `ai bake`.
Enabling template expansion, dynamic AI or chaos overrides changes that setup.

## What a seed guarantees

`--seed` (or `MOCKFORGE_AI_SEED`) is forwarded to OpenAI-compatible sampling and
Ollama's `options.seed`. A live provider's sampling seed is best effort, not a
promise of byte-identical future generations. Anthropic has no equivalent
sampling seed, so supplying one is rejected; omit it and freeze the first result.
The CI guarantee comes from serving the committed static data, not from asking
the provider to regenerate it. See [seeded generation](system-generation.md#seeded-generation-852)
for the library configuration and provider limitations.

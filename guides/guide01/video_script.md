# Video Script — What is Mockforge? (7–9 min)

## Hook (0:00–0:25)
- Problem: waiting on real backends slows UI/tests.
- Promise: in 5 min, a working mock backend + dynamic response.

## Setup (0:25–1:10)
- Show prerequisites quickly (CLI, Node).
- Create folder; `mockforge init --no-examples`.

## Build (1:10–3:00)
- Add route `/api/hello` via config file (edit `mockforge.yaml`).
- Start server with `mockforge serve --http-port 4000`; `curl` health check.
- Show response in browser and terminal.

## Make It Realistic (3:00–4:00)
- Add dynamic template reading `?name=` query param using `{{request.query.name}}`.
- Enable template expansion in config.
- Show request/response changing.

## Edge & Latency (4:00–5:00)
- Add latency config in `mockforge.yaml` (300ms fixed delay); show UI spinner or `time curl`.

## When to Use vs Real Backends (5:00–6:30)
- 3 bullets “Use Mockforge when…”
- 2 bullets “Use real backends when…”
- Mention hybrid record/replay.

## Tests Integration (6:30–7:40)
- Brief Playwright example with `beforeAll/afterAll`.
- Emphasize deterministic tests in CI.

## CTA (7:40–8:10)
- Link to Routes 101 and Dynamic Responses.
- Ask to star the repo / try the template.

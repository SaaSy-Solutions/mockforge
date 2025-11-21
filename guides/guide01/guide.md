# What is Mockforge? When to use it vs real backends (+ 5‑min demo)

## Who it’s for
- FE devs validating UIs early
- QA/SDET building reliable automated tests
- BE/Platform enabling contract-first workflows
- PM/EM needing demoable environments

## Outcome in 5–10 mins (clear promise)
- You’ll spin up a realistic mock backend, connect a frontend/test to it, and understand when to reach for Mockforge vs a real service. Quick check: your app/test receives a dynamic response from a Mockforge route you create.

## Prereqs
- Node 18+ (for sample FE), Git
- A terminal with curl
- Optional: Docker if your sample app uses it

## Steps (≈7)
0) Install Mockforge  
   - Why: get the CLI tool ready before creating your first mock.
   - Choose one method:
   
   **Method 1: Cargo Install (Recommended)**
   ```bash
   cargo install mockforge-cli
   ```
   
   **Method 2: Docker**
   ```bash
   git clone https://github.com/SaaSy-Solutions/mockforge.git
   cd mockforge
   docker build -t mockforge .
   ```
   
   **Method 3: Build from Source**
   ```bash
   git clone https://github.com/SaaSy-Solutions/mockforge.git
   cd mockforge
   cargo build --release
   # Binary will be at target/release/mockforge
   # Or install globally: cargo install --path crates/mockforge-cli
   ```
   
   - Verify installation:
     ```bash
     mockforge --version
     ```
   - Result: Mockforge CLI is ready to use.

1) Install & init a project  
   - Why: set up a self-contained mock workspace.
   - Commands:
     ```bash
     mkdir mf-hello && cd mf-hello
     mockforge init --no-examples
     ```
   - Result: `mockforge.yaml` config file created.

2) Create a simple route (config file)  
   - Edit `mockforge.yaml` to add a route:
     ```yaml
     routes:
       - path: "/api/hello"
         method: "GET"
         response:
           status: 200
           body:
             message: "Hello from Mockforge"
     ```
   - Alternative: Use the Admin UI (start server with `--admin` flag) → "Add Route" → fill method/path/body.

3) Start the mock server  
   ```bash
   mockforge serve --http-port 4000
   # health check
   curl -s http://localhost:4000/api/hello | jq .
   ```
   - Expect: `{"message":"Hello from Mockforge"}`
   - Note: Press Ctrl+C to stop the server.

4) Make it dynamic (request-aware)  
   - Update `mockforge.yaml`:
     ```yaml
     routes:
       - path: "/api/hello"
         method: "GET"
         response:
           status: 200
           body:
             message: "Hello {{request.query.name}}"
     ```
   - Enable template expansion (add to config or use env var):
     ```yaml
     http:
       response_template_expand: true
     ```
   - Restart server and test:
     ```bash
     curl -s "http://localhost:4000/api/hello?name=Ray" | jq .
     ```
   - Expect: `{"message":"Hello Ray"}`

5) Add latency to simulate real-world issues  
   - Update `mockforge.yaml`:
     ```yaml
     routes:
       - path: "/api/hello"
         method: "GET"
         response:
           status: 200
           body:
             message: "Hello {{request.query.name}}"
         latency:
           enabled: true
           probability: 1.0
           fixed_delay_ms: 300
     ```
   - Restart server and test:
     ```bash
     time curl -s http://localhost:4000/api/hello > /dev/null
     ```
   - Why: surface client timeouts/spinners in the UI.

6) Wire into tests (Playwright/Cypress example)  
   - Example (Playwright):
     ```ts
     // tests/example.spec.ts
     import { exec } from 'child_process';
     import { promisify } from 'util';
     const execAsync = promisify(exec);
     
     test.beforeAll(async () => {
       // Start server in background
       await execAsync('mockforge serve --http-port 4000 &');
       // Wait for server to be ready
       await new Promise(resolve => setTimeout(resolve, 2000));
     })
     test.afterAll(async () => {
       // Stop server (find and kill process)
       await execAsync('pkill -f "mockforge serve"');
     })
     test('hello route', async ({ request }) => {
       const res = await request.get('http://localhost:4000/api/hello?name=Ray')
       expect(await res.json()).toEqual({ message: 'Hello Ray' })
     })
     ```

## When to use Mockforge vs real backends
- **Use Mockforge when:**
  - You need instant, deterministic responses for UI/QA work.
  - Backend is not ready or flaky; you want to design-first using contracts.
  - You need to simulate errors/latency/edge cases that prod won’t give you easily.
- **Use real backends when:**
  - You’re validating true integration, auth to 3rd parties, or performance at scale.
  - You need production data shape/behavior that’s too complex to model quickly.
- **Hybrid:** Proxy/record real traffic once, then run offline with sanitized replays.

## Gotchas & Debugging
- **404 on /api/hello** → Route path mismatch (`/api/hello` vs `/api/hello/`). Fix path in config.  
- **Port conflict** → Pass `--http-port 4001` or free port.  
- **Dynamic template errors** → Ensure `response_template_expand: true` in config. Check template syntax uses `{{request.query.param}}` format.  
- **Tests flapping** → Wait for server to be ready before running tests (add sleep or health check).
- **Template not expanding** → Set `http.response_template_expand: true` in `mockforge.yaml` or `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true` env var.

## Automate It (CI minimal)
```yaml
name: guide01-hello
on: [pull_request]
jobs:
  demo:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Install Mockforge CLI
        run: cargo install mockforge-cli
      - name: Start server
        run: |
          mockforge serve --http-port 4000 &
          sleep 3  # Wait for server to start
      - name: Health check
        run: curl -f http://localhost:4000/api/hello
      - name: Tests
        run: npm test --if-present
      - name: Teardown
        if: always()
        run: pkill -f "mockforge serve" || true
```

## Next Up (cross-links)
- **Routes 101**: static responses, headers, cookies  
- **Dynamic Responses**: templating & generators

## Assets
- Sample repo skeleton: `/mf-hello` (see `scaffold/` below)
- Copy-paste snippets: see `snippets/` folder

## Shorts Pack
- Post 1: “Spin up a ‘realistic’ backend in 5 minutes with Mockforge. Deterministic, scriptable, and CI-friendly.”  
- Post 2: “Stop waiting on flaky APIs. Mockforge lets you design-first, then test UI and flows reliably.”  
- Post 3: “Chaos on demand: latency & faults to harden your client—without touching prod.”  
- Clip idea (20–30s): terminal split-screen: `mockforge init → edit config → serve → curl` with result, then UI view of the route.

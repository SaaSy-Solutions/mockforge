# CI & Snippets

## GitHub Actions (minimal) — see guide.md for full job
```yaml
- name: Start mocks
  run: |
    mockforge serve --http-port 4000 &
    sleep 3  # Wait for server to start
- name: Run tests
  run: npm test --if-present
- name: Teardown
  if: always()
  run: pkill -f "mockforge serve" || true
```

## API Usage (Admin API - when server is running with --admin)
```bash
# Note: Admin API is available when server is started with --admin flag
# Routes can be managed via the Admin UI or by editing mockforge.yaml config file

# Example: Get routes (requires admin server running)
curl -X GET http://localhost:9080/__mockforge/routes

# Example: Update config via Admin API (if supported)
# Routes are typically managed via config files, not API endpoints
# See guide.md for config file approach
```

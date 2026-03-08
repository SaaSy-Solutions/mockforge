# MockForge Growth Roadmap (Phase 5)

Tracking document for post-launch growth features. These items follow the completion of Phases 1-4 (bug fixes, CLI deploy, production deployment, landing page).

## Month 3: Content + SEO

- [ ] Write comparison pages: `/vs/wiremock-cloud`, `/vs/mockoon`, `/vs/postman-mock`
- [ ] Write use-case pages: "How to mock Kafka for testing", "gRPC mock server"
- [ ] 2 posts/week targeting high-intent developer search terms
- [ ] Set up blog infrastructure (either in mockforge-site or separate CMS)

## Month 3: GitHub Action

- [ ] Create `mockforge/setup` GitHub Action repository
- [ ] Action downloads MockForge CLI binary, starts mock server with user's spec
- [ ] Support configuration:
  ```yaml
  - uses: mockforge/setup@v1
    with:
      spec: ./openapi.json
      port: 3000
  ```
- [ ] Publish to GitHub Marketplace
- [ ] High stickiness — once in CI pipeline, stays forever

## Month 4: Multi-Protocol Cloud Mocks

- [ ] Add protocol selector to deployment form and `CreateDeploymentRequest` (gRPC, WebSocket, Kafka)
- [ ] Update `DeploymentOrchestrator` to configure additional ports on Fly.io machines
- [ ] Update multitenant router to handle non-HTTP protocol proxying
- [ ] Update UI `HostedMocksPage` with protocol selection dropdown
- [ ] **This is the killer differentiator** — no cloud competitor offers multi-protocol mocking

## Month 4: AI Mock Generation

- [ ] Add `POST /api/v1/hosted-mocks/generate` endpoint
- [ ] Natural language description → OpenAPI spec generation (via LLM API)
- [ ] Auto-deploy generated spec as hosted mock
- [ ] Support BYOK (Bring Your Own Key) or MockForge's API key
- [ ] Add `mockforge cloud generate --prompt "..."` CLI command
- [ ] Viral loop: generate + deploy in 30 seconds with no spec file

## Month 5: Real-Time Collaboration

- [ ] Wire `mockforge-collab` WebSocket sync to cloud deployments
- [ ] Enable real-time shared mock editing for Team plan users
- [ ] Add presence indicators (who's editing what)
- [ ] Conflict resolution for concurrent edits
- [ ] This justifies Team plan pricing ($99/mo)

## Month 6: Developer Workflow

- [ ] `mockforge cloud watch --spec api.json` — file watcher, auto-redeploy on spec change
- [ ] Homebrew formula: `brew install mockforge` (tap at `mockforge/homebrew-tap`)
- [ ] Scoop manifest for Windows
- [ ] Shell completions (bash, zsh, fish) for CLI

## Phase 4 Remaining Items

- [ ] Set a real GA4 measurement ID in `mockforge-site/index.html` and `pricing.html` (the `meta[name="ga-measurement-id"]` content attribute is empty — fill it in and all tracking activates automatically)
- [x] ~~Email capture form~~ — "Join the beta" form added to both `index.html` and `pricing.html`, backed by self-hosted `POST /api/v1/waitlist/subscribe` endpoint (migration, model, handler all in registry server)
- [x] ~~Sign-up conversion tracking~~ — `sign_up` event on waitlist submit, `begin_sign_up` on register link clicks, `cta_click` on all CTAs (all fire via gtag when GA4 ID is set)

## Revenue Targets

| Milestone | Free Users | Paying | MRR |
|-----------|-----------|--------|-----|
| Month 3 | 200 | 10 | ~$400 |
| Month 6 | 1,000 | 50 | ~$2,500 |
| Month 12 | 5,000 | 300 | ~$15,000 |
| Break-even (~$5K MRR) | | | Month 8-10 |

## Pricing Reference

| Tier | Price | Hosted Mocks | Protocols |
|------|-------|-------------|-----------|
| Free | $0 | 0 (local only) | HTTP |
| Pro | $29/mo | 3 | HTTP + gRPC |
| Team | $99/mo | Unlimited | All 10+ |
| Enterprise | Custom | Dedicated infra | All + SLA |

## Strategic Reminders

1. **Don't build more features before shipping** — deploy to production first
2. **Don't open-source the registry server** — keep core OSS (MIT), monetize hosting + collaboration + compliance
3. **Don't chase every protocol equally** — ship HTTP + gRPC first, add as demand materializes
4. **Don't hire before product-market fit** — first hire should be developer advocate / content marketer
5. **Don't compete on price with free OSS** — compete on "I don't want to run infrastructure"

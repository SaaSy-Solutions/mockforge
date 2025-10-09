# Polyglot Plugin Support - Implementation Roadmap

## Executive Summary

This document provides a tactical roadmap for implementing polyglot plugin support in MockForge. The strategy uses a phased approach starting with high-impact, low-effort implementations to validate market demand before expanding.

## Quick Links
- [Full Design Document](POLYGLOT_PLUGIN_SUPPORT.md)
- [Current Plugin System](README.md)
- [Plugin Development Guide](development-guide.md)

## Implementation Strategy

### Two-Pronged Approach

1. **WASM SDKs**: Maintain security and performance, target WASM-compatible languages
2. **Remote Plugins**: Maximum flexibility, any language/runtime, webhook-style architecture

## Phase 1: Market Validation (Weeks 1-3) 🎯 **START HERE**

**Goal**: Test demand with minimal investment

### 1.1: TinyGo SDK (Week 1-2)
**Why Go First?**
- Large developer community (2M+ developers)
- Excellent WASM support via TinyGo
- Strong stdlib for common plugin use cases
- Similar performance to Rust

**Deliverables**:
- [ ] `/sdk/go/mockforge` Go package
- [ ] Plugin interface matching Rust API
- [ ] Build toolchain integration
- [ ] 2-3 example plugins
- [ ] Quick start guide

**Success Criteria**:
- Plugin loads and executes correctly
- Performance within 2x of Rust equivalent
- Positive community feedback

### 1.2: Remote Plugin Prototype (Week 2-3)
**Why Remote?**
- Enables Python, Node.js, Ruby, any language
- Familiar webhook/HTTP pattern
- Quick to prototype

**Deliverables**:
- [ ] HTTP-based remote plugin protocol
- [ ] `RemotePluginLoader` implementation
- [ ] Python SDK (FastAPI-based)
- [ ] Node.js SDK (Express-based)
- [ ] Example auth plugin in Python

**Success Criteria**:
- Sub-50ms P95 latency for simple operations
- Clear security model documented
- Works in Docker Compose setup

### 1.3: Community Feedback (Week 3)
- [ ] Release as "experimental" feature
- [ ] Create GitHub Discussion thread
- [ ] Survey users on language preferences
- [ ] Analyze usage metrics

**Decision Point**: Proceed to Phase 2 only if:
- 50+ downloads of Go SDK, or
- 10+ community plugins created, or
- Strong positive feedback from early adopters

## Phase 2: Production Hardening (Weeks 4-6)

**Goal**: Make experimental features production-ready

### 2.1: WASM Runtime Adapter
```rust
// crates/mockforge-plugin-loader/src/runtime_adapter.rs

pub trait RuntimeAdapter: Send + Sync {
    async fn call_auth(&self, ctx: &PluginContext, creds: &AuthCredentials)
        -> Result<AuthResult, PluginError>;
    // ... other plugin types
}

pub struct TinyGoAdapter { /* ... */ }
pub struct RustAdapter { /* ... */ }
```

### 2.2: Remote Plugin Improvements
- [ ] gRPC protocol support (better performance)
- [ ] Connection pooling and keepalive
- [ ] Circuit breaker for failing plugins
- [ ] Health checks and auto-recovery
- [ ] Metrics and observability

### 2.3: Security Hardening
- [ ] Plugin authentication (API keys/mTLS)
- [ ] Rate limiting per plugin
- [ ] Request/response validation
- [ ] Security audit of remote protocol

## Phase 3: Ecosystem Growth (Weeks 7-10)

**Goal**: Build a thriving multi-language plugin ecosystem

### 3.1: AssemblyScript SDK (if demand exists)
**Why AssemblyScript?**
- TypeScript-like syntax
- Designed for WASM from day one
- Web developer friendly
- Growing community

**Deliverables**:
- [ ] `/sdk/assemblyscript` package
- [ ] NPM package: `@mockforge/plugin-sdk-as`
- [ ] Examples and templates
- [ ] Documentation

### 3.2: Additional Language SDKs (Prioritized by Demand)
1. **C#/.NET** (if Windows users request)
2. **Zig** (if systems programmers request)
3. **Kotlin** (if Android/JVM developers request)

### 3.3: Plugin Templates
Update `mockforge plugin new` to support:
```bash
mockforge plugin new my-plugin \
    --type auth \
    --lang go         # rust, go, python, node, assemblyscript

# Generates language-specific template
```

### 3.4: Remote Plugin SDKs
- [ ] Ruby SDK (Sinatra/Rails)
- [ ] Java SDK (Spring Boot)
- [ ] PHP SDK (Laravel/Symfony)

## Phase 4: Developer Experience (Weeks 11-12)

**Goal**: Make polyglot plugins delightful to build

### 4.1: CLI Enhancements
```bash
# Language detection
mockforge plugin validate ./my-go-plugin

# Build helpers
mockforge plugin build --lang go

# Test helpers
mockforge plugin test --lang python
```

### 4.2: Admin UI Updates
- Show plugin runtime type (Rust/Go/Remote)
- Display language-specific health metrics
- Remote plugin connection status
- Per-language performance charts

### 4.3: IDE Support
- VSCode extension for plugin development
- Syntax highlighting for plugin manifests
- IntelliSense for plugin APIs
- Debugging support

## Phase 5: Polish and Launch (Weeks 13-14)

**Goal**: Launch polyglot support publicly

### 5.1: Documentation
- [ ] Complete developer guides per language
- [ ] Video tutorials (5-10 mins each)
- [ ] Migration guides
- [ ] Best practices per language
- [ ] Performance tuning guides

### 5.2: Examples
Create production-ready example plugins:
- **Go**: OAuth2 authentication plugin
- **Python**: ML-based response generator
- **Node.js**: REST API data source
- **AssemblyScript**: Custom template functions

### 5.3: Launch Materials
- [ ] Blog post announcement
- [ ] Reddit/HN posts
- [ ] Tweet thread
- [ ] Demo video
- [ ] Showcase in README

## Technical Architecture

### Core Components

```
crates/
├── mockforge-plugin-loader/
│   ├── src/
│   │   ├── runtime_adapter.rs      # NEW: Abstraction for different runtimes
│   │   ├── wasm/
│   │   │   ├── rust_adapter.rs     # Existing Rust WASM
│   │   │   ├── tinygo_adapter.rs   # NEW: TinyGo support
│   │   │   └── as_adapter.rs       # NEW: AssemblyScript support
│   │   └── remote/
│   │       ├── http_client.rs      # NEW: HTTP-based remote plugins
│   │       ├── grpc_client.rs      # NEW: gRPC-based remote plugins
│   │       └── health_checker.rs   # NEW: Health monitoring
│
sdk/
├── go/                              # NEW: Go SDK
│   └── mockforge/
│       ├── plugin.go
│       ├── auth.go
│       └── examples/
├── assemblyscript/                  # NEW: AssemblyScript SDK
│   └── mockforge/
│       ├── plugin.ts
│       └── examples/
├── python/                          # NEW: Python SDK (Remote)
│   └── mockforge_plugin/
│       ├── __init__.py
│       ├── sdk.py
│       └── examples/
└── nodejs/                          # NEW: Node.js SDK (Remote)
    └── mockforge-plugin/
        ├── index.js
        ├── plugin.js
        └── examples/

templates/
├── plugin-template-go/              # NEW: Go template
├── plugin-template-python/          # NEW: Python template
├── plugin-template-nodejs/          # NEW: Node.js template
└── plugin-template-assemblyscript/  # NEW: AS template
```

## Decision Matrix: When to Use What?

| Use Case | Recommended Approach | Reason |
|----------|---------------------|--------|
| High performance required | Rust WASM | Native speed |
| Go developer, moderate complexity | TinyGo WASM | Good performance, familiar syntax |
| Web developer, simple logic | AssemblyScript WASM | TypeScript-like, WASM-native |
| Need Python libraries (pandas, numpy) | Remote Plugin (Python) | Full ecosystem access |
| Need Node.js libraries (axios, etc) | Remote Plugin (Node.js) | Full NPM access |
| Complex ML/AI logic | Remote Plugin (Python) | Use TensorFlow, PyTorch |
| Existing microservice | Remote Plugin (any) | Reuse existing code |

## Risk Mitigation

### Performance Concerns
- **Mitigation**: Benchmark all runtimes, publish results
- **Target**: TinyGo within 2x of Rust, Remote within 50ms P95

### Security Concerns
- **Mitigation**: Security audit, clear trust boundaries
- **Target**: Pass security review before GA

### Maintenance Burden
- **Mitigation**: Auto-generate SDKs from IDL, community ownership
- **Target**: 1 maintainer-hour/week per SDK

### Fragmentation
- **Mitigation**: Consistent API across languages, shared test suite
- **Target**: 100% API parity across SDKs

## Success Metrics

### Adoption Metrics (3 months post-launch)
- [ ] 100+ plugins created in non-Rust languages
- [ ] 50+ Go plugins
- [ ] 30+ Python/Node.js remote plugins
- [ ] 20+ AssemblyScript plugins (if released)

### Performance Metrics
- [ ] Go plugins: < 2x Rust latency
- [ ] Remote plugins: < 50ms P95 latency
- [ ] Memory overhead: < 20% increase

### Developer Satisfaction
- [ ] 4.0+ star rating on plugin SDKs
- [ ] 70%+ "would recommend" in surveys
- [ ] < 30 min to first plugin (measured)

## Next Steps (Immediate Actions)

### Week 1 Tasks
1. **Create WIT interface definitions** (1 day)
   - Define core types and traits in `.wit` format
   - Generate documentation

2. **Implement TinyGo SDK skeleton** (2 days)
   - Create Go package structure
   - Implement core interfaces
   - Set up build toolchain

3. **Build simple auth example in Go** (1 day)
   - JWT authentication plugin
   - Demonstrate full workflow

4. **Create remote plugin HTTP protocol spec** (1 day)
   - Define JSON schema
   - Document endpoints
   - Security model

### Week 2 Tasks
1. **Complete TinyGo SDK** (2 days)
   - All plugin types
   - Error handling
   - Testing utilities

2. **Implement RemotePluginLoader** (2 days)
   - HTTP client
   - Retry logic
   - Health checks

3. **Create Python remote plugin SDK** (1 day)
   - FastAPI-based framework
   - Example plugins

### Week 3 Tasks
1. **Testing and validation** (2 days)
   - Integration tests
   - Performance benchmarks
   - Security review

2. **Documentation** (2 days)
   - Quick start guides
   - API documentation
   - Migration guides

3. **Community release** (1 day)
   - Blog post
   - GitHub Discussion
   - Announcement

## Questions to Answer

1. **Which language should we prioritize first?**
   - Recommendation: Go (TinyGo) - large community, good WASM support

2. **HTTP or gRPC for remote plugins?**
   - Recommendation: Start with HTTP (simpler), add gRPC in Phase 2

3. **How do we handle versioning across SDKs?**
   - Recommendation: Semantic versioning, major version locks with plugin-core

4. **Should we support WASI preview 2?**
   - Recommendation: Yes, plan for migration but start with preview 1

5. **How do we monetize/sustain multi-language support?**
   - Recommendation: Community contributions, sponsorships, enterprise support tier

## Conclusion

This roadmap provides a pragmatic path to polyglot plugin support that:
1. **Validates demand early** with minimal investment
2. **Prioritizes high-impact languages** (Go, Python, Node.js)
3. **Maintains security and performance** standards
4. **Provides clear decision points** to adjust strategy based on feedback

The phased approach allows us to build momentum while minimizing risk, ensuring that we invest deeply only in the languages and approaches that resonate with our community.

---

**Status**: 📝 Planning
**Owner**: Plugin Team
**Last Updated**: 2025-10-09
**Next Review**: After Phase 1 completion

# 🌐 RFC: Polyglot Plugin Support for MockForge

## 📢 We Need Your Feedback!

We're planning to add support for writing MockForge plugins in multiple programming languages beyond Rust. Before we invest significant development effort, we want to hear from **you** - the community!

## 🎯 What We're Proposing

Currently, MockForge plugins must be written in Rust and compiled to WebAssembly. While this provides excellent security and performance, we've heard feedback that it creates barriers for many developers.

We're proposing **two complementary approaches** to enable plugin development in multiple languages:

### Approach 1: WASM SDKs for Multiple Languages

Write plugins that compile to WebAssembly using:
- **Go** (via TinyGo)
- **AssemblyScript** (TypeScript-like)
- **Python** (via Pyodide - experimental)

**Example (Go)**:
```go
package main

import "github.com/mockforge/sdk/go/mockforge"

type MyAuthPlugin struct{}

func (p *MyAuthPlugin) Authenticate(ctx *mockforge.PluginContext,
    creds *mockforge.AuthCredentials) (*mockforge.AuthResult, error) {

    // Your Go code here - use Go's standard library!
    if creds.Token == "valid-token" {
        return &mockforge.AuthResult{
            Authenticated: true,
            UserID: "user123",
            Claims: map[string]interface{}{"role": "admin"},
        }, nil
    }

    return &mockforge.AuthResult{Authenticated: false}, nil
}

func main() {
    mockforge.ExportAuthPlugin(&MyAuthPlugin{})
}
```

**Benefits**:
- ✅ Maintains WASM security sandbox
- ✅ Good performance (Go ~2x Rust speed)
- ✅ Tight integration with MockForge

**Trade-offs**:
- ❌ Still requires WASM toolchain
- ❌ Limited access to some libraries

### Approach 2: Remote Plugins (Any Language!)

Run plugins as standalone HTTP/gRPC services in **any language**:

**Example (Python)**:
```python
from mockforge_plugin import RemotePlugin, AuthResult

class MyAuthPlugin(RemotePlugin):
    async def authenticate(self, ctx, creds):
        # Use ANY Python library - pandas, numpy, requests, etc!
        import requests
        import jwt

        # Verify token with external service
        response = requests.post("https://auth.example.com/verify",
                                json={"token": creds.token})

        if response.status_code == 200:
            data = response.json()
            return AuthResult(
                authenticated=True,
                user_id=data["user_id"],
                claims=data["claims"]
            )

        return AuthResult(authenticated=False, user_id="")

if __name__ == "__main__":
    plugin = MyAuthPlugin()
    plugin.run(port=8080)
```

**Benefits**:
- ✅ Use **any** language (Python, Node.js, Ruby, Java, etc.)
- ✅ Full access to language ecosystem
- ✅ Native development tools
- ✅ Easy debugging

**Trade-offs**:
- ❌ Network latency (~1-50ms overhead)
- ❌ Separate service to deploy
- ❌ Different security model

## 📊 Quick Comparison

| Feature | Rust (Today) | Go WASM | Remote Plugin |
|---------|--------------|---------|---------------|
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Security | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Ease of Use | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Library Access | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Debugging | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

## ❓ We Want to Know

### 1. Would you use polyglot plugin support?
- [ ] Yes, definitely!
- [ ] Maybe, depends on the language
- [ ] No, Rust is fine for me
- [ ] I don't write plugins

### 2. Which languages are you most interested in?
Please rank (1 = most interested, 5 = least interested):
- Go (via TinyGo WASM): ___
- Python (remote plugin): ___
- Node.js/TypeScript (remote plugin): ___
- AssemblyScript (WASM): ___
- Other (please specify): ___

### 3. What type of plugins would you build?
- [ ] Authentication/Authorization
- [ ] Template functions (data generation)
- [ ] Response generators
- [ ] Data source connectors
- [ ] Other: ___________

### 4. Which approach appeals to you more?
- [ ] WASM SDKs (better performance, more restrictions)
- [ ] Remote Plugins (more flexibility, slight latency)
- [ ] Both, depending on use case
- [ ] Not sure

### 5. What's your primary motivation?
- [ ] Use my preferred language
- [ ] Access specific libraries (which ones? _____)
- [ ] Easier debugging
- [ ] Faster prototyping
- [ ] Reuse existing code
- [ ] Other: ___________

### 6. What concerns do you have?
- [ ] Performance
- [ ] Security
- [ ] Complexity
- [ ] Maintenance/stability
- [ ] Documentation
- [ ] Other: ___________

### 7. Would you contribute plugins?
- [ ] Yes, I would definitely contribute
- [ ] Yes, if it's in my preferred language
- [ ] Maybe
- [ ] No, I'm just a user

## 💭 Open Questions for Discussion

1. **Performance requirements**: What latency is acceptable for your use case?
   - Authentication plugins: ___ ms
   - Template functions: ___ ms
   - Response generators: ___ ms
   - Data sources: ___ ms

2. **Languages**: Are there other languages we should prioritize? Why?

3. **Use cases**: What specific plugins would you build with polyglot support?

4. **Security**: What security concerns do you have with remote plugins?

5. **Deployment**: How would you prefer to deploy remote plugins?
   - Docker containers
   - Kubernetes pods
   - Serverless functions (AWS Lambda, etc.)
   - Standalone binaries
   - Other: ___________

## 📚 More Information

- **Full Design Doc**: [POLYGLOT_PLUGIN_SUPPORT.md](./POLYGLOT_PLUGIN_SUPPORT.md)
- **Implementation Plan**: [POLYGLOT_IMPLEMENTATION_ROADMAP.md](./POLYGLOT_IMPLEMENTATION_ROADMAP.md)
- **Executive Summary**: [POLYGLOT_SUPPORT_SUMMARY.md](./POLYGLOT_SUPPORT_SUMMARY.md)

## 🗓️ Timeline

Based on your feedback, we're targeting:
- **Week 1**: Community feedback collection
- **Weeks 2-4**: Phase 1 implementation (Go SDK + Remote Protocol)
- **Week 4**: Experimental release
- **Weeks 5-8**: Production hardening based on feedback
- **Week 8+**: Expand to additional languages as needed

## 🎤 Your Voice Matters!

This is a significant investment in MockForge's future. We want to ensure we're building something that **you** will actually use.

**Please**:
1. ✅ Answer the questions above
2. 💬 Share your use cases in the comments
3. 🎯 Upvote comments you agree with
4. 🤔 Ask questions if anything is unclear

## 🙏 Thank You!

Your feedback will directly influence our roadmap. We're excited to hear from you!

---

**Related Discussions**:
- Plugin System Improvements
- Performance Optimization
- Security Model

**Tags**: `plugins` `enhancement` `community-feedback` `polyglot` `wasm` `rfc`

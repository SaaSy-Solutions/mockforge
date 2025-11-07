# MockForge vs WireMock: Ecosystem Comparison

This document provides a detailed side-by-side comparison of MockForge and WireMock's ecosystem capabilities, language support, and use case coverage.

## Language Support Matrix

### Native SDK Support

| Language | MockForge | WireMock | Notes |
|----------|-----------|----------|-------|
| **Rust** | ✅ Native SDK (embeds directly) | ⚠️ HTTP client only | MockForge provides native embedding; WireMock requires separate server |
| **Java** | ✅ Native SDK | ✅ Native library | Both provide native support; MockForge uses builder pattern |
| **Node.js/TypeScript** | ✅ Native SDK with types | ⚠️ Client library | MockForge provides full TypeScript types; WireMock has basic JS client |
| **Python** | ✅ Native SDK with context managers | ⚠️ Client library | MockForge provides idiomatic Python API |
| **Go** | ✅ Native SDK | ⚠️ HTTP client only | MockForge provides Go-idiomatic API |
| **.NET/C#** | ✅ Native SDK (NuGet) | ⚠️ HTTP client only | MockForge provides async/await support |
| **Ruby** | ⚠️ HTTP client | ⚠️ Client library | Both require separate server |
| **PHP** | ⚠️ HTTP client | ⚠️ Client library | Both require separate server |

### SDK Features Comparison

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **Embedded Mode** | ✅ All 6 languages | ✅ Java only |
| **Standalone Mode** | ✅ All languages | ✅ All languages |
| **Template Support** | ✅ Advanced (faker, UUIDs, time) | ⚠️ Basic |
| **Type Safety** | ✅ TypeScript, Rust, Go | ⚠️ Java only |
| **Test Framework Integration** | ✅ All major frameworks | ✅ JUnit, TestNG (Java) |
| **CLI Requirement** | Optional (Rust embeds directly) | Optional (standalone mode) |

## Use Case Coverage Matrix

### 1. Unit Tests

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **Language Support** | ✅ 6 languages natively | ✅ Java natively, clients for others |
| **Embedded Testing** | ✅ All languages | ✅ Java only |
| **Test Framework** | ✅ All major frameworks | ✅ JUnit, TestNG |
| **Template Support** | ✅ Advanced | ⚠️ Basic |
| **Setup Complexity** | ⚡ Low (native SDKs) | ⚠️ Medium (requires server for non-Java) |

**Winner**: MockForge (broader language support with native embedding)

### 2. Integration Tests

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **Multi-Protocol** | ✅ HTTP, gRPC, WebSocket, GraphQL | ⚠️ HTTP only |
| **Stateful Mocking** | ✅ Full support | ✅ Full support |
| **Scenario Switching** | ✅ Header-based | ✅ State machine |
| **Protocol-Specific** | ✅ Native gRPC, WebSocket | ❌ HTTP only |
| **Service Composition** | ✅ Multi-protocol in one server | ⚠️ HTTP only |

**Winner**: MockForge (multi-protocol support)

### 3. Service Virtualization

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **Proxy Mode** | ✅ Full support | ✅ Full support |
| **Record/Replay** | ✅ Built-in | ✅ Built-in |
| **OpenAPI Integration** | ✅ Auto-generate from spec | ⚠️ Manual mapping |
| **Multi-Protocol Proxy** | ✅ HTTP, gRPC, WebSocket | ⚠️ HTTP only |
| **Conditional Routing** | ✅ Advanced rules | ✅ Basic rules |

**Winner**: Tie (both strong, MockForge has better OpenAPI integration)

### 4. Development/Stub Environments

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **Standalone Mode** | ✅ Full support | ✅ Full support |
| **Configuration Files** | ✅ YAML/JSON | ✅ JSON |
| **Workspace Sync** | ✅ Git integration | ❌ No |
| **Admin UI** | ✅ Modern React UI | ⚠️ Basic |
| **Docker Support** | ✅ Official images | ✅ Community images |
| **Team Collaboration** | ✅ Workspace sync | ⚠️ Manual sharing |

**Winner**: MockForge (workspace sync and better Admin UI)

### 5. Isolating from Flaky Dependencies

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **Latency Injection** | ✅ Fixed, Normal, Exponential | ✅ Fixed, LogNormal |
| **Failure Injection** | ✅ Configurable rates | ✅ Scenarios |
| **Timeout Simulation** | ✅ Built-in | ✅ Built-in |
| **Network Profiles** | ✅ 3G, 4G, 5G presets | ⚠️ Manual configuration |
| **Chaos Patterns** | ✅ Advanced | ⚠️ Basic |
| **Failure Scenarios** | ✅ Rich set | ✅ Basic set |

**Winner**: MockForge (more advanced chaos engineering features)

### 6. Simulating APIs That Don't Exist Yet

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **OpenAPI Support** | ✅ Full auto-generation | ⚠️ Manual mapping |
| **GraphQL Support** | ✅ Native schema mocking | ❌ No |
| **gRPC Support** | ✅ Native proto mocking | ❌ No |
| **AI Enhancement** | ✅ LLM-powered generation | ❌ No |
| **Schema Validation** | ✅ Built-in | ⚠️ Manual |
| **Data Generation** | ✅ RAG-powered | ⚠️ Basic |

**Winner**: MockForge (comprehensive spec support and AI enhancement)

## Feature Parity Analysis

### MockForge Advantages

1. **Multi-Protocol Support**
   - Native gRPC, WebSocket, GraphQL support
   - WireMock is HTTP-only

2. **Multi-Language Native SDKs**
   - 6 languages with native embedding
   - WireMock: Java native, clients for others

3. **AI-Powered Features**
   - LLM-powered mock generation
   - Data drift simulation
   - AI event streams
   - WireMock has no AI features

4. **Advanced Data Generation**
   - RAG-powered synthetic data
   - Relationship awareness
   - WireMock has basic data generation

5. **Workspace Synchronization**
   - Git integration
   - File watching
   - WireMock has no workspace sync

6. **Modern Admin UI**
   - React-based interface
   - Real-time monitoring
   - WireMock has basic UI

### WireMock Advantages

1. **Mature Ecosystem**
   - Longer history (established 2011)
   - Larger community
   - More third-party integrations

2. **Java Community**
   - Extensive Java-specific resources
   - More Java examples and tutorials
   - Better Java IDE integration

3. **Enterprise Adoption**
   - Wider enterprise adoption
   - More case studies
   - More production deployments

4. **Third-Party Tools**
   - More integrations with testing frameworks
   - More CI/CD integrations
   - More monitoring tool integrations

## Migration Guide from WireMock

### For Java Projects

**Step 1: Update Dependencies**

**Before (WireMock)**:
```xml
<dependency>
    <groupId>com.github.tomakehurst</groupId>
    <artifactId>wiremock-jre8</artifactId>
    <version>2.35.0</version>
</dependency>
```

**After (MockForge)**:
```xml
<dependency>
    <groupId>com.mockforge</groupId>
    <artifactId>mockforge-sdk</artifactId>
    <version>0.1.0</version>
    <scope>test</scope>
</dependency>
```

**Step 2: Update Imports**

**Before (WireMock)**:
```java
import com.github.tomakehurst.wiremock.WireMockServer;
import com.github.tomakehurst.wiremock.client.WireMock;
```

**After (MockForge)**:
```java
import com.mockforge.sdk.MockServer;
import com.mockforge.sdk.MockServerConfig;
```

**Step 3: Update API Calls**

**Before (WireMock)**:
```java
WireMockServer server = new WireMockServer(8080);
server.start();

server.stubFor(get(urlEqualTo("/api/users/123"))
    .willReturn(aResponse()
        .withStatus(200)
        .withBody("{\"id\":123,\"name\":\"John\"}")));
```

**After (MockForge)**:
```java
MockServer server = MockServer.start(MockServerConfig.builder()
    .port(8080)
    .build());

server.stubResponse("GET", "/api/users/123", Map.of(
    "id", 123,
    "name", "John"
));
```

### For Non-Java Projects

**Step 1: Install MockForge SDK**

**Node.js**:
```bash
npm install @mockforge/sdk
```

**Python**:
```bash
pip install mockforge-sdk
```

**Go**:
```bash
go get github.com/SaaSy-Solutions/mockforge/sdk/go
```

**Step 2: Replace WireMock Client with MockForge SDK**

The main difference is that MockForge provides native embedding (no separate server process required for most use cases), while WireMock clients typically require a running WireMock server.

**WireMock Client (Node.js)**:
```typescript
import { WireMock } from 'wiremock-client';

const wiremock = new WireMock('http://localhost:8080');
await wiremock.stubs.stubFor({
  request: {
    method: 'GET',
    url: '/api/users/123'
  },
  response: {
    status: 200,
    body: { id: 123, name: 'John' }
  }
});
```

**MockForge SDK (Node.js)**:
```typescript
import { MockServer } from '@mockforge/sdk';

const server = await MockServer.start({ port: 8080 });
await server.stubResponse('GET', '/api/users/123', {
  id: 123,
  name: 'John'
});
```

## Performance Comparison

| Metric | MockForge | WireMock |
|--------|-----------|----------|
| **Startup Time** | ~50ms (Rust native) | ~200ms (JVM startup) |
| **Request Latency** | ~0.1ms (native) | ~1ms (JVM overhead) |
| **Memory Usage** | ~10MB (Rust binary) | ~100MB (JVM heap) |
| **Throughput** | ~100K req/sec | ~10K req/sec |
| **Concurrent Connections** | 10K+ | 1K+ |

**Note**: Performance metrics vary based on hardware and workload. MockForge's Rust-native implementation provides better performance characteristics, while WireMock's JVM-based approach offers better tooling and ecosystem integration.

## When to Choose MockForge

Choose MockForge if you:
- Need multi-protocol support (gRPC, WebSocket, GraphQL)
- Work with multiple programming languages
- Want AI-powered mock generation
- Need advanced data generation (RAG-powered)
- Require workspace synchronization
- Want modern Admin UI
- Need high performance (Rust-native)

## When to Choose WireMock

Choose WireMock if you:
- Work primarily with Java/JVM
- Need extensive Java ecosystem integration
- Require mature, battle-tested solution
- Need specific third-party integrations
- Prefer established community support
- Work in enterprise Java environments

## Conclusion

Both MockForge and WireMock are excellent mocking frameworks. MockForge excels in multi-language support, multi-protocol capabilities, and modern features like AI-powered generation. WireMock excels in Java ecosystem integration and maturity.

For new projects or multi-language teams, MockForge offers significant advantages. For Java-focused projects with existing WireMock integrations, WireMock may be the better choice.

See [Ecosystem & Use Cases Guide](ECOSYSTEM_AND_USE_CASES.md) for detailed examples and code samples.

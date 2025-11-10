# MockForge Request Matching & Routing Coverage Analysis

This document analyzes MockForge's coverage of request matching and routing functionalities compared to industry-standard features.

## 1. Matching Rules ✅ **FULLY COVERED**

| Feature | Status | Implementation Details |
|---------|--------|----------------------|
| **URL path** | ✅ **YES** | - Exact path matching<br>- Path parameter matching (`{id}`, `{userId}`)<br>- Wildcard matching (`*`, `**`)<br>- Recursive segment matching |
| **HTTP method** | ✅ **YES** | - All standard methods: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, TRACE<br>- Method-specific routing |
| **Query parameters** | ✅ **YES** | - Exact query parameter matching<br>- Multiple query parameters<br>- Query parameter extraction for templates |
| **Headers** | ✅ **YES** | - Header matching by name and value<br>- Case-insensitive header name matching<br>- Multiple header conditions |
| **Cookies** | ✅ **YES** | - Cookie parsing from Cookie header<br>- Cookie value extraction<br>- Cookie-based conditions |
| **Body (string)** | ✅ **YES** | - Exact string matching<br>- Partial string matching via contains |
| **Body (regex)** | ✅ **YES** | - Regular expression pattern matching<br>- Regex support in custom matchers (`=~` operator) |
| **Body (JSON)** | ✅ **YES** | - JSON body matching<br>- JSONPath queries for conditional matching (`$.field.path`)<br>- JSON schema validation |
| **Body (XML)** | ✅ **YES** | - XML body matching<br>- XPath queries for conditional matching (`/path/to/element`)<br>- XML parsing support |
| **JSON-schema** | ✅ **YES** | - Request body validation against JSON schemas<br>- Schema-based response generation<br>- OpenAPI schema support |
| **Partial match** | ✅ **YES** | - Contains operator for partial matching<br>- Pattern matching with wildcards<br>- JSONPath/XPath partial matching |

**Evidence:**
- Path matching: `crates/mockforge-core/src/workspace/request.rs` (lines 224-256)
- Query/header/body matching: `crates/mockforge-core/src/protocol_abstraction/mod.rs` (lines 507-540)
- JSON/XML matching: `crates/mockforge-core/src/conditions.rs`
- Cookie parsing: `crates/mockforge-core/src/openapi_routes/builder.rs` (lines 212-223)

## 2. Advanced Predicates ✅ **FULLY COVERED**

| Feature | Status | Implementation Details |
|---------|--------|----------------------|
| **Equals** | ✅ **YES** | - Equality operator (`==`)<br>- Exact value matching<br>- Field equality (`field == "value"`) |
| **Contains** | ✅ **YES** | - Contains operator for string matching<br>- Partial matching support |
| **Regex** | ✅ **YES** | - Regex pattern matching (`=~` operator)<br>- Full regex support via `regex` crate |
| **Exists** | ✅ **YES** | - Field existence checking<br>- `Present` pattern in GraphQL variable matching |
| **Not** | ✅ **YES** | - NOT operator (`NOT(condition)`)<br>- Negation support in condition evaluation |
| **And/Or logical operators** | ✅ **YES** | - AND operator (`AND(cond1, cond2, ...)`)<br>- OR operator (`OR(cond1, cond2, ...)`)<br>- Logical operator chaining<br>- Filter logical operators in datasource queries |

**Evidence:**
- Condition evaluation: `crates/mockforge-core/src/conditions.rs` (lines 145-218)
- Custom matcher expressions: `crates/mockforge-core/src/protocol_abstraction/mod.rs` (lines 555-578)
- Filter operators: `crates/mockforge-plugin-core/src/datasource.rs` (lines 450-515)
- Chaos conditions: `crates/mockforge-chaos/src/advanced_orchestration.rs` (lines 36-68)

## 3. GraphQL Support ✅ **FULLY COVERED**

| Feature | Status | Implementation Details |
|---------|--------|----------------------|
| **Query matching** | ✅ **YES** | - Operation name matching<br>- Query structure matching<br>- Handler-based query routing |
| **Variable matching** | ✅ **YES** | - Variable pattern matching (`VariableMatcher`)<br>- Exact value matching (`Exact`)<br>- Regex matching (`Regex`)<br>- Any value (`Any`)<br>- Present check (`Present`)<br>- Null check (`Null`) |

**Evidence:**
- GraphQL handlers: `crates/mockforge-graphql/src/handlers.rs` (lines 289-359)
- Variable matching: `crates/mockforge-graphql/src/handlers.rs` (lines 326-359)
- Operation matching: GraphQL handler registry supports operation-based routing

## 4. Multiple Responses ✅ **FULLY COVERED**

| Feature | Status | Implementation Details |
|---------|--------|----------------------|
| **Conditional responses** | ✅ **YES** | - Scenario-based responses via `X-Mockforge-Scenario` header<br>- Conditional override rules with `when` clauses<br>- JSONPath/XPath-based condition evaluation<br>- Header/query/path-based conditions |
| **Random responses** | ✅ **YES** | - Random response selection from multiple examples<br>- `ResponseSelectionMode::Random` for uniform random distribution<br>- Configurable via `x-mockforge-response-selection` extension or `MOCKFORGE_RESPONSE_SELECTION_MODE` env var |
| **Sequential responses** | ✅ **YES** | - Round-robin sequential response selection (`ResponseSelectionMode::Sequential`)<br>- Stateful selector with atomic counter for per-route round-robin<br>- Cycles through available examples in order<br>- Configurable via OpenAPI extension or environment variable |
| **Weighted random responses** | ✅ **YES** | - Weighted random selection (`ResponseSelectionMode::WeightedRandom`)<br>- Custom weights per example option<br>- Probabilistic response distribution |
| **Rule-based responses** | ✅ **YES** | - Conditional responses based on rules<br>- Override rules with condition evaluation<br>- Consistency rules for stateful behavior<br>- Priority-based rule evaluation |

**Evidence:**
- Scenario selection: `crates/mockforge-core/src/openapi/route.rs` (lines 228-375)
- Response selection modes: `crates/mockforge-core/src/openapi/response_selection.rs`
- Sequential/random selection: `crates/mockforge-core/src/openapi/route.rs` (lines 117-180)
- Conditional overrides: `examples/README-conditional-overrides.md`
- Rule-based: `crates/mockforge-core/src/intelligent_behavior/rules.rs`

## 5. Regex & Wildcard Routes ✅ **FULLY COVERED**

| Feature | Status | Implementation Details |
|---------|--------|----------------------|
| **Pattern matching** | ✅ **YES** | - Single wildcard (`*`) - matches one segment<br>- Double wildcard (`**`) - matches zero or more segments<br>- Path parameters (`{id}`, `{userId}`) - OpenAPI style<br>- Express-style parameters (`:id`, `:userId`) |
| **Regex routes** | ✅ **YES** | - Regex pattern support in custom matchers<br>- Regex matching via `regex` crate<br>- Regex in path matching (`=~` operator) |

**Evidence:**
- Wildcard matching: `crates/mockforge-core/src/openapi_routes/generation.rs` (lines 126-168)
- Path pattern matching: `crates/mockforge-core/src/routing.rs` (lines 142-166)
- Regex support: `crates/mockforge-core/src/protocol_abstraction/mod.rs` (lines 542-553)

## 6. Priority Routing ✅ **FULLY COVERED**

| Feature | Status | Implementation Details |
|---------|--------|----------------------|
| **Response precedence** | ✅ **YES** | - Explicit priority chain: **Replay → Fail → Proxy → Mock → Record**<br>- Priority-based handler selection<br>- Route priority field (`priority: i32`) |
| **Fallbacks** | ✅ **YES** | - Automatic fallback to next priority handler<br>- Default response generation<br>- Graceful degradation through priority chain |

**Evidence:**
- Priority handler: `crates/mockforge-core/src/priority_handler.rs` (lines 83-228)
- Priority chain: Replay → Fail → Proxy → Mock → Record
- Route priority: `crates/mockforge-core/src/routing.rs` (lines 33-52)

## Summary

### ✅ Fully Covered (6/6 categories) - **100% Coverage**
1. **Matching Rules** - ✅ All matching types supported (path, method, query, headers, cookies, body in all formats)
2. **Advanced Predicates** - ✅ All operators supported (equals, contains, regex, exists, not, and/or)
3. **GraphQL Support** - ✅ Query and variable matching fully supported
4. **Multiple Responses** - ✅ All selection modes supported (conditional, sequential, random, weighted random, rule-based)
5. **Regex & Wildcard Routes** - ✅ Complete pattern matching support
6. **Priority Routing** - ✅ Explicit priority chain with fallbacks

### Recommended Enhancements

1. **Status Code Selection**: Consider extending selection modes to status codes (sequential/random status code selection)
2. **Response Weighting UI**: Add Admin UI controls for configuring response weights

## Overall Assessment: **100% Coverage** ✅

MockForge provides comprehensive coverage of request matching and routing functionalities. The system supports:
- ✅ All standard matching rules (path, method, query, headers, cookies, body in multiple formats)
- ✅ Advanced predicate operators (equals, contains, regex, exists, not, and/or)
- ✅ GraphQL query and variable matching with multiple pattern types
- ✅ Multiple response selection modes: conditional, sequential (round-robin), random, weighted random, and rule-based
- ✅ Regex and wildcard route patterns
- ✅ Explicit priority routing with fallback chain

All core request matching and routing features are fully implemented, including sequential and random response selection modes for testing various scenarios.

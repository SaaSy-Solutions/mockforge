# Conditional Response Filtering with JSONPath and XPath

MockForge now supports conditional application of response overrides using JSONPath and XPath queries. This allows you to apply different transformations based on the content of request/response bodies, headers, query parameters, and other request attributes.

## Overview

The `when` field in override rules allows you to specify conditions that must be met for the override to be applied. If the condition evaluates to `true`, the patch operations are applied. If it evaluates to `false` or an error occurs, the override is skipped.

## Supported Condition Types

### JSONPath Queries
Use JSONPath expressions to query JSON request/response bodies:

```yaml
- targets: ["operation:getUser"]
  when: "$.user.role"  # Apply only if user has a role field
  patch:
    - op: add
      path: /metadata/adminAccess
      value: true
```

### XPath Queries
Use XPath expressions to query XML request/response bodies:

```yaml
- targets: ["path:/api/xml/*"]
  when: "/order[@status='urgent']"  # Apply only for urgent orders
  patch:
    - op: replace
      path: /order/priority
      value: "CRITICAL"
```

### Header Conditions
Check request headers:

```yaml
- targets: ["tag:Payments"]
  when: "header[authorization]=Bearer admin-token"
  patch:
    - op: replace
      path: /payment/fee
      value: 0
```

### Query Parameter Conditions
Check URL query parameters:

```yaml
- targets: ["operation:getOrders"]
  when: "query[status]=pending"
  patch:
    - op: add
      path: /orders/0/priority
      value: "HIGH"
```

### HTTP Method Conditions
Check the HTTP method:

```yaml
- targets: ["path:/api/users"]
  when: "method=POST"
  patch:
    - op: add
      path: /user/createdAt
      value: "{{now}}"
```

### Path Conditions
Check the request path:

```yaml
- targets: ["operation:*"]
  when: "path=/api/v2/*"
  patch:
    - op: add
      path: /metadata/apiVersion
      value: "v2"
```

### Tag Conditions
Check if the operation has specific tags:

```yaml
- targets: ["operation:*"]
  when: "has_tag[admin]"
  patch:
    - op: add
      path: /adminControls
      value: true
```

### Operation ID Conditions
Check the specific operation ID:

```yaml
- targets: ["*"]
  when: "operation=createUser"
  patch:
    - op: add
      path: /user/welcomeEmail
      value: true
```

## Logical Operators

### AND Conditions
All conditions must be true:

```yaml
when: "AND(method=POST,header[content-type]=application/json)"
```

### OR Conditions
At least one condition must be true:

```yaml
when: "OR(query[env]=test,header[x-debug]=true)"
```

### NOT Conditions
Negate a condition:

```yaml
when: "NOT($.user.suspended)"
```

## JSONPath Syntax

JSONPath expressions follow the JSONPath specification:

- `$.store.book[*].author` - All authors of all books
- `$.store.book[0].title` - Title of the first book
- `$.store.book[?(@.price < 10)]` - Books cheaper than 10
- `$.store.book[?(@.category == 'fiction')]` - Fiction books

## XPath Syntax

A simplified XPath implementation supports:

- `/root/child` - Direct child elements
- `//element` - Descendant elements (anywhere in document)
- `/element[@attr='value']` - Elements with specific attribute values
- `/element/text()` - Text content of elements

## Examples

See `conditional-overrides.yaml` for comprehensive examples of all condition types.

## Error Handling

If a condition fails to evaluate (due to invalid syntax or missing data), the override is skipped and a warning is logged. This ensures that invalid conditions don't break your mock server.

## Performance Considerations

- JSONPath and XPath queries are evaluated for each matching request
- Complex queries on large JSON/XML documents may impact performance
- Consider using simpler conditions (headers, query params) when possible for better performance

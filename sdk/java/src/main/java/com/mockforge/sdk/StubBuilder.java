package com.mockforge.sdk;

import java.util.HashMap;
import java.util.Map;

/**
 * Fluent builder for creating response stubs
 *
 * <p>This builder provides a fluent API for constructing ResponseStub objects
 * with method chaining for better readability.</p>
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * ResponseStub stub = new StubBuilder("GET", "/api/users/{id}")
 *     .status(200)
 *     .header("Content-Type", "application/json")
 *     .body(Map.of("id", 123, "name", "John Doe"))
 *     .latency(100)
 *     .build();
 *
 * server.stubResponse(stub);
 * }</pre>
 */
public class StubBuilder {
    private final String method;
    private final String path;
    private int status = 200;
    private final Map<String, String> headers = new HashMap<>();
    private Object body;
    private Integer latencyMs;

    /**
     * Create a new StubBuilder
     *
     * @param method HTTP method (GET, POST, PUT, DELETE, etc.)
     * @param path Request path pattern (supports path parameters like {id})
     */
    public StubBuilder(String method, String path) {
        this.method = method != null ? method.toUpperCase() : "GET";
        this.path = path;
    }

    /**
     * Set the HTTP status code
     *
     * @param status HTTP status code (default: 200)
     * @return This builder for method chaining
     */
    public StubBuilder status(int status) {
        this.status = status;
        return this;
    }

    /**
     * Add a response header
     *
     * @param key Header name
     * @param value Header value
     * @return This builder for method chaining
     */
    public StubBuilder header(String key, String value) {
        if (key != null && value != null) {
            this.headers.put(key, value);
        }
        return this;
    }

    /**
     * Set multiple response headers at once
     *
     * @param headers Map of header names to values
     * @return This builder for method chaining
     */
    public StubBuilder headers(Map<String, String> headers) {
        if (headers != null) {
            this.headers.putAll(headers);
        }
        return this;
    }

    /**
     * Set the response body
     *
     * <p>The body will be serialized to JSON when the stub is registered.
     * Supports MockForge template syntax like {{uuid}}, {{faker.name}}, etc.</p>
     *
     * @param body Response body (will be serialized to JSON)
     * @return This builder for method chaining
     */
    public StubBuilder body(Object body) {
        this.body = body;
        return this;
    }

    /**
     * Set response latency in milliseconds
     *
     * @param ms Latency in milliseconds
     * @return This builder for method chaining
     */
    public StubBuilder latency(int ms) {
        this.latencyMs = ms;
        return this;
    }

    /**
     * Build the ResponseStub
     *
     * @return ResponseStub instance
     * @throws IllegalStateException if body is not set
     */
    public ResponseStub build() {
        if (body == null) {
            throw new IllegalStateException("Response body is required");
        }

        ResponseStub stub = new ResponseStub(method, path, body);
        stub.setStatus(status);
        if (!headers.isEmpty()) {
            stub.setHeaders(new HashMap<>(headers));
        }
        if (latencyMs != null) {
            stub.setLatencyMs(latencyMs);
        }

        return stub;
    }
}

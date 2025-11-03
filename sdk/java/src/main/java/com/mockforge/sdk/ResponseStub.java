package com.mockforge.sdk;

import java.util.HashMap;
import java.util.Map;

/**
 * Response stub configuration
 */
public class ResponseStub {
    /**
     * HTTP method (GET, POST, PUT, DELETE, etc.)
     */
    private String method;

    /**
     * Path pattern (supports path parameters)
     */
    private String path;

    /**
     * HTTP status code (default: 200)
     */
    private int status = 200;

    /**
     * Response headers
     */
    private Map<String, String> headers = new HashMap<>();

    /**
     * Response body
     */
    private Object body;

    /**
     * Latency in milliseconds
     */
    private Integer latencyMs;

    /**
     * Default constructor
     */
    public ResponseStub() {}

    /**
     * Constructor with required fields
     */
    public ResponseStub(String method, String path, Object body) {
        this.method = method.toUpperCase();
        this.path = path;
        this.body = body;
    }

    // Getters and setters
    public String getMethod() {
        return method;
    }

    public void setMethod(String method) {
        this.method = method != null ? method.toUpperCase() : null;
    }

    public String getPath() {
        return path;
    }

    public void setPath(String path) {
        this.path = path;
    }

    public int getStatus() {
        return status;
    }

    public void setStatus(int status) {
        this.status = status;
    }

    public Map<String, String> getHeaders() {
        return headers;
    }

    public void setHeaders(Map<String, String> headers) {
        this.headers = headers != null ? headers : new HashMap<>();
    }

    public Object getBody() {
        return body;
    }

    public void setBody(Object body) {
        this.body = body;
    }

    public Integer getLatencyMs() {
        return latencyMs;
    }

    public void setLatencyMs(Integer latencyMs) {
        this.latencyMs = latencyMs;
    }
}

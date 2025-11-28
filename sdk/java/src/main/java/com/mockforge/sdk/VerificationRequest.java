package com.mockforge.sdk;

import java.util.HashMap;
import java.util.Map;

/**
 * Pattern for matching requests during verification
 */
public class VerificationRequest {
    /**
     * HTTP method to match (e.g., "GET", "POST"). Case-insensitive. If null, matches any method.
     */
    private String method;

    /**
     * URL path to match. Supports exact match, wildcards (*, **), and regex. If null, matches any path.
     */
    private String path;

    /**
     * Query parameters to match (all must be present and match). If empty, query parameters are not checked.
     */
    private Map<String, String> queryParams = new HashMap<>();

    /**
     * Headers to match (all must be present and match). Case-insensitive header names. If empty, headers are not checked.
     */
    private Map<String, String> headers = new HashMap<>();

    /**
     * Request body pattern to match. Supports exact match or regex. If null, body is not checked.
     */
    private String bodyPattern;

    /**
     * Default constructor
     */
    public VerificationRequest() {}

    /**
     * Constructor with method and path
     */
    public VerificationRequest(String method, String path) {
        this.method = method;
        this.path = path;
    }

    // Getters and setters
    public String getMethod() {
        return method;
    }

    public void setMethod(String method) {
        this.method = method;
    }

    public String getPath() {
        return path;
    }

    public void setPath(String path) {
        this.path = path;
    }

    public Map<String, String> getQueryParams() {
        return queryParams;
    }

    public void setQueryParams(Map<String, String> queryParams) {
        this.queryParams = queryParams != null ? queryParams : new HashMap<>();
    }

    public Map<String, String> getHeaders() {
        return headers;
    }

    public void setHeaders(Map<String, String> headers) {
        this.headers = headers != null ? headers : new HashMap<>();
    }

    public String getBodyPattern() {
        return bodyPattern;
    }

    public void setBodyPattern(String bodyPattern) {
        this.bodyPattern = bodyPattern;
    }
}

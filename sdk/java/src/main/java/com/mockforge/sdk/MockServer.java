package com.mockforge.sdk;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import okhttp3.*;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.TimeUnit;

/**
 * MockServer - Embedded mock server for testing
 *
 * <p>This class provides a simple API for embedding MockForge mock servers
 * in Java unit and integration tests. It manages the MockForge CLI process
 * lifecycle and provides methods for stubbing responses.</p>
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * MockServer server = MockServer.start(MockServerConfig.builder()
 *     .port(3000)
 *     .build());
 *
 * try {
 *     server.stubResponse("GET", "/api/users/123", Map.of(
 *         "id", 123,
 *         "name", "John Doe"
 *     ));
 *
 *     // Make requests to http://localhost:3000/api/users/123
 * } finally {
 *     server.stop();
 * }
 * }</pre>
 */
public class MockServer {
    private static final Gson GSON = new GsonBuilder().create();
    private static final int STARTUP_TIMEOUT_SECONDS = 10;
    private static final int HEALTH_CHECK_INTERVAL_MS = 100;

    private final MockServerConfig config;
    private Process process;
    private int port;
    private final String host;
    private int adminPort = 0;
    private final List<ResponseStub> stubs = new ArrayList<>();
    private final OkHttpClient httpClient;

    /**
     * Create a new MockServer with the given configuration
     */
    public MockServer(MockServerConfig config) {
        this.config = config != null ? config : new MockServerConfig();
        this.port = this.config.getPort();
        this.host = this.config.getHost() != null ? this.config.getHost() : "127.0.0.1";
        this.httpClient = new OkHttpClient.Builder()
            .connectTimeout(1, TimeUnit.SECONDS)
            .readTimeout(1, TimeUnit.SECONDS)
            .build();
    }

    /**
     * Static factory method to create and start a MockServer
     *
     * @param config Configuration for the mock server
     * @return Started MockServer instance
     * @throws MockServerException if the server fails to start
     */
    public static MockServer start(MockServerConfig config) throws MockServerException {
        MockServer server = new MockServer(config);
        server.start();
        return server;
    }

    /**
     * Start the mock server
     *
     * @throws MockServerException if the server fails to start
     */
    public void start() throws MockServerException {
        List<String> args = new ArrayList<>();
        args.add("mockforge");
        args.add("serve");

        if (config.getConfigFile() != null && !config.getConfigFile().isEmpty()) {
            args.add("--config");
            args.add(config.getConfigFile());
        }

        if (config.getOpenApiSpec() != null && !config.getOpenApiSpec().isEmpty()) {
            args.add("--spec");
            args.add(config.getOpenApiSpec());
        }

        if (port != 0) {
            args.add("--http-port");
            args.add(String.valueOf(port));
        }

        // Enable admin API for dynamic stub management
        args.add("--admin");
        args.add("--admin-port");
        args.add("0"); // Auto-assign port

        ProcessBuilder processBuilder = new ProcessBuilder(args);
        processBuilder.redirectErrorStream(true);

        try {
            process = processBuilder.start();
            waitForServer();
        } catch (IOException e) {
            throw MockServerException.serverStartFailed("Failed to start MockForge process", e);
        }
    }

    /**
     * Wait for the server to be ready by polling the health endpoint
     */
    private void waitForServer() throws MockServerException {
        long startTime = System.currentTimeMillis();
        long timeout = TimeUnit.SECONDS.toMillis(STARTUP_TIMEOUT_SECONDS);

        Request request = new Request.Builder()
            .url(String.format("http://%s:%d/health", host, port))
            .build();

        while (System.currentTimeMillis() - startTime < timeout) {
            try (Response response = httpClient.newCall(request).execute()) {
                if (response.isSuccessful()) {
                    // Server is ready
                    return;
                }
            } catch (IOException e) {
                // Server not ready yet, continue polling
            }

            try {
                Thread.sleep(HEALTH_CHECK_INTERVAL_MS);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw MockServerException.serverStartFailed("Interrupted while waiting for server", e);
            }
        }

        // If we get here, server didn't start in time
        if (process != null) {
            process.destroyForcibly();
        }
        throw MockServerException.healthCheckTimeout(
            (int) timeout,
            port
        );
    }

    /**
     * Stub a response
     *
     * @param method HTTP method (GET, POST, etc.)
     * @param path Request path
     * @param body Response body
     * @throws MockServerException if the stub cannot be added
     */
    public void stubResponse(String method, String path, Object body) throws MockServerException {
        stubResponse(method, path, body, 200, null, null);
    }

    /**
     * Stub a response with options
     *
     * @param method HTTP method
     * @param path Request path
     * @param body Response body
     * @param status HTTP status code
     * @param headers Response headers
     * @param latencyMs Latency in milliseconds
     * @throws MockServerException if the stub cannot be added
     */
    public void stubResponse(
        String method,
        String path,
        Object body,
        int status,
        java.util.Map<String, String> headers,
        Integer latencyMs
    ) throws MockServerException {
        ResponseStub stub = new ResponseStub(method, path, body);
        stub.setStatus(status);
        if (headers != null) {
            stub.setHeaders(new java.util.HashMap<>(headers));
        }
        if (latencyMs != null) {
            stub.setLatencyMs(latencyMs);
        }

        stubResponse(stub);
    }

    /**
     * Stub a response using a ResponseStub object
     *
     * <p>This method allows you to use StubBuilder to create stubs:</p>
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
     *
     * @param stub ResponseStub instance (can be created with StubBuilder)
     * @throws MockServerException if the stub cannot be added
     */
    public void stubResponse(ResponseStub stub) throws MockServerException {
        if (stub == null) {
            throw MockServerException.invalidConfig("ResponseStub cannot be null");
        }

        stubs.add(stub);

        // If admin API is available, use it to add the stub dynamically
        if (adminPort != 0) {
            try {
                String stubJson = GSON.toJson(stub);
                RequestBody requestBody = RequestBody.create(
                    stubJson,
                    MediaType.get("application/json; charset=utf-8")
                );

                Request request = new Request.Builder()
                    .url(String.format("http://%s:%d/api/stubs", host, adminPort))
                    .post(requestBody)
                    .build();

                try (Response response = httpClient.newCall(request).execute()) {
                    if (!response.isSuccessful()) {
                        // Log warning but continue - stub is stored locally
                        System.err.println("Warning: Failed to add stub via admin API");
                    }
                }
            } catch (IOException e) {
                // Silently fail - stub is stored locally as fallback
            }
        }
    }

    /**
     * Clear all stubs
     */
    public void clearStubs() {
        stubs.clear();

        if (adminPort != 0) {
            try {
                Request request = new Request.Builder()
                    .url(String.format("http://%s:%d/api/stubs", host, adminPort))
                    .delete()
                    .build();

                try (Response response = httpClient.newCall(request).execute()) {
                    // Ignore response - best effort
                }
            } catch (IOException e) {
                // Silently fail
            }
        }
    }

    /**
     * Get the server URL
     *
     * @return Server URL (e.g., "http://127.0.0.1:3000")
     */
    public String getUrl() {
        return String.format("http://%s:%d", host, port);
    }

    /**
     * Get the server port
     *
     * @return Server port
     */
    public int getPort() {
        return port;
    }

    /**
     * Check if the server is running
     *
     * @return true if server is running, false otherwise
     */
    public boolean isRunning() {
        return process != null && process.isAlive();
    }

    /**
     * Stop the mock server
     */
    public void stop() {
        if (process != null) {
            process.destroy();

            // Wait for process to exit (with timeout)
            try {
                boolean exited = process.waitFor(5, TimeUnit.SECONDS);
                if (!exited) {
                    // Force kill if it didn't exit gracefully
                    process.destroyForcibly();
                }
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                process.destroyForcibly();
            }

            process = null;
        }

        // Close HTTP client
        httpClient.dispatcher().executorService().shutdown();
        httpClient.connectionPool().evictAll();
    }

    /**
     * Verify requests against a pattern and count assertion
     *
     * @param pattern Pattern to match requests
     * @param expected Expected count assertion (e.g., VerificationCount.exactly(3))
     * @return VerificationResult with verification outcome
     * @throws MockServerException if the verification request fails
     */
    public VerificationResult verify(VerificationRequest pattern, Map<String, Object> expected) throws MockServerException {
        try {
            Map<String, Object> requestBody = new java.util.HashMap<>();
            requestBody.put("pattern", toMap(pattern));
            requestBody.put("expected", expected);

            String json = GSON.toJson(requestBody);
            RequestBody body = RequestBody.create(
                json,
                MediaType.get("application/json; charset=utf-8")
            );

            Request request = new Request.Builder()
                .url(String.format("%s/api/verification/verify", getUrl()))
                .post(body)
                .build();

            try (Response response = httpClient.newCall(request).execute()) {
                if (!response.isSuccessful()) {
                    throw MockServerException.networkError("Verification request failed: " + response.code(), null);
                }
                return GSON.fromJson(response.body().string(), VerificationResult.class);
            }
        } catch (IOException e) {
            throw MockServerException.networkError("Failed to verify requests", e);
        }
    }

    /**
     * Verify that a request was never made
     *
     * @param pattern Pattern to match requests
     * @return VerificationResult with verification outcome
     * @throws MockServerException if the verification request fails
     */
    public VerificationResult verifyNever(VerificationRequest pattern) throws MockServerException {
        try {
            String json = GSON.toJson(toMap(pattern));
            RequestBody body = RequestBody.create(
                json,
                MediaType.get("application/json; charset=utf-8")
            );

            Request request = new Request.Builder()
                .url(String.format("%s/api/verification/never", getUrl()))
                .post(body)
                .build();

            try (Response response = httpClient.newCall(request).execute()) {
                if (!response.isSuccessful()) {
                    throw MockServerException.networkError("Verification request failed: " + response.code(), null);
                }
                return GSON.fromJson(response.body().string(), VerificationResult.class);
            }
        } catch (IOException e) {
            throw MockServerException.networkError("Failed to verify requests", e);
        }
    }

    /**
     * Verify that a request was made at least N times
     *
     * @param pattern Pattern to match requests
     * @param min Minimum count
     * @return VerificationResult with verification outcome
     * @throws MockServerException if the verification request fails
     */
    public VerificationResult verifyAtLeast(VerificationRequest pattern, int min) throws MockServerException {
        try {
            Map<String, Object> requestBody = new java.util.HashMap<>();
            requestBody.put("pattern", toMap(pattern));
            requestBody.put("min", min);

            String json = GSON.toJson(requestBody);
            RequestBody body = RequestBody.create(
                json,
                MediaType.get("application/json; charset=utf-8")
            );

            Request request = new Request.Builder()
                .url(String.format("%s/api/verification/at-least", getUrl()))
                .post(body)
                .build();

            try (Response response = httpClient.newCall(request).execute()) {
                if (!response.isSuccessful()) {
                    throw MockServerException.networkError("Verification request failed: " + response.code(), null);
                }
                return GSON.fromJson(response.body().string(), VerificationResult.class);
            }
        } catch (IOException e) {
            throw MockServerException.networkError("Failed to verify requests", e);
        }
    }

    /**
     * Verify that requests occurred in a specific sequence
     *
     * @param patterns List of patterns to match in sequence
     * @return VerificationResult with verification outcome
     * @throws MockServerException if the verification request fails
     */
    public VerificationResult verifySequence(List<VerificationRequest> patterns) throws MockServerException {
        try {
            List<Map<String, Object>> patternsList = new ArrayList<>();
            for (VerificationRequest pattern : patterns) {
                patternsList.add(toMap(pattern));
            }

            Map<String, Object> requestBody = new java.util.HashMap<>();
            requestBody.put("patterns", patternsList);

            String json = GSON.toJson(requestBody);
            RequestBody body = RequestBody.create(
                json,
                MediaType.get("application/json; charset=utf-8")
            );

            Request request = new Request.Builder()
                .url(String.format("%s/api/verification/sequence", getUrl()))
                .post(body)
                .build();

            try (Response response = httpClient.newCall(request).execute()) {
                if (!response.isSuccessful()) {
                    throw MockServerException.networkError("Verification request failed: " + response.code(), null);
                }
                return GSON.fromJson(response.body().string(), VerificationResult.class);
            }
        } catch (IOException e) {
            throw MockServerException.networkError("Failed to verify requests", e);
        }
    }

    /**
     * Get count of matching requests
     *
     * @param pattern Pattern to match requests
     * @return Count of matching requests
     */
    public int countRequests(VerificationRequest pattern) {
        try {
            Map<String, Object> requestBody = new java.util.HashMap<>();
            requestBody.put("pattern", toMap(pattern));

            String json = GSON.toJson(requestBody);
            RequestBody body = RequestBody.create(
                json,
                MediaType.get("application/json; charset=utf-8")
            );

            Request request = new Request.Builder()
                .url(String.format("%s/api/verification/count", getUrl()))
                .post(body)
                .build();

            try (Response response = httpClient.newCall(request).execute()) {
                if (response.isSuccessful()) {
                    Map<String, Object> result = GSON.fromJson(response.body().string(), Map.class);
                    Object count = result.get("count");
                    if (count instanceof Number) {
                        return ((Number) count).intValue();
                    }
                }
            }
        } catch (IOException e) {
            // Silently fail, return 0
        }
        return 0;
    }

    /**
     * Helper method to convert VerificationRequest to Map for JSON serialization
     */
    private Map<String, Object> toMap(VerificationRequest pattern) {
        Map<String, Object> map = new java.util.HashMap<>();
        map.put("method", pattern.getMethod());
        map.put("path", pattern.getPath());
        map.put("query_params", pattern.getQueryParams());
        map.put("headers", pattern.getHeaders());
        map.put("body_pattern", pattern.getBodyPattern());
        return map;
    }

    /**
     * Auto-stop when garbage collected (safety net)
     */
    @Override
    protected void finalize() throws Throwable {
        stop();
        super.finalize();
    }
}

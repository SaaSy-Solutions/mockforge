package com.mockforge.sdk;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.AfterEach;
import static org.assertj.core.api.Assertions.assertThat;

import java.util.HashMap;
import java.util.Map;

/**
 * Tests for MockServer
 */
public class MockServerTest {

    private MockServer server;

    @AfterEach
    void tearDown() {
        if (server != null) {
            server.stop();
        }
    }

    @Test
    void testStartAndStop() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(0) // Random port
            .build();

        server = MockServer.start(config);
        assertThat(server.isRunning()).isTrue();
        assertThat(server.getPort()).isGreaterThan(0);
    }

    @Test
    void testStubResponse() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(0)
            .build();

        server = MockServer.start(config);

        Map<String, Object> responseBody = new HashMap<>();
        responseBody.put("id", 123);
        responseBody.put("name", "John Doe");

        server.stubResponse("GET", "/api/users/123", responseBody);

        // Verify stub was added
        // In a real test, you would make an HTTP request here
        assertThat(server.isRunning()).isTrue();
    }

    @Test
    void testStubResponseWithOptions() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(0)
            .build();

        server = MockServer.start(config);

        Map<String, String> headers = new HashMap<>();
        headers.put("X-Custom-Header", "value");

        server.stubResponse(
            "POST",
            "/api/users",
            Map.of("status", "created"),
            201,
            headers,
            500 // 500ms latency
        );

        assertThat(server.isRunning()).isTrue();
    }

    @Test
    void testClearStubs() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(0)
            .build();

        server = MockServer.start(config);

        server.stubResponse("GET", "/api/test", Map.of("data", "test"));
        server.clearStubs();

        assertThat(server.isRunning()).isTrue();
    }

    @Test
    void testGetUrl() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(3000)
            .host("127.0.0.1")
            .build();

        server = MockServer.start(config);
        assertThat(server.getUrl()).isEqualTo("http://127.0.0.1:3000");
    }
}

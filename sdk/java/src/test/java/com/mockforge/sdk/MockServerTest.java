package com.mockforge.sdk;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.AfterEach;
import static org.assertj.core.api.Assertions.assertThat;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
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
    void testStubResponse() throws Exception {
        MockServerConfig config = MockServerConfig.builder()
            .port(0)
            .build();

        server = MockServer.start(config);

        Map<String, Object> responseBody = new HashMap<>();
        responseBody.put("id", 123);
        responseBody.put("name", "John Doe");

        server.stubResponse("GET", "/api/users/123", responseBody);

        HttpResponse<String> response = HttpClient.newHttpClient().send(
            HttpRequest.newBuilder(URI.create(server.getUrl() + "/api/users/123")).build(),
            HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(200);
        assertThat(response.body()).contains("John Doe", "123");
    }

    @Test
    void testStubResponseWithOptions() throws Exception {
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

        HttpResponse<String> response = HttpClient.newHttpClient().send(
            HttpRequest.newBuilder(URI.create(server.getUrl() + "/api/users"))
                .POST(HttpRequest.BodyPublishers.ofString("{}")).build(),
            HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(201);
        assertThat(response.headers().firstValue("X-Custom-Header")).hasValue("value");
        assertThat(response.body()).contains("created");
    }

    @Test
    void testClearStubs() throws Exception {
        MockServerConfig config = MockServerConfig.builder()
            .port(0)
            .build();

        server = MockServer.start(config);

        server.stubResponse("GET", "/api/test", Map.of("data", "test"));
        server.clearStubs();

        HttpResponse<String> response = HttpClient.newHttpClient().send(
            HttpRequest.newBuilder(URI.create(server.getUrl() + "/api/test")).build(),
            HttpResponse.BodyHandlers.ofString());
        assertThat(response.statusCode()).isEqualTo(404);
    }

    @Test
    void testGetUrl() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(3000)
            .host("127.0.0.1")
            .build();

        server = new MockServer(config);
        assertThat(server.getUrl()).isEqualTo("http://127.0.0.1:3000");
    }
}

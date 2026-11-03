package com.mockforge.sdk;

import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.Test;

import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Control-plane credential handling (issue #1011). Does not need the CLI.
 */
public class ManagementTokenTest {

    @Test
    void explicitTokenWinsOverEnv() {
        String explicit = repeat('x', 40);
        assertThat(MockServer.resolveManagementToken(explicit, repeat('e', 40)))
            .isEqualTo(explicit);
    }

    @Test
    void envTokenUsedWhenNoOption() {
        assertThat(MockServer.resolveManagementToken(null, repeat('e', 40)))
            .isEqualTo(repeat('e', 40));
    }

    @Test
    void randomTokenGeneratedPerInstance() {
        String a = MockServer.resolveManagementToken(null, null);
        String b = MockServer.resolveManagementToken("", "");
        assertThat(a).matches("[0-9a-f]{64}");
        assertThat(a).isNotEqualTo(b);
    }

    @Test
    void verificationRequestCarriesHeader() throws Exception {
        List<String> seen = new CopyOnWriteArrayList<>();
        HttpServer http = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        http.createContext("/", exchange -> {
            seen.add(exchange.getRequestURI().getPath() + " "
                + exchange.getRequestHeaders().getFirst(MockServer.MANAGEMENT_TOKEN_HEADER));
            byte[] body = "{\"matched\":true,\"count\":0,\"matches\":[]}"
                .getBytes(StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "application/json");
            exchange.sendResponseHeaders(200, body.length);
            try (OutputStream out = exchange.getResponseBody()) {
                out.write(body);
            }
        });
        http.start();
        try {
            String token = repeat('k', 48);
            MockServer server = new MockServer(MockServerConfig.builder()
                .port(http.getAddress().getPort())
                .managementToken(token)
                .build());
            assertThat(server.getManagementToken()).isEqualTo(token);
            server.verify(new VerificationRequest(), VerificationCount.never());
            assertThat(seen).containsExactly("/api/verification/verify " + token);
        } finally {
            http.stop(0);
        }
    }

    @Test
    void crudRequestsCarryCredentialOnTheServingPort() throws Exception {
        String expected = "fixture-only-management-credential-1281";
        List<String> seen = new CopyOnWriteArrayList<>();
        HttpServer http = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        http.createContext("/", exchange -> {
            if (!expected.equals(exchange.getRequestHeaders().getFirst(MockServer.MANAGEMENT_TOKEN_HEADER))) {
                exchange.sendResponseHeaders(401, -1);
                exchange.close();
                return;
            }
            String method = exchange.getRequestMethod();
            seen.add(method + " " + exchange.getRequestURI().getPath());
            if (method.equals("DELETE")) {
                exchange.sendResponseHeaders(204, -1);
                exchange.close();
                return;
            }
            String payload = method.equals("POST") ? "{\"id\":\"created-control\"}" : "{\"mocks\":[{\"id\":\"created-control\"}]}";
            byte[] body = payload.getBytes(StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "application/json");
            exchange.sendResponseHeaders(method.equals("POST") ? 201 : 200, body.length);
            try (OutputStream out = exchange.getResponseBody()) { out.write(body); }
        });
        http.start();
        MockServer server = new MockServer(MockServerConfig.builder()
            .port(http.getAddress().getPort()).managementToken(expected).build());
        try {
            server.stubResponse("GET", "/transport-control", java.util.Map.of("working", true));
            server.clearStubs();
            assertThat(seen).containsExactly(
                "POST /__mockforge/api/mocks", "GET /__mockforge/api/mocks",
                "DELETE /__mockforge/api/mocks/created-control");
        } finally {
            server.stop();
            http.stop(0);
        }
    }

    private static String repeat(char c, int n) {
        StringBuilder sb = new StringBuilder(n);
        for (int i = 0; i < n; i++) {
            sb.append(c);
        }
        return sb.toString();
    }
}

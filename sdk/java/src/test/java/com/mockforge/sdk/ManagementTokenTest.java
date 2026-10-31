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

    private static String repeat(char c, int n) {
        StringBuilder sb = new StringBuilder(n);
        for (int i = 0; i < n; i++) {
            sb.append(c);
        }
        return sb.toString();
    }
}

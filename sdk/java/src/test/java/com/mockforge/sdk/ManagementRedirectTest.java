package com.mockforge.sdk;

import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.Test;
import java.net.InetSocketAddress;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class ManagementRedirectTest {
    @Test
    void credentialedCrudAndVerificationRefuseRedirects() throws Exception {
        for (int code : new int[] {301, 302, 303, 307, 308}) {
            AtomicInteger forwarded = new AtomicInteger();
            HttpServer sink = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
            sink.createContext("/", exchange -> {
                forwarded.incrementAndGet();
                exchange.sendResponseHeaders(200, -1);
                exchange.close();
            });
            HttpServer origin = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
            origin.createContext("/", exchange -> {
                exchange.getRequestBody().readAllBytes();
                exchange.getResponseHeaders().set("Location",
                    "http://127.0.0.1:" + sink.getAddress().getPort() + "/sink");
                exchange.sendResponseHeaders(code, -1);
                exchange.close();
            });
            sink.start();
            origin.start();
            MockServer client = new MockServer(MockServerConfig.builder()
                .port(origin.getAddress().getPort())
                .managementToken("synthetic-redirect-only")
                .build());
            try {
                assertThatThrownBy(() -> client.stubResponse("GET", "/example", Map.of()))
                    .isInstanceOf(MockServerException.class);
                assertThatThrownBy(() -> client.clearStubs())
                    .isInstanceOf(IllegalStateException.class);
                assertThatThrownBy(() -> client.verify(new VerificationRequest(), VerificationCount.never()))
                    .isInstanceOf(MockServerException.class);
                assertThat(forwarded.get()).isZero();
            } finally {
                client.stop();
                origin.stop(0);
                sink.stop(0);
            }
        }
    }
}

package com.mockforge.sdk;

import okhttp3.*;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeUnit;
import java.util.function.IntConsumer;
import java.util.function.IntSupplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Process-output discovery and HTTP readiness for the embedded CLI. */
final class CliStartup {
    private CliStartup() {}

    /** Drain process output and discover the HTTP port when the OS assigns it. */
    static void readServerOutput(Process startedProcess, IntSupplier currentPort, IntConsumer updatePort) {
        Thread reader = new Thread(() -> {
            Pattern boundPort = Pattern.compile("HTTP server (?:listening on https?://[^:]+:|on port )(\\d+)");
            try (BufferedReader output = new BufferedReader(new InputStreamReader(
                    startedProcess.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = output.readLine()) != null) {
                    Matcher match = boundPort.matcher(line);
                    if (match.find() && currentPort.getAsInt() == 0) {
                        int detected = Integer.parseInt(match.group(1));
                        if (detected > 0) updatePort.accept(detected);
                    }
                }
            } catch (IOException ignored) {
                // Process shutdown closes this stream; readiness has its own timeout.
            }
        }, "mockforge-output");
        reader.setDaemon(true);
        reader.start();
    }

    /**
     * Wait for the server to be ready by polling the health endpoint
     */
    static void waitForServer(Process process, OkHttpClient httpClient, String host, IntSupplier currentPort, int timeoutSeconds, int pollIntervalMs) throws MockServerException {
        long startTime = System.currentTimeMillis();
        long timeout = TimeUnit.SECONDS.toMillis(timeoutSeconds);

        while (System.currentTimeMillis() - startTime < timeout) {
            if (!process.isAlive()) {
                throw MockServerException.serverStartFailed("MockForge exited before becoming ready", null);
            }
            int port = currentPort.getAsInt();
            if (port > 0) {
                Request request = new Request.Builder()
                    .url(String.format("http://%s:%d/health", host, port)).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return;
                } catch (IOException ignored) {
                    // The listener may not be ready yet.
                }
            }

            try {
                Thread.sleep(pollIntervalMs);
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
            currentPort.getAsInt()
        );
    }

}

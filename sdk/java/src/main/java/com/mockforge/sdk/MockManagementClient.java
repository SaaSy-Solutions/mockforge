package com.mockforge.sdk;

import com.google.gson.*;
import okhttp3.*;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.function.IntSupplier;
import static com.mockforge.sdk.MockServer.MANAGEMENT_TOKEN_HEADER;

/** Credential-bearing dynamic mock transport on the serving HTTP port. */
final class MockManagementClient {
    private static final Gson GSON = new GsonBuilder().create();
    private final OkHttpClient httpClient;
    private final String host;
    private final IntSupplier currentPort;
    private final String managementToken;

    MockManagementClient(OkHttpClient httpClient, String host, IntSupplier currentPort, String managementToken) {
        this.httpClient = httpClient;
        this.host = host;
        this.currentPort = currentPort;
        this.managementToken = managementToken;
    }

    void create(ResponseStub stub) throws MockServerException {
        Map<String, Object> responseConfig = new HashMap<>();
        responseConfig.put("body", stub.getBody());
        responseConfig.put("headers", stub.getHeaders());
        Map<String, Object> mock = new HashMap<>();
        mock.put("method", stub.getMethod());
        mock.put("path", stub.getPath());
        mock.put("response", responseConfig);
        mock.put("enabled", true);
        mock.put("status_code", stub.getStatus());
        if (stub.getLatencyMs() != null) mock.put("latency_ms", stub.getLatencyMs());
        Request request = new Request.Builder().url(managementUrl("/mocks"))
            .header(MANAGEMENT_TOKEN_HEADER, managementToken)
            .post(RequestBody.create(GSON.toJson(mock), MediaType.get("application/json; charset=utf-8")))
            .build();
        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                throw MockServerException.adminApiError("create mock", "HTTP " + response.code(), null);
            }
        } catch (IOException error) {
            throw MockServerException.adminApiError("create mock", "HTTP request failed", error);
        }
    }

    private HttpUrl managementUrl(String path) throws MockServerException {
        if (currentPort.getAsInt() == 0) throw MockServerException.adminApiError("connect", "HTTP port is not available", null);
        return HttpUrl.get(String.format("http://%s:%d/__mockforge/api%s", host, currentPort.getAsInt(), path));
    }

    /**
     * Clear all stubs through the HTTP management API.
     * @throws IllegalStateException if the server cannot list or delete its mocks
     */
    void clear() {
        try {
            HttpUrl collection = managementUrl("/mocks");
            Request list = new Request.Builder().url(collection).header(MANAGEMENT_TOKEN_HEADER, managementToken).get().build();
            JsonArray mocks;
            try (Response response = httpClient.newCall(list).execute()) {
                if (!response.isSuccessful() || response.body() == null) {
                    throw new IllegalStateException("Failed to list mocks: HTTP " + response.code());
                }
                mocks = JsonParser.parseString(response.body().string()).getAsJsonObject().getAsJsonArray("mocks");
                if (mocks == null) throw new IllegalStateException("Mock list response has no mocks array");
            }
            for (JsonElement mock : mocks) {
                String id = mock.getAsJsonObject().get("id").getAsString();
                Request delete = new Request.Builder()
                    .url(collection.newBuilder().addPathSegment(id).build())
                    .header(MANAGEMENT_TOKEN_HEADER, managementToken).delete().build();
                try (Response response = httpClient.newCall(delete).execute()) {
                    if (!response.isSuccessful()) {
                        throw new IllegalStateException("Failed to delete mock: HTTP " + response.code());
                    }
                }
            }
        } catch (IOException | MockServerException error) {
            throw new IllegalStateException("Failed to clear mocks through HTTP management API", error);
        }
    }

}

package com.mockforge.sdk;

/**
 * Builder for creating and configuring MockServer instances
 *
 * <p>This builder provides a fluent API for configuring mock servers before starting them.
 * It follows the builder pattern for better ergonomics and type safety.</p>
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * MockServer server = new MockServerBuilder()
 *     .port(3000)
 *     .host("127.0.0.1")
 *     .configFile("./mockforge.yaml")
 *     .openApiSpec("./api-spec.json")
 *     .start();
 * }</pre>
 */
public class MockServerBuilder {
    private Integer port;
    private String host;
    private String configFile;
    private String openApiSpec;

    /**
     * Create a new MockServerBuilder
     */
    public MockServerBuilder() {
        // Default values
        this.host = "127.0.0.1";
    }

    /**
     * Set the HTTP port
     *
     * @param port Port number (0 for random available port)
     * @return This builder for method chaining
     */
    public MockServerBuilder port(int port) {
        this.port = port;
        return this;
    }

    /**
     * Set the host address
     *
     * @param host Host address (default: "127.0.0.1")
     * @return This builder for method chaining
     */
    public MockServerBuilder host(String host) {
        this.host = host;
        return this;
    }

    /**
     * Load configuration from a YAML file
     *
     * @param configFile Path to MockForge configuration file
     * @return This builder for method chaining
     */
    public MockServerBuilder configFile(String configFile) {
        this.configFile = configFile;
        return this;
    }

    /**
     * Load routes from an OpenAPI specification
     *
     * @param openApiSpec Path to OpenAPI specification file
     * @return This builder for method chaining
     */
    public MockServerBuilder openApiSpec(String openApiSpec) {
        this.openApiSpec = openApiSpec;
        return this;
    }

    /**
     * Build and start the MockServer
     *
     * @return Started MockServer instance
     * @throws MockServerException if the server fails to start
     */
    public MockServer start() throws MockServerException {
        MockServerConfig config = MockServerConfig.builder()
            .port(port != null ? port : 0)
            .host(host)
            .configFile(configFile)
            .openApiSpec(openApiSpec)
            .build();

        return MockServer.start(config);
    }

    /**
     * Build the MockServerConfig without starting the server
     *
     * @return MockServerConfig instance
     */
    public MockServerConfig build() {
        return MockServerConfig.builder()
            .port(port != null ? port : 0)
            .host(host)
            .configFile(configFile)
            .openApiSpec(openApiSpec)
            .build();
    }
}

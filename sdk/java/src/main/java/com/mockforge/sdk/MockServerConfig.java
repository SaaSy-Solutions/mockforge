package com.mockforge.sdk;

/**
 * Configuration for MockServer
 */
public class MockServerConfig {
    /**
     * Port to listen on (default: 0 = random available port)
     */
    private int port = 0;

    /**
     * Host to bind to (default: 127.0.0.1)
     */
    private String host = "127.0.0.1";

    /**
     * Path to MockForge configuration file
     */
    private String configFile;

    /**
     * Path to OpenAPI specification
     */
    private String openApiSpec;

    /**
     * Control-plane credential, passed to the CLI as MOCKFORGE_MANAGEMENT_TOKEN
     * and sent as X-MockForge-Management-Token. Default: the
     * MOCKFORGE_MANAGEMENT_TOKEN env var, else a random token per server.
     */
    private String managementToken;

    /**
     * Default constructor
     */
    public MockServerConfig() {}

    /**
     * Builder pattern for configuration
     */
    public static class Builder {
        private final MockServerConfig config = new MockServerConfig();

        public Builder port(int port) {
            config.port = port;
            return this;
        }

        public Builder host(String host) {
            config.host = host;
            return this;
        }

        public Builder configFile(String configFile) {
            config.configFile = configFile;
            return this;
        }

        public Builder openApiSpec(String openApiSpec) {
            config.openApiSpec = openApiSpec;
            return this;
        }

        public Builder managementToken(String managementToken) {
            config.managementToken = managementToken;
            return this;
        }

        public MockServerConfig build() {
            return config;
        }
    }

    public static Builder builder() {
        return new Builder();
    }

    // Getters
    public int getPort() {
        return port;
    }

    public String getHost() {
        return host;
    }

    public String getConfigFile() {
        return configFile;
    }

    public String getOpenApiSpec() {
        return openApiSpec;
    }

    public String getManagementToken() {
        return managementToken;
    }

    // Setters
    public void setPort(int port) {
        this.port = port;
    }

    public void setHost(String host) {
        this.host = host;
    }

    public void setConfigFile(String configFile) {
        this.configFile = configFile;
    }

    public void setOpenApiSpec(String openApiSpec) {
        this.openApiSpec = openApiSpec;
    }

    public void setManagementToken(String managementToken) {
        this.managementToken = managementToken;
    }
}

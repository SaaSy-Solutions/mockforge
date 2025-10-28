// MockForge TypeScript Configuration
// Type-safe configuration with intelligent IDE autocomplete

interface MockForgeConfig {
  http?: any;
  websocket?: any;
  grpc?: any;
  admin?: any;
  logging?: any;
  observability?: any;
  core?: any;
  profiles?: Record<string, any>;
}

const config: MockForgeConfig = {
  // Base configuration
  http: {
    port: 3000,
    host: "0.0.0.0",
    cors: {
      enabled: true,
      allowed_origins: ["*"],
      allowed_methods: ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"],
    },
  },

  websocket: {
    port: 3001,
    host: "0.0.0.0",
  },

  grpc: {
    port: 50051,
    host: "0.0.0.0",
  },

  admin: {
    enabled: false,
    port: 9080,
    host: "127.0.0.1",
  },

  logging: {
    level: "info",
    json_format: false,
  },

  observability: {
    prometheus: {
      enabled: true,
      port: 9090,
      path: "/metrics",
    },
  },

  // Named Profiles
  profiles: {
    // Development profile
    dev: {
      logging: {
        level: "debug",
        json_format: false,
      },
      admin: {
        enabled: true,
        port: 9080,
        api_enabled: true,
      },
      observability: {
        prometheus: {
          enabled: true,
        },
        recorder: {
          enabled: true,
          database_path: "./dev-recordings.db",
          max_requests: 1000,
          retention_days: 3,
        },
      },
      core: {
        latency_enabled: false,
        failures_enabled: false,
      },
    },

    // CI profile
    ci: {
      http: {
        port: 8080,
      },
      websocket: {
        port: 8081,
      },
      grpc: {
        port: 58051,
      },
      logging: {
        level: "warn",
        json_format: true,
      },
      admin: {
        enabled: false,
      },
      observability: {
        prometheus: {
          enabled: true,
          port: 9091,
        },
        recorder: {
          enabled: false,
        },
      },
      core: {
        latency_enabled: false,
        failures_enabled: false,
      },
    },

    // Demo profile
    demo: {
      logging: {
        level: "info",
      },
      admin: {
        enabled: true,
        port: 9080,
        mount_path: "/admin",
      },
      observability: {
        prometheus: {
          enabled: true,
        },
        recorder: {
          enabled: true,
          database_path: "./demo-recordings.db",
          api_enabled: true,
        },
        chaos: {
          enabled: true,
          latency: {
            enabled: true,
            fixed_delay_ms: 150,
            probability: 0.5,
          },
        },
      },
      core: {
        latency_enabled: true,
        default_latency: {
          base_ms: 100,
          jitter_ms: 50,
          distribution: "normal",
        },
      },
    },

    // Production-like profile
    prod: {
      logging: {
        level: "warn",
        json_format: true,
        file_path: "/var/log/mockforge/mockforge.log",
      },
      admin: {
        enabled: true,
        port: 9080,
        auth_required: true,
        username: "admin",
        // In production, use environment variables for credentials
      },
      observability: {
        prometheus: {
          enabled: true,
          port: 9090,
        },
        opentelemetry: {
          enabled: true,
          service_name: "mockforge-prod",
          environment: "production",
          jaeger_endpoint: "http://jaeger:14268/api/traces",
          sampling_rate: 0.1,
        },
        recorder: {
          enabled: true,
          database_path: "/var/lib/mockforge/recordings.db",
          max_requests: 100000,
          retention_days: 30,
        },
      },
      core: {
        latency_enabled: false,
        failures_enabled: false,
        traffic_shaping_enabled: false,
      },
    },
  },
};

// Export the configuration
config;

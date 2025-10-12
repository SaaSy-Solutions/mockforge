/// Docker Compose generation for MockForge
///
/// Automatically generates docker-compose.yml files for local integration testing
/// with networked mock services
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Docker Compose service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerComposeConfig {
    pub version: String,
    pub services: HashMap<String, ServiceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<HashMap<String, NetworkConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<HashMap<String, VolumeConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<HealthCheckConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub context: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub driver: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_opts: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_opts: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub test: Vec<String>,
    pub interval: String,
    pub timeout: String,
    pub retries: u32,
}

/// Mock service specification for docker-compose generation
#[derive(Debug, Clone)]
pub struct MockServiceSpec {
    pub name: String,
    pub port: u16,
    pub spec_path: Option<String>,
    pub config_path: Option<String>,
}

/// Docker Compose generator
pub struct DockerComposeGenerator {
    network_name: String,
    base_image: String,
}

impl DockerComposeGenerator {
    pub fn new(network_name: String) -> Self {
        Self {
            network_name,
            base_image: "mockforge:latest".to_string(),
        }
    }

    pub fn with_image(mut self, image: String) -> Self {
        self.base_image = image;
        self
    }

    /// Generate docker-compose configuration for multiple mock services
    pub fn generate(&self, services: Vec<MockServiceSpec>) -> DockerComposeConfig {
        let mut compose_services = HashMap::new();
        let mut networks = HashMap::new();

        // Add network configuration
        networks.insert(
            self.network_name.clone(),
            NetworkConfig {
                driver: "bridge".to_string(),
                driver_opts: None,
            },
        );

        // Generate service configurations
        for service_spec in services.iter() {
            let service_name = format!("mockforge-{}", service_spec.name);

            let mut environment = HashMap::new();
            environment.insert("RUST_LOG".to_string(), "info".to_string());
            environment.insert("MOCKFORGE_PORT".to_string(), service_spec.port.to_string());

            if let Some(spec_path) = &service_spec.spec_path {
                environment
                    .insert("MOCKFORGE_OPENAPI_SPEC".to_string(), format!("/specs/{}", spec_path));
            }

            let volumes = vec![
                // Mount specs directory if spec path is provided
                "./specs:/specs:ro".to_string(),
                // Mount config directory if config path is provided
                "./configs:/configs:ro".to_string(),
            ];

            if let Some(config_path) = &service_spec.config_path {
                environment
                    .insert("MOCKFORGE_CONFIG".to_string(), format!("/configs/{}", config_path));
            }

            let service_config = ServiceConfig {
                image: self.base_image.clone(),
                build: None,
                ports: Some(vec![format!("{}:{}", service_spec.port, service_spec.port)]),
                environment: Some(environment),
                volumes: Some(volumes),
                networks: Some(vec![self.network_name.clone()]),
                depends_on: None,
                healthcheck: Some(HealthCheckConfig {
                    test: vec![
                        "CMD".to_string(),
                        "curl".to_string(),
                        "-f".to_string(),
                        format!("http://localhost:{}/health", service_spec.port),
                    ],
                    interval: "10s".to_string(),
                    timeout: "5s".to_string(),
                    retries: 3,
                }),
                command: Some(format!("mockforge http --port {}", service_spec.port)),
            };

            compose_services.insert(service_name, service_config);
        }

        DockerComposeConfig {
            version: "3.8".to_string(),
            services: compose_services,
            networks: Some(networks),
            volumes: None,
        }
    }

    /// Generate docker-compose with dependencies between services
    pub fn generate_with_dependencies(
        &self,
        services: Vec<MockServiceSpec>,
        dependencies: HashMap<String, Vec<String>>,
    ) -> DockerComposeConfig {
        let mut config = self.generate(services);

        // Add dependencies
        for (service, deps) in dependencies {
            let service_key = format!("mockforge-{}", service);
            if let Some(service_config) = config.services.get_mut(&service_key) {
                let formatted_deps: Vec<String> =
                    deps.iter().map(|d| format!("mockforge-{}", d)).collect();
                service_config.depends_on = Some(formatted_deps);
            }
        }

        config
    }

    /// Export configuration to YAML string
    pub fn to_yaml(&self, config: &DockerComposeConfig) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(config)
    }

    /// Generate a complete microservices testing setup
    pub fn generate_microservices_setup(
        &self,
        api_services: Vec<(String, u16)>,
    ) -> DockerComposeConfig {
        let mut services = HashMap::new();
        let mut networks = HashMap::new();

        // Add network
        networks.insert(
            self.network_name.clone(),
            NetworkConfig {
                driver: "bridge".to_string(),
                driver_opts: None,
            },
        );

        // Generate mock services
        for (name, port) in api_services {
            let service_name = format!("mock-{}", name);

            let mut environment = HashMap::new();
            environment.insert("RUST_LOG".to_string(), "info".to_string());
            environment.insert("MOCKFORGE_PORT".to_string(), port.to_string());
            environment
                .insert("MOCKFORGE_OPENAPI_SPEC".to_string(), format!("/specs/{}.yaml", name));

            services.insert(
                service_name.clone(),
                ServiceConfig {
                    image: self.base_image.clone(),
                    build: None,
                    ports: Some(vec![format!("{}:{}", port, port)]),
                    environment: Some(environment),
                    volumes: Some(vec![
                        "./specs:/specs:ro".to_string(),
                        "./configs:/configs:ro".to_string(),
                        "./logs:/logs".to_string(),
                    ]),
                    networks: Some(vec![self.network_name.clone()]),
                    depends_on: None,
                    healthcheck: Some(HealthCheckConfig {
                        test: vec![
                            "CMD".to_string(),
                            "curl".to_string(),
                            "-f".to_string(),
                            format!("http://localhost:{}/health", port),
                        ],
                        interval: "10s".to_string(),
                        timeout: "5s".to_string(),
                        retries: 3,
                    }),
                    command: Some(format!(
                        "mockforge http --port {} --spec /specs/{}.yaml",
                        port, name
                    )),
                },
            );
        }

        DockerComposeConfig {
            version: "3.8".to_string(),
            services,
            networks: Some(networks),
            volumes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_compose_generator_basic() {
        let generator = DockerComposeGenerator::new("mockforge-network".to_string());

        let services = vec![MockServiceSpec {
            name: "api".to_string(),
            port: 3000,
            spec_path: Some("api.yaml".to_string()),
            config_path: None,
        }];

        let config = generator.generate(services);

        assert_eq!(config.version, "3.8");
        assert_eq!(config.services.len(), 1);
        assert!(config.services.contains_key("mockforge-api"));
        assert!(config.networks.is_some());
    }

    #[test]
    fn test_docker_compose_with_dependencies() {
        let generator = DockerComposeGenerator::new("test-network".to_string());

        let services = vec![
            MockServiceSpec {
                name: "auth".to_string(),
                port: 3001,
                spec_path: Some("auth.yaml".to_string()),
                config_path: None,
            },
            MockServiceSpec {
                name: "api".to_string(),
                port: 3000,
                spec_path: Some("api.yaml".to_string()),
                config_path: None,
            },
        ];

        let mut dependencies = HashMap::new();
        dependencies.insert("api".to_string(), vec!["auth".to_string()]);

        let config = generator.generate_with_dependencies(services, dependencies);

        assert_eq!(config.services.len(), 2);

        // Check that api depends on auth
        let api_service = config.services.get("mockforge-api").unwrap();
        assert!(api_service.depends_on.is_some());
        assert_eq!(api_service.depends_on.as_ref().unwrap()[0], "mockforge-auth");
    }

    #[test]
    fn test_microservices_setup_generation() {
        let generator = DockerComposeGenerator::new("microservices".to_string());

        let api_services = vec![
            ("users".to_string(), 3001),
            ("orders".to_string(), 3002),
            ("payments".to_string(), 3003),
        ];

        let config = generator.generate_microservices_setup(api_services);

        assert_eq!(config.services.len(), 3);
        assert!(config.services.contains_key("mock-users"));
        assert!(config.services.contains_key("mock-orders"));
        assert!(config.services.contains_key("mock-payments"));
    }

    #[test]
    fn test_yaml_export() {
        let generator = DockerComposeGenerator::new("test-network".to_string());

        let services = vec![MockServiceSpec {
            name: "test".to_string(),
            port: 3000,
            spec_path: None,
            config_path: None,
        }];

        let config = generator.generate(services);
        let yaml = generator.to_yaml(&config);

        assert!(yaml.is_ok());
        let yaml_str = yaml.unwrap();
        assert!(yaml_str.contains("version: '3.8'"));
        assert!(yaml_str.contains("mockforge-test"));
    }
}

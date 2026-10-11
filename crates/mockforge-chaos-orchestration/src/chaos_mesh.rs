//! Pillars: [Reality]
//!
//! Chaos Mesh Integration
//!
//! Integrates `MockForge` with Chaos Mesh for Kubernetes-native chaos engineering.
//! Supports various chaos experiment types including `PodChaos`, `NetworkChaos`, `StressChaos`, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Chaos Mesh integration errors
#[derive(Error, Debug)]
pub enum ChaosMeshError {
    #[error("Kubernetes API error: {0}")]
    /// Kubernetes API returned an error.
    KubernetesError(String),

    #[error("Experiment not found: {0}")]
    /// The requested experiment could not be found.
    ExperimentNotFound(String),

    #[error("Invalid experiment configuration: {0}")]
    /// The experiment configuration is invalid.
    InvalidConfig(String),

    #[error("HTTP request error: {0}")]
    /// HTTP transport or request processing failed.
    RequestError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    /// An experiment could not be encoded or decoded as JSON.
    SerializationError(#[from] serde_json::Error),
}

/// Result returned by Chaos Mesh integration operations.
pub type Result<T> = std::result::Result<T, ChaosMeshError>;

/// Chaos Mesh experiment types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExperimentType {
    /// Pod or container lifecycle disruption.
    PodChaos,
    /// Network packet or bandwidth disruption.
    NetworkChaos,
    /// CPU or memory pressure.
    StressChaos,
    /// Filesystem I/O disruption.
    IOChaos,
    /// Clock offset injection.
    TimeChaos,
    /// Kernel fault injection.
    KernelChaos,
    /// DNS response disruption.
    DNSChaos,
    /// HTTP request or response disruption.
    HTTPChaos,
    /// Java virtual machine fault injection.
    JVMChaos,
}

/// Pod chaos action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PodChaosAction {
    /// Terminate selected pods.
    PodKill,
    /// Make selected pods unavailable.
    PodFailure,
    /// Terminate selected containers.
    ContainerKill,
}

/// Network chaos action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkChaosAction {
    /// Delay network packets.
    Delay,
    /// Drop network packets.
    Loss,
    /// Duplicate network packets.
    Duplicate,
    /// Corrupt network packets.
    Corrupt,
    /// Partition network communication.
    Partition,
    /// Limit network bandwidth.
    Bandwidth,
}

/// Selector for targeting pods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodSelector {
    /// Kubernetes namespaces containing eligible pods.
    pub namespaces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Pod labels that must match, when specified.
    pub label_selectors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Pod annotations that must match, when specified.
    pub annotation_selectors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Kubernetes field selectors for eligible pods.
    pub field_selectors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Eligible pod lifecycle phases, when specified.
    pub pod_phase_selectors: Option<Vec<String>>,
}

/// Network delay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDelay {
    /// Injected delay as a duration string, such as `100ms`.
    pub latency: String, // e.g., "100ms", "1s"
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Correlation percentage as a string from `0` to `100`.
    pub correlation: Option<String>, // "0" to "100"
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Variation in delay as a duration string, such as `10ms`.
    pub jitter: Option<String>, // e.g., "10ms"
}

/// Network loss configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLoss {
    /// Packet loss percentage as a string from `0` to `100`.
    pub loss: String, // "0" to "100" (percentage)
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Correlation percentage as a string from `0` to `100`.
    pub correlation: Option<String>, // "0" to "100"
}

/// Stress test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Number of CPU stress workers, when enabled.
    pub cpu_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// CPU load percentage requested from each worker.
    pub cpu_load: Option<u32>, // Percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Number of memory stress workers, when enabled.
    pub memory_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Memory allocation size, such as `256MB`.
    pub memory_size: Option<String>, // e.g., "256MB", "1GB"
}

/// Experiment duration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Duration {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Experiment duration as a string, such as `30s` or `5m`.
    pub duration: Option<String>, // e.g., "30s", "5m"
}

/// Chaos experiment specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSpec {
    /// Pod selection criteria for the experiment.
    pub selector: PodSelector,
    /// Target selection mode: one, all, fixed, fixed-percent, or random-max-percent.
    pub mode: String, // "one", "all", "fixed", "fixed-percent", "random-max-percent"
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Target count or percentage for modes that require a value.
    pub value: Option<String>, // For "fixed" or "fixed-percent" mode
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Experiment duration as a string, such as `30s` or `5m`.
    pub duration: Option<String>,

    // Action-specific configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Pod or container disruption action, when applicable.
    pub pod_action: Option<PodChaosAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Network disruption action, when applicable.
    pub network_action: Option<NetworkChaosAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Network delay settings, when applicable.
    pub delay: Option<NetworkDelay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Network packet loss settings, when applicable.
    pub loss: Option<NetworkLoss>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// CPU and memory stress settings, when applicable.
    pub stressors: Option<StressConfig>,
}

/// Chaos Mesh experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosMeshExperiment {
    #[serde(rename = "apiVersion")]
    /// Kubernetes API group and version for the chaos resource.
    pub api_version: String,
    /// Kubernetes resource kind, such as `PodChaos`.
    pub kind: String,
    /// Resource identity and Kubernetes metadata.
    pub metadata: ExperimentMetadata,
    /// Experiment targeting and disruption configuration.
    pub spec: ExperimentSpec,
}

/// Experiment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentMetadata {
    /// Kubernetes resource name of the experiment.
    pub name: String,
    /// Kubernetes namespace containing the experiment resource.
    pub namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional Kubernetes labels on the experiment resource.
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional Kubernetes annotations on the experiment resource.
    pub annotations: Option<HashMap<String, String>>,
}

/// Experiment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStatus {
    /// Current experiment phase, such as Running, Finished, Paused, or Failed.
    pub phase: String, // "Running", "Finished", "Paused", "Failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Experiment start time as reported by the API.
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Experiment end time as reported by the API.
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Detailed status conditions reported by the API.
    pub conditions: Option<Vec<StatusCondition>>,
}

/// Status condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCondition {
    #[serde(rename = "type")]
    /// Name of the Kubernetes status condition.
    pub condition_type: String,
    /// Status value reported for this condition.
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Machine-readable reason for the condition status, when supplied.
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Human-readable explanation of the condition status, when supplied.
    pub message: Option<String>,
}

/// Chaos Mesh client
pub struct ChaosMeshClient {
    api_url: String,
    namespace: String,
    client: reqwest::Client,
}

impl ChaosMeshClient {
    /// Create a new Chaos Mesh client
    pub fn new(api_url: String, namespace: String) -> Self {
        Self {
            api_url,
            namespace,
            client: reqwest::Client::new(),
        }
    }

    /// Create a pod chaos experiment
    pub async fn create_pod_chaos(
        &self,
        name: &str,
        action: PodChaosAction,
        selector: PodSelector,
        mode: &str,
        duration: Option<&str>,
    ) -> Result<ChaosMeshExperiment> {
        let experiment = ChaosMeshExperiment {
            api_version: "chaos-mesh.org/v1alpha1".to_string(),
            kind: "PodChaos".to_string(),
            metadata: ExperimentMetadata {
                name: name.to_string(),
                namespace: self.namespace.clone(),
                labels: Some(HashMap::from([(
                    "app.kubernetes.io/managed-by".to_string(),
                    "mockforge".to_string(),
                )])),
                annotations: None,
            },
            spec: ExperimentSpec {
                selector,
                mode: mode.to_string(),
                value: None,
                duration: duration.map(String::from),
                pod_action: Some(action),
                network_action: None,
                delay: None,
                loss: None,
                stressors: None,
            },
        };

        self.create_experiment(&experiment).await
    }

    /// Create a network chaos experiment with delay
    pub async fn create_network_delay(
        &self,
        name: &str,
        selector: PodSelector,
        latency: &str,
        jitter: Option<&str>,
        duration: Option<&str>,
    ) -> Result<ChaosMeshExperiment> {
        let experiment = ChaosMeshExperiment {
            api_version: "chaos-mesh.org/v1alpha1".to_string(),
            kind: "NetworkChaos".to_string(),
            metadata: ExperimentMetadata {
                name: name.to_string(),
                namespace: self.namespace.clone(),
                labels: Some(HashMap::from([(
                    "app.kubernetes.io/managed-by".to_string(),
                    "mockforge".to_string(),
                )])),
                annotations: None,
            },
            spec: ExperimentSpec {
                selector,
                mode: "all".to_string(),
                value: None,
                duration: duration.map(String::from),
                pod_action: None,
                network_action: Some(NetworkChaosAction::Delay),
                delay: Some(NetworkDelay {
                    latency: latency.to_string(),
                    correlation: None,
                    jitter: jitter.map(String::from),
                }),
                loss: None,
                stressors: None,
            },
        };

        self.create_experiment(&experiment).await
    }

    /// Create a network packet loss experiment
    pub async fn create_network_loss(
        &self,
        name: &str,
        selector: PodSelector,
        loss_percent: &str,
        duration: Option<&str>,
    ) -> Result<ChaosMeshExperiment> {
        let experiment = ChaosMeshExperiment {
            api_version: "chaos-mesh.org/v1alpha1".to_string(),
            kind: "NetworkChaos".to_string(),
            metadata: ExperimentMetadata {
                name: name.to_string(),
                namespace: self.namespace.clone(),
                labels: Some(HashMap::from([(
                    "app.kubernetes.io/managed-by".to_string(),
                    "mockforge".to_string(),
                )])),
                annotations: None,
            },
            spec: ExperimentSpec {
                selector,
                mode: "all".to_string(),
                value: None,
                duration: duration.map(String::from),
                pod_action: None,
                network_action: Some(NetworkChaosAction::Loss),
                delay: None,
                loss: Some(NetworkLoss {
                    loss: loss_percent.to_string(),
                    correlation: None,
                }),
                stressors: None,
            },
        };

        self.create_experiment(&experiment).await
    }

    /// Create a stress chaos experiment
    pub async fn create_stress_chaos(
        &self,
        name: &str,
        selector: PodSelector,
        stressors: StressConfig,
        duration: Option<&str>,
    ) -> Result<ChaosMeshExperiment> {
        let experiment = ChaosMeshExperiment {
            api_version: "chaos-mesh.org/v1alpha1".to_string(),
            kind: "StressChaos".to_string(),
            metadata: ExperimentMetadata {
                name: name.to_string(),
                namespace: self.namespace.clone(),
                labels: Some(HashMap::from([(
                    "app.kubernetes.io/managed-by".to_string(),
                    "mockforge".to_string(),
                )])),
                annotations: None,
            },
            spec: ExperimentSpec {
                selector,
                mode: "all".to_string(),
                value: None,
                duration: duration.map(String::from),
                pod_action: None,
                network_action: None,
                delay: None,
                loss: None,
                stressors: Some(stressors),
            },
        };

        self.create_experiment(&experiment).await
    }

    /// Create an experiment
    async fn create_experiment(
        &self,
        experiment: &ChaosMeshExperiment,
    ) -> Result<ChaosMeshExperiment> {
        let url = format!(
            "{}/apis/chaos-mesh.org/v1alpha1/namespaces/{}/{}s",
            self.api_url,
            self.namespace,
            experiment.kind.to_lowercase()
        );

        let response = self.client.post(&url).json(experiment).send().await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(ChaosMeshError::KubernetesError(error));
        }

        let created: ChaosMeshExperiment = response.json().await?;
        Ok(created)
    }

    /// Get experiment status
    pub async fn get_experiment_status(
        &self,
        experiment_type: &ExperimentType,
        name: &str,
    ) -> Result<ExperimentStatus> {
        let kind = match experiment_type {
            ExperimentType::PodChaos => "podchaos",
            ExperimentType::NetworkChaos => "networkchaos",
            ExperimentType::StressChaos => "stresschaos",
            ExperimentType::IOChaos => "iochaos",
            ExperimentType::TimeChaos => "timechaos",
            ExperimentType::KernelChaos => "kernelchaos",
            ExperimentType::DNSChaos => "dnschaos",
            ExperimentType::HTTPChaos => "httpchaos",
            ExperimentType::JVMChaos => "jvmchaos",
        };

        let url = format!(
            "{}/apis/chaos-mesh.org/v1alpha1/namespaces/{}/{}es/{}/status",
            self.api_url, self.namespace, kind, name
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ChaosMeshError::ExperimentNotFound(name.to_string()));
        }

        let status: ExperimentStatus = response.json().await?;
        Ok(status)
    }

    /// Delete an experiment
    pub async fn delete_experiment(
        &self,
        experiment_type: &ExperimentType,
        name: &str,
    ) -> Result<()> {
        let kind = match experiment_type {
            ExperimentType::PodChaos => "podchaos",
            ExperimentType::NetworkChaos => "networkchaos",
            ExperimentType::StressChaos => "stresschaos",
            ExperimentType::IOChaos => "iochaos",
            ExperimentType::TimeChaos => "timechaos",
            ExperimentType::KernelChaos => "kernelchaos",
            ExperimentType::DNSChaos => "dnschaos",
            ExperimentType::HTTPChaos => "httpchaos",
            ExperimentType::JVMChaos => "jvmchaos",
        };

        let url = format!(
            "{}/apis/chaos-mesh.org/v1alpha1/namespaces/{}/{}es/{}",
            self.api_url, self.namespace, kind, name
        );

        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(ChaosMeshError::KubernetesError(error));
        }

        Ok(())
    }

    /// Pause an experiment
    pub async fn pause_experiment(
        &self,
        experiment_type: &ExperimentType,
        name: &str,
    ) -> Result<()> {
        self.update_experiment_annotation(
            experiment_type,
            name,
            "experiment.chaos-mesh.org/pause",
            "true",
        )
        .await
    }

    /// Resume a paused experiment
    pub async fn resume_experiment(
        &self,
        experiment_type: &ExperimentType,
        name: &str,
    ) -> Result<()> {
        self.update_experiment_annotation(
            experiment_type,
            name,
            "experiment.chaos-mesh.org/pause",
            "false",
        )
        .await
    }

    /// Update experiment annotation
    async fn update_experiment_annotation(
        &self,
        experiment_type: &ExperimentType,
        name: &str,
        annotation_key: &str,
        annotation_value: &str,
    ) -> Result<()> {
        let kind = match experiment_type {
            ExperimentType::PodChaos => "podchaos",
            ExperimentType::NetworkChaos => "networkchaos",
            ExperimentType::StressChaos => "stresschaos",
            ExperimentType::IOChaos => "iochaos",
            ExperimentType::TimeChaos => "timechaos",
            ExperimentType::KernelChaos => "kernelchaos",
            ExperimentType::DNSChaos => "dnschaos",
            ExperimentType::HTTPChaos => "httpchaos",
            ExperimentType::JVMChaos => "jvmchaos",
        };

        let url = format!(
            "{}/apis/chaos-mesh.org/v1alpha1/namespaces/{}/{}es/{}",
            self.api_url, self.namespace, kind, name
        );

        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    annotation_key: annotation_value
                }
            }
        });

        let response = self
            .client
            .patch(&url)
            .header("Content-Type", "application/merge-patch+json")
            .json(&patch)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(ChaosMeshError::KubernetesError(error));
        }

        Ok(())
    }

    /// List all experiments in namespace
    pub async fn list_experiments(
        &self,
        experiment_type: &ExperimentType,
    ) -> Result<Vec<ChaosMeshExperiment>> {
        let kind = match experiment_type {
            ExperimentType::PodChaos => "podchaos",
            ExperimentType::NetworkChaos => "networkchaos",
            ExperimentType::StressChaos => "stresschaos",
            ExperimentType::IOChaos => "iochaos",
            ExperimentType::TimeChaos => "timechaos",
            ExperimentType::KernelChaos => "kernelchaos",
            ExperimentType::DNSChaos => "dnschaos",
            ExperimentType::HTTPChaos => "httpchaos",
            ExperimentType::JVMChaos => "jvmchaos",
        };

        let url = format!(
            "{}/apis/chaos-mesh.org/v1alpha1/namespaces/{}/{}es",
            self.api_url, self.namespace, kind
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(ChaosMeshError::KubernetesError(error));
        }

        #[derive(Deserialize)]
        struct ExperimentList {
            items: Vec<ChaosMeshExperiment>,
        }

        let list: ExperimentList = response.json().await?;
        Ok(list.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_serialization() {
        let selector = PodSelector {
            namespaces: vec!["default".to_string()],
            label_selectors: Some(HashMap::from([("app".to_string(), "test".to_string())])),
            annotation_selectors: None,
            field_selectors: None,
            pod_phase_selectors: None,
        };

        let experiment = ChaosMeshExperiment {
            api_version: "chaos-mesh.org/v1alpha1".to_string(),
            kind: "PodChaos".to_string(),
            metadata: ExperimentMetadata {
                name: "test-chaos".to_string(),
                namespace: "default".to_string(),
                labels: None,
                annotations: None,
            },
            spec: ExperimentSpec {
                selector,
                mode: "one".to_string(),
                value: None,
                duration: Some("30s".to_string()),
                pod_action: Some(PodChaosAction::PodKill),
                network_action: None,
                delay: None,
                loss: None,
                stressors: None,
            },
        };

        let json = serde_json::to_string_pretty(&experiment).unwrap();
        assert!(json.contains("PodChaos"));
        assert!(json.contains("test-chaos"));
    }

    #[test]
    fn test_network_delay_config() {
        let delay = NetworkDelay {
            latency: "100ms".to_string(),
            correlation: Some("50".to_string()),
            jitter: Some("10ms".to_string()),
        };

        let json = serde_json::to_string(&delay).unwrap();
        assert!(json.contains("100ms"));
        assert!(json.contains("10ms"));
    }

    #[test]
    fn test_stress_config() {
        let stress = StressConfig {
            cpu_workers: Some(2),
            cpu_load: Some(50),
            memory_workers: Some(1),
            memory_size: Some("256MB".to_string()),
        };

        let json = serde_json::to_string(&stress).unwrap();
        assert!(json.contains("256MB"));
    }
}

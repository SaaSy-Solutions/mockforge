//! Chaos Mesh Integration
//!
//! Integrates MockForge with Chaos Mesh for Kubernetes-native chaos engineering.
//! Supports various chaos experiment types including PodChaos, NetworkChaos, StressChaos, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Chaos Mesh integration errors
#[derive(Error, Debug)]
pub enum ChaosMeshError {
    #[error("Kubernetes API error: {0}")]
    KubernetesError(String),

    #[error("Experiment not found: {0}")]
    ExperimentNotFound(String),

    #[error("Invalid experiment configuration: {0}")]
    InvalidConfig(String),

    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ChaosMeshError>;

/// Chaos Mesh experiment types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExperimentType {
    PodChaos,
    NetworkChaos,
    StressChaos,
    IOChaos,
    TimeChaos,
    KernelChaos,
    DNSChaos,
    HTTPChaos,
    JVMChaos,
}

/// Pod chaos action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PodChaosAction {
    PodKill,
    PodFailure,
    ContainerKill,
}

/// Network chaos action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkChaosAction {
    Delay,
    Loss,
    Duplicate,
    Corrupt,
    Partition,
    Bandwidth,
}

/// Selector for targeting pods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodSelector {
    pub namespaces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_selectors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_selectors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_selectors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_phase_selectors: Option<Vec<String>>,
}

/// Network delay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDelay {
    pub latency: String, // e.g., "100ms", "1s"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation: Option<String>, // "0" to "100"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter: Option<String>, // e.g., "10ms"
}

/// Network loss configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLoss {
    pub loss: String, // "0" to "100" (percentage)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation: Option<String>, // "0" to "100"
}

/// Stress test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_load: Option<u32>, // Percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_size: Option<String>, // e.g., "256MB", "1GB"
}

/// Experiment duration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Duration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>, // e.g., "30s", "5m"
}

/// Chaos experiment specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSpec {
    pub selector: PodSelector,
    pub mode: String, // "one", "all", "fixed", "fixed-percent", "random-max-percent"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>, // For "fixed" or "fixed-percent" mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,

    // Action-specific configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_action: Option<PodChaosAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_action: Option<NetworkChaosAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<NetworkDelay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss: Option<NetworkLoss>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stressors: Option<StressConfig>,
}

/// Chaos Mesh experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosMeshExperiment {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: ExperimentMetadata,
    pub spec: ExperimentSpec,
}

/// Experiment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentMetadata {
    pub name: String,
    pub namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,
}

/// Experiment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStatus {
    pub phase: String, // "Running", "Finished", "Paused", "Failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<StatusCondition>>,
}

/// Status condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCondition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

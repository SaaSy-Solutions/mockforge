//! Load testing scenario definitions

use serde::{Deserialize, Serialize};

/// Load testing scenarios
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LoadScenario {
    /// Constant load - maintains steady number of VUs
    Constant,
    /// Ramp-up - gradually increases load
    RampUp,
    /// Spike - sudden increase in load
    Spike,
    /// Stress - continuously increasing load to find breaking point
    Stress,
    /// Soak - sustained load over extended period
    Soak,
}

impl LoadScenario {
    /// Parse scenario from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "constant" => Ok(Self::Constant),
            "ramp-up" | "ramp_up" | "rampup" => Ok(Self::RampUp),
            "spike" => Ok(Self::Spike),
            "stress" => Ok(Self::Stress),
            "soak" => Ok(Self::Soak),
            _ => Err(format!("Unknown scenario: {}", s)),
        }
    }

    /// Generate k6 stages configuration for this scenario
    pub fn generate_stages(&self, duration_secs: u64, max_vus: u32) -> Vec<Stage> {
        match self {
            Self::Constant => {
                vec![Stage {
                    duration: format!("{}s", duration_secs),
                    target: max_vus,
                }]
            }
            Self::RampUp => {
                let ramp_duration = duration_secs / 3;
                let sustain_duration = duration_secs / 3;
                let ramp_down_duration = duration_secs - ramp_duration - sustain_duration;

                vec![
                    Stage {
                        duration: format!("{}s", ramp_duration / 2),
                        target: max_vus / 4,
                    },
                    Stage {
                        duration: format!("{}s", ramp_duration / 2),
                        target: max_vus / 2,
                    },
                    Stage {
                        duration: format!("{}s", sustain_duration),
                        target: max_vus,
                    },
                    Stage {
                        duration: format!("{}s", ramp_down_duration),
                        target: 0,
                    },
                ]
            }
            Self::Spike => {
                let baseline_duration = duration_secs / 5;
                let spike_duration = duration_secs / 10;
                let recovery_duration = duration_secs - (baseline_duration * 2) - spike_duration;

                vec![
                    Stage {
                        duration: format!("{}s", baseline_duration),
                        target: max_vus / 10,
                    },
                    Stage {
                        duration: format!("{}s", spike_duration),
                        target: max_vus,
                    },
                    Stage {
                        duration: format!("{}s", recovery_duration),
                        target: max_vus / 10,
                    },
                    Stage {
                        duration: format!("{}s", baseline_duration),
                        target: 0,
                    },
                ]
            }
            Self::Stress => {
                let step_duration = duration_secs / 6;
                let step_vus = max_vus / 5;

                vec![
                    Stage {
                        duration: format!("{}s", step_duration),
                        target: step_vus,
                    },
                    Stage {
                        duration: format!("{}s", step_duration),
                        target: step_vus * 2,
                    },
                    Stage {
                        duration: format!("{}s", step_duration),
                        target: step_vus * 3,
                    },
                    Stage {
                        duration: format!("{}s", step_duration),
                        target: step_vus * 4,
                    },
                    Stage {
                        duration: format!("{}s", step_duration),
                        target: max_vus,
                    },
                    Stage {
                        duration: format!("{}s", step_duration),
                        target: 0,
                    },
                ]
            }
            Self::Soak => {
                // Minimal ramp-up, long sustained load
                let ramp_duration = duration_secs / 20;
                let sustain_duration = duration_secs - (ramp_duration * 2);

                vec![
                    Stage {
                        duration: format!("{}s", ramp_duration),
                        target: max_vus,
                    },
                    Stage {
                        duration: format!("{}s", sustain_duration),
                        target: max_vus,
                    },
                    Stage {
                        duration: format!("{}s", ramp_duration),
                        target: 0,
                    },
                ]
            }
        }
    }

    /// Get description of this scenario
    pub fn description(&self) -> &str {
        match self {
            Self::Constant => "Constant load with steady VUs",
            Self::RampUp => "Gradually increase load to target VUs",
            Self::Spike => "Sudden spike in load to test system resilience",
            Self::Stress => "Continuously increase load to find breaking point",
            Self::Soak => "Sustained load over extended period",
        }
    }
}

/// A k6 load stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub duration: String,
    pub target: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_from_str() {
        assert_eq!(LoadScenario::from_str("constant").unwrap(), LoadScenario::Constant);
        assert_eq!(LoadScenario::from_str("ramp-up").unwrap(), LoadScenario::RampUp);
        assert_eq!(LoadScenario::from_str("spike").unwrap(), LoadScenario::Spike);
        assert!(LoadScenario::from_str("unknown").is_err());
    }

    #[test]
    fn test_constant_stages() {
        let scenario = LoadScenario::Constant;
        let stages = scenario.generate_stages(60, 10);
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].target, 10);
    }

    #[test]
    fn test_rampup_stages() {
        let scenario = LoadScenario::RampUp;
        let stages = scenario.generate_stages(120, 100);
        assert!(stages.len() >= 3);
        assert_eq!(stages.last().unwrap().target, 0);
    }

    #[test]
    fn test_spike_stages() {
        let scenario = LoadScenario::Spike;
        let stages = scenario.generate_stages(100, 100);
        assert!(stages.len() >= 3);
        // Check that there's a spike
        let max_stage = stages.iter().max_by_key(|s| s.target).unwrap();
        assert_eq!(max_stage.target, 100);
    }
}

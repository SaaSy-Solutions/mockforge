//! Pre-configured network condition profiles for easy simulation of various network scenarios
//!
//! This module provides user-friendly presets that package latency, bandwidth, and
//! packet loss settings into common network scenarios like "3G", "Slow 2G", "Satellite", etc.

use crate::latency::{LatencyDistribution, LatencyProfile};
use crate::traffic_shaping::{BandwidthConfig, BurstLossConfig, TrafficShapingConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pre-configured network condition profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProfile {
    /// Profile name
    pub name: String,
    /// Profile description
    pub description: String,
    /// Latency configuration
    pub latency: LatencyProfile,
    /// Traffic shaping configuration
    pub traffic_shaping: TrafficShapingConfig,
}

impl NetworkProfile {
    /// Create a custom network profile
    pub fn custom(
        name: String,
        description: String,
        latency: LatencyProfile,
        traffic_shaping: TrafficShapingConfig,
    ) -> Self {
        Self {
            name,
            description,
            latency,
            traffic_shaping,
        }
    }

    /// Apply this profile to get its configurations
    pub fn apply(&self) -> (LatencyProfile, TrafficShapingConfig) {
        (self.latency.clone(), self.traffic_shaping.clone())
    }
}

/// Network profile catalog with built-in presets
#[derive(Debug, Clone)]
pub struct NetworkProfileCatalog {
    profiles: HashMap<String, NetworkProfile>,
}

impl Default for NetworkProfileCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkProfileCatalog {
    /// Create a new catalog with built-in profiles
    pub fn new() -> Self {
        let mut catalog = Self {
            profiles: HashMap::new(),
        };

        // Add built-in profiles
        catalog.add_builtin_profiles();
        catalog
    }

    /// Add all built-in network profiles
    fn add_builtin_profiles(&mut self) {
        // Perfect network (no degradation)
        self.add_profile(Self::perfect_network());

        // Mobile networks
        self.add_profile(Self::mobile_5g());
        self.add_profile(Self::mobile_4g());
        self.add_profile(Self::mobile_3g());
        self.add_profile(Self::mobile_2g());
        self.add_profile(Self::mobile_edge());

        // Satellite connections
        self.add_profile(Self::satellite_leo());  // Low Earth Orbit (like Starlink)
        self.add_profile(Self::satellite_geo());  // Geostationary (traditional satellite)

        // Impaired networks
        self.add_profile(Self::congested_network());
        self.add_profile(Self::lossy_network());
        self.add_profile(Self::high_latency());

        // Edge cases
        self.add_profile(Self::intermittent_connection());
        self.add_profile(Self::extremely_poor());
    }

    /// Add a custom profile to the catalog
    pub fn add_profile(&mut self, profile: NetworkProfile) {
        self.profiles.insert(profile.name.clone(), profile);
    }

    /// Get a profile by name
    pub fn get(&self, name: &str) -> Option<&NetworkProfile> {
        self.profiles.get(name)
    }

    /// Get all available profile names
    pub fn list_profiles(&self) -> Vec<String> {
        let mut names: Vec<String> = self.profiles.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get all profiles with descriptions
    pub fn list_profiles_with_description(&self) -> Vec<(String, String)> {
        let mut profiles: Vec<_> = self
            .profiles
            .values()
            .map(|p| (p.name.clone(), p.description.clone()))
            .collect();
        profiles.sort_by(|a, b| a.0.cmp(&b.0));
        profiles
    }

    // ========================================================================
    // Built-in Profile Definitions
    // ========================================================================

    /// Perfect network (no degradation)
    fn perfect_network() -> NetworkProfile {
        NetworkProfile {
            name: "perfect".to_string(),
            description: "Perfect network with no degradation".to_string(),
            latency: LatencyProfile::new(0, 0),
            traffic_shaping: TrafficShapingConfig::default(),
        }
    }

    /// 5G mobile network
    fn mobile_5g() -> NetworkProfile {
        NetworkProfile {
            name: "5g".to_string(),
            description: "5G mobile network (10-30ms latency, ~100 Mbps)".to_string(),
            latency: LatencyProfile::with_normal_distribution(20, 5.0)
                .with_min_ms(10)
                .with_max_ms(30),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    100_000_000 / 8, // 100 Mbps in bytes/sec
                    10_000_000,      // 10MB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: false,
                    ..Default::default()
                },
            },
        }
    }

    /// 4G/LTE mobile network
    fn mobile_4g() -> NetworkProfile {
        NetworkProfile {
            name: "4g".to_string(),
            description: "4G/LTE mobile network (30-60ms latency, ~20 Mbps)".to_string(),
            latency: LatencyProfile::with_normal_distribution(45, 10.0)
                .with_min_ms(30)
                .with_max_ms(70),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    20_000_000 / 8, // 20 Mbps in bytes/sec
                    2_500_000,      // 2.5MB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.05,  // 5% chance of burst
                    burst_duration_ms: 2000,  // 2 second bursts
                    loss_rate_during_burst: 0.1, // 10% loss during burst
                    recovery_time_ms: 30000,  // 30 second recovery
                    ..Default::default()
                },
            },
        }
    }

    /// 3G mobile network
    fn mobile_3g() -> NetworkProfile {
        NetworkProfile {
            name: "3g".to_string(),
            description: "3G mobile network (100-200ms latency, ~1 Mbps)".to_string(),
            latency: LatencyProfile::with_normal_distribution(150, 30.0)
                .with_min_ms(100)
                .with_max_ms(250),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    1_000_000 / 8, // 1 Mbps in bytes/sec
                    125_000,       // 125KB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.1,
                    burst_duration_ms: 3000,
                    loss_rate_during_burst: 0.15,
                    recovery_time_ms: 20000,
                    ..Default::default()
                },
            },
        }
    }

    /// 2G mobile network (EDGE)
    fn mobile_2g() -> NetworkProfile {
        NetworkProfile {
            name: "2g".to_string(),
            description: "2G/EDGE mobile network (300-500ms latency, ~250 Kbps)".to_string(),
            latency: LatencyProfile::with_normal_distribution(400, 80.0)
                .with_min_ms(300)
                .with_max_ms(600),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    250_000 / 8, // 250 Kbps in bytes/sec
                    31_250,      // 31KB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.15,
                    burst_duration_ms: 5000,
                    loss_rate_during_burst: 0.2,
                    recovery_time_ms: 15000,
                    ..Default::default()
                },
            },
        }
    }

    /// EDGE mobile network (worst case)
    fn mobile_edge() -> NetworkProfile {
        NetworkProfile {
            name: "edge".to_string(),
            description: "EDGE mobile network (500-800ms latency, ~100 Kbps)".to_string(),
            latency: LatencyProfile::with_normal_distribution(650, 120.0)
                .with_min_ms(500)
                .with_max_ms(1000),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    100_000 / 8, // 100 Kbps in bytes/sec
                    12_500,      // 12.5KB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.2,
                    burst_duration_ms: 8000,
                    loss_rate_during_burst: 0.25,
                    recovery_time_ms: 10000,
                    ..Default::default()
                },
            },
        }
    }

    /// Low Earth Orbit satellite (like Starlink)
    fn satellite_leo() -> NetworkProfile {
        NetworkProfile {
            name: "satellite_leo".to_string(),
            description: "LEO satellite (20-40ms latency, ~100 Mbps, variable)".to_string(),
            latency: LatencyProfile::with_pareto_distribution(30, 2.5)
                .with_min_ms(20)
                .with_max_ms(150), // Occasional higher latency
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    100_000_000 / 8, // 100 Mbps in bytes/sec
                    10_000_000,      // 10MB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.08,
                    burst_duration_ms: 3000,
                    loss_rate_during_burst: 0.15,
                    recovery_time_ms: 25000,
                    ..Default::default()
                },
            },
        }
    }

    /// Geostationary satellite (traditional)
    fn satellite_geo() -> NetworkProfile {
        NetworkProfile {
            name: "satellite_geo".to_string(),
            description: "GEO satellite (550-750ms latency, ~15 Mbps)".to_string(),
            latency: LatencyProfile::with_normal_distribution(650, 80.0)
                .with_min_ms(550)
                .with_max_ms(850),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    15_000_000 / 8, // 15 Mbps in bytes/sec
                    1_875_000,      // 1.875MB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.1,
                    burst_duration_ms: 5000,
                    loss_rate_during_burst: 0.2,
                    recovery_time_ms: 20000,
                    ..Default::default()
                },
            },
        }
    }

    /// Congested network with high variable latency
    fn congested_network() -> NetworkProfile {
        NetworkProfile {
            name: "congested".to_string(),
            description: "Congested network (100-500ms latency, ~2 Mbps, high jitter)".to_string(),
            latency: LatencyProfile::with_pareto_distribution(150, 1.8)
                .with_min_ms(100)
                .with_max_ms(800),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    2_000_000 / 8, // 2 Mbps in bytes/sec
                    250_000,       // 250KB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.12,
                    burst_duration_ms: 4000,
                    loss_rate_during_burst: 0.2,
                    recovery_time_ms: 18000,
                    ..Default::default()
                },
            },
        }
    }

    /// Network with significant packet loss
    fn lossy_network() -> NetworkProfile {
        NetworkProfile {
            name: "lossy".to_string(),
            description: "Lossy network (50-100ms latency, 20% packet loss)".to_string(),
            latency: LatencyProfile::with_normal_distribution(75, 15.0)
                .with_min_ms(50)
                .with_max_ms(120),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    10_000_000 / 8, // 10 Mbps in bytes/sec
                    1_250_000,      // 1.25MB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.3,  // High probability of loss bursts
                    burst_duration_ms: 2000,
                    loss_rate_during_burst: 0.5, // 50% loss during burst
                    recovery_time_ms: 8000,
                    ..Default::default()
                },
            },
        }
    }

    /// High latency network
    fn high_latency() -> NetworkProfile {
        NetworkProfile {
            name: "high_latency".to_string(),
            description: "High latency network (500-1000ms latency, normal bandwidth)".to_string(),
            latency: LatencyProfile::with_normal_distribution(750, 150.0)
                .with_min_ms(500)
                .with_max_ms(1200),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    10_000_000 / 8, // 10 Mbps in bytes/sec
                    1_250_000,      // 1.25MB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: false,
                    ..Default::default()
                },
            },
        }
    }

    /// Intermittent connection (frequent disconnections)
    fn intermittent_connection() -> NetworkProfile {
        NetworkProfile {
            name: "intermittent".to_string(),
            description: "Intermittent connection (100-300ms latency, frequent drops)".to_string(),
            latency: LatencyProfile::with_normal_distribution(200, 50.0)
                .with_min_ms(100)
                .with_max_ms(400),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    5_000_000 / 8, // 5 Mbps in bytes/sec
                    625_000,       // 625KB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.4,  // Very frequent drops
                    burst_duration_ms: 5000, // Long outages
                    loss_rate_during_burst: 0.8, // 80% loss during burst
                    recovery_time_ms: 10000,
                    ..Default::default()
                },
            },
        }
    }

    /// Extremely poor network conditions
    fn extremely_poor() -> NetworkProfile {
        NetworkProfile {
            name: "extremely_poor".to_string(),
            description: "Extremely poor network (1000ms+ latency, <50 Kbps, high loss)".to_string(),
            latency: LatencyProfile::with_pareto_distribution(1000, 1.5)
                .with_min_ms(800)
                .with_max_ms(3000),
            traffic_shaping: TrafficShapingConfig {
                bandwidth: BandwidthConfig::new(
                    50_000 / 8, // 50 Kbps in bytes/sec
                    6_250,      // 6.25KB burst
                ),
                burst_loss: BurstLossConfig {
                    enabled: true,
                    burst_probability: 0.5,
                    burst_duration_ms: 10000,
                    loss_rate_during_burst: 0.7,
                    recovery_time_ms: 5000,
                    ..Default::default()
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_profile_creation() {
        let profile = NetworkProfile::custom(
            "test".to_string(),
            "Test profile".to_string(),
            LatencyProfile::new(100, 20),
            TrafficShapingConfig::default(),
        );

        assert_eq!(profile.name, "test");
        assert_eq!(profile.description, "Test profile");
    }

    #[test]
    fn test_catalog_has_builtin_profiles() {
        let catalog = NetworkProfileCatalog::new();
        let profiles = catalog.list_profiles();

        // Check that we have all expected profiles
        assert!(profiles.contains(&"perfect".to_string()));
        assert!(profiles.contains(&"5g".to_string()));
        assert!(profiles.contains(&"4g".to_string()));
        assert!(profiles.contains(&"3g".to_string()));
        assert!(profiles.contains(&"2g".to_string()));
        assert!(profiles.contains(&"edge".to_string()));
        assert!(profiles.contains(&"satellite_leo".to_string()));
        assert!(profiles.contains(&"satellite_geo".to_string()));
        assert!(profiles.contains(&"congested".to_string()));
        assert!(profiles.contains(&"lossy".to_string()));
        assert!(profiles.contains(&"high_latency".to_string()));
        assert!(profiles.contains(&"intermittent".to_string()));
        assert!(profiles.contains(&"extremely_poor".to_string()));

        assert!(profiles.len() >= 13);
    }

    #[test]
    fn test_get_profile() {
        let catalog = NetworkProfileCatalog::new();

        let profile_3g = catalog.get("3g");
        assert!(profile_3g.is_some());
        assert_eq!(profile_3g.unwrap().name, "3g");

        let profile_nonexistent = catalog.get("nonexistent");
        assert!(profile_nonexistent.is_none());
    }

    #[test]
    fn test_apply_profile() {
        let catalog = NetworkProfileCatalog::new();
        let profile = catalog.get("4g").unwrap();

        let (latency, traffic_shaping) = profile.apply();

        // 4G should have latency around 30-70ms
        assert!(latency.base_ms >= 30);
        assert!(latency.base_ms <= 70);

        // 4G should have bandwidth enabled
        assert!(traffic_shaping.bandwidth.enabled);
    }

    #[test]
    fn test_list_profiles_with_description() {
        let catalog = NetworkProfileCatalog::new();
        let profiles = catalog.list_profiles_with_description();

        // Check that we get tuples with names and descriptions
        assert!(!profiles.is_empty());

        for (name, desc) in &profiles {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn test_custom_profile_addition() {
        let mut catalog = NetworkProfileCatalog::new();

        let custom = NetworkProfile::custom(
            "custom_test".to_string(),
            "Custom test profile".to_string(),
            LatencyProfile::new(50, 10),
            TrafficShapingConfig::default(),
        );

        catalog.add_profile(custom);

        let profiles = catalog.list_profiles();
        assert!(profiles.contains(&"custom_test".to_string()));
    }

    #[test]
    fn test_profile_characteristics() {
        let catalog = NetworkProfileCatalog::new();

        // Test 5G has lower latency than 3G
        let profile_5g = catalog.get("5g").unwrap();
        let profile_3g = catalog.get("3g").unwrap();
        assert!(profile_5g.latency.base_ms < profile_3g.latency.base_ms);

        // Test satellite_geo has high latency
        let profile_sat = catalog.get("satellite_geo").unwrap();
        assert!(profile_sat.latency.base_ms >= 550);

        // Test lossy network has burst loss enabled
        let profile_lossy = catalog.get("lossy").unwrap();
        assert!(profile_lossy.traffic_shaping.burst_loss.enabled);
    }
}

//! Time-based scheduling for Reality Continuum
//!
//! Provides time-based progression of blend ratios, allowing gradual transition
//! from mock to real data over a simulated timeline.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Transition curve type for blend ratio progression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TransitionCurve {
    /// Linear progression (constant rate)
    Linear,
    /// Exponential progression (slow start, fast end)
    Exponential,
    /// Sigmoid progression (slow start and end, fast middle)
    Sigmoid,
}

impl Default for TransitionCurve {
    fn default() -> Self {
        TransitionCurve::Linear
    }
}

/// Time schedule for blend ratio progression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TimeSchedule {
    /// Start time for the transition
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub start_time: DateTime<Utc>,
    /// End time for the transition
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub end_time: DateTime<Utc>,
    /// Initial blend ratio at start time
    pub start_ratio: f64,
    /// Final blend ratio at end time
    pub end_ratio: f64,
    /// Transition curve type
    #[serde(default)]
    pub curve: TransitionCurve,
}

impl TimeSchedule {
    /// Create a new time schedule
    pub fn new(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        start_ratio: f64,
        end_ratio: f64,
    ) -> Self {
        Self {
            start_time,
            end_time,
            start_ratio: start_ratio.clamp(0.0, 1.0),
            end_ratio: end_ratio.clamp(0.0, 1.0),
            curve: TransitionCurve::Linear,
        }
    }

    /// Create a new time schedule with a specific curve
    pub fn with_curve(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        start_ratio: f64,
        end_ratio: f64,
        curve: TransitionCurve,
    ) -> Self {
        Self {
            start_time,
            end_time,
            start_ratio: start_ratio.clamp(0.0, 1.0),
            end_ratio: end_ratio.clamp(0.0, 1.0),
            curve,
        }
    }

    /// Calculate the blend ratio at a specific time
    ///
    /// Returns the blend ratio based on the current time relative to the schedule.
    /// If the time is before start_time, returns start_ratio.
    /// If the time is after end_time, returns end_ratio.
    /// Otherwise, calculates based on the transition curve.
    pub fn calculate_ratio(&self, current_time: DateTime<Utc>) -> f64 {
        // Before start time, return start ratio
        if current_time < self.start_time {
            return self.start_ratio;
        }

        // After end time, return end ratio
        if current_time > self.end_time {
            return self.end_ratio;
        }

        // Calculate progress (0.0 to 1.0)
        let total_duration = self.end_time - self.start_time;
        let elapsed = current_time - self.start_time;

        let progress = if total_duration.num_seconds() == 0 {
            1.0
        } else {
            elapsed.num_seconds() as f64 / total_duration.num_seconds() as f64
        };

        // Apply transition curve
        let curved_progress = match self.curve {
            TransitionCurve::Linear => progress,
            TransitionCurve::Exponential => {
                // Exponential: e^(k * progress) - 1 / (e^k - 1)
                // Using k=2 for moderate exponential curve
                let k = 2.0;
                (progress * k).exp() - 1.0 / (k.exp() - 1.0)
            }
            TransitionCurve::Sigmoid => {
                // Sigmoid: 1 / (1 + e^(-k * (progress - 0.5)))
                // Using k=10 for smooth sigmoid curve
                let k = 10.0;
                1.0 / (1.0 + (-k * (progress - 0.5)).exp())
            }
        };

        // Interpolate between start and end ratio
        self.start_ratio + (self.end_ratio - self.start_ratio) * curved_progress
    }

    /// Check if the schedule is active at the given time
    pub fn is_active(&self, current_time: DateTime<Utc>) -> bool {
        current_time >= self.start_time && current_time <= self.end_time
    }

    /// Get the duration of the transition
    pub fn duration(&self) -> Duration {
        self.end_time - self.start_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_schedule_before_start() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::new(start, end, 0.0, 1.0);

        let before_start = start - Duration::days(1);
        assert_eq!(schedule.calculate_ratio(before_start), 0.0);
    }

    #[test]
    fn test_time_schedule_after_end() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::new(start, end, 0.0, 1.0);

        let after_end = end + Duration::days(1);
        assert_eq!(schedule.calculate_ratio(after_end), 1.0);
    }

    #[test]
    fn test_time_schedule_linear_midpoint() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::with_curve(start, end, 0.0, 1.0, TransitionCurve::Linear);

        let midpoint = start + Duration::days(15);
        let ratio = schedule.calculate_ratio(midpoint);
        // Should be approximately 0.5 for linear curve at midpoint
        assert!((ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_time_schedule_is_active() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::new(start, end, 0.0, 1.0);

        assert!(!schedule.is_active(start - Duration::days(1)));
        assert!(schedule.is_active(start + Duration::days(15)));
        assert!(!schedule.is_active(end + Duration::days(1)));
    }

    #[test]
    fn test_time_schedule_duration() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::new(start, end, 0.0, 1.0);

        assert_eq!(schedule.duration().num_days(), 30);
    }

    #[test]
    fn test_exponential_curve() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::with_curve(start, end, 0.0, 1.0, TransitionCurve::Exponential);

        let midpoint = start + Duration::days(15);
        let ratio = schedule.calculate_ratio(midpoint);
        // Exponential should be less than linear at midpoint (slow start)
        assert!(ratio < 0.5);
    }

    #[test]
    fn test_sigmoid_curve() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::with_curve(start, end, 0.0, 1.0, TransitionCurve::Sigmoid);

        let midpoint = start + Duration::days(15);
        let ratio = schedule.calculate_ratio(midpoint);
        // Sigmoid should be close to 0.5 at midpoint
        assert!((ratio - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_ratio_clamping() {
        let start = Utc::now();
        let end = start + Duration::days(30);
        let schedule = TimeSchedule::new(start, end, -0.5, 1.5);

        // Should be clamped to [0.0, 1.0]
        assert_eq!(schedule.start_ratio, 0.0);
        assert_eq!(schedule.end_ratio, 1.0);
    }
}

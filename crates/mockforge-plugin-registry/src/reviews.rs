//! Plugin ratings and reviews system

use crate::{RegistryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Review ID
    pub id: String,

    /// Plugin name
    pub plugin_name: String,

    /// Plugin version reviewed
    pub version: String,

    /// User information
    pub user: UserInfo,

    /// Rating (1-5)
    pub rating: u8,

    /// Review text
    pub comment: String,

    /// Review title
    pub title: Option<String>,

    /// Helpful votes count
    pub helpful_count: u32,

    /// Unhelpful votes count
    pub unhelpful_count: u32,

    /// Verified purchase
    pub verified: bool,

    /// Created timestamp
    pub created_at: String,

    /// Updated timestamp
    pub updated_at: String,

    /// Response from author
    pub author_response: Option<AuthorResponse>,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

/// Author response to review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorResponse {
    pub text: String,
    pub created_at: String,
}

/// Review submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitReviewRequest {
    pub plugin_name: String,
    pub version: String,
    pub rating: u8,
    pub title: Option<String>,
    pub comment: String,
}

impl SubmitReviewRequest {
    /// Validate review request
    pub fn validate(&self) -> Result<()> {
        if self.rating < 1 || self.rating > 5 {
            return Err(RegistryError::InvalidManifest(
                "Rating must be between 1 and 5".to_string(),
            ));
        }

        if self.comment.len() < 10 {
            return Err(RegistryError::InvalidManifest(
                "Review comment must be at least 10 characters".to_string(),
            ));
        }

        if self.comment.len() > 5000 {
            return Err(RegistryError::InvalidManifest(
                "Review comment must be less than 5000 characters".to_string(),
            ));
        }

        if let Some(title) = &self.title {
            if title.len() > 100 {
                return Err(RegistryError::InvalidManifest(
                    "Review title must be less than 100 characters".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Review update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReviewRequest {
    pub rating: Option<u8>,
    pub title: Option<String>,
    pub comment: Option<String>,
}

/// Vote on review helpfulness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    pub helpful: bool,
}

/// Review statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewStats {
    pub total_reviews: u32,
    pub average_rating: f64,
    pub rating_distribution: HashMap<u8, u32>,
}

impl ReviewStats {
    /// Create empty stats
    pub fn empty() -> Self {
        Self {
            total_reviews: 0,
            average_rating: 0.0,
            rating_distribution: HashMap::new(),
        }
    }

    /// Calculate from reviews
    pub fn from_reviews(reviews: &[Review]) -> Self {
        let mut distribution = HashMap::new();
        let mut total_rating = 0u32;

        for review in reviews {
            *distribution.entry(review.rating).or_insert(0) += 1;
            total_rating += review.rating as u32;
        }

        let average_rating = if reviews.is_empty() {
            0.0
        } else {
            total_rating as f64 / reviews.len() as f64
        };

        Self {
            total_reviews: reviews.len() as u32,
            average_rating,
            rating_distribution: distribution,
        }
    }
}

/// Query reviews for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewQuery {
    pub plugin_name: String,
    pub version: Option<String>,
    pub min_rating: Option<u8>,
    pub sort_by: ReviewSortOrder,
    pub page: usize,
    pub per_page: usize,
}

impl Default for ReviewQuery {
    fn default() -> Self {
        Self {
            plugin_name: String::new(),
            version: None,
            min_rating: None,
            sort_by: ReviewSortOrder::MostHelpful,
            page: 0,
            per_page: 20,
        }
    }
}

/// Review sort order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSortOrder {
    MostHelpful,
    MostRecent,
    HighestRated,
    LowestRated,
}

/// Review search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResults {
    pub reviews: Vec<Review>,
    pub stats: ReviewStats,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// Review moderation action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationAction {
    Approve,
    Reject,
    Flag,
    Delete,
}

/// Moderation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationRequest {
    pub action: ModerationAction,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_validation() {
        let valid = SubmitReviewRequest {
            plugin_name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            rating: 4,
            title: Some("Great plugin!".to_string()),
            comment: "This plugin works great for my use case.".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid_rating = SubmitReviewRequest {
            rating: 6,
            ..valid.clone()
        };
        assert!(invalid_rating.validate().is_err());

        let short_comment = SubmitReviewRequest {
            comment: "Too short".to_string(),
            ..valid.clone()
        };
        assert!(short_comment.validate().is_err());
    }

    #[test]
    fn test_review_stats_calculation() {
        let reviews = vec![
            Review {
                id: "1".to_string(),
                plugin_name: "test".to_string(),
                version: "1.0.0".to_string(),
                user: UserInfo {
                    id: "u1".to_string(),
                    name: "User 1".to_string(),
                    avatar_url: None,
                },
                rating: 5,
                comment: "Excellent!".to_string(),
                title: None,
                helpful_count: 10,
                unhelpful_count: 0,
                verified: true,
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
                author_response: None,
            },
            Review {
                id: "2".to_string(),
                plugin_name: "test".to_string(),
                version: "1.0.0".to_string(),
                user: UserInfo {
                    id: "u2".to_string(),
                    name: "User 2".to_string(),
                    avatar_url: None,
                },
                rating: 3,
                comment: "It's okay".to_string(),
                title: None,
                helpful_count: 5,
                unhelpful_count: 2,
                verified: false,
                created_at: "2025-01-02T00:00:00Z".to_string(),
                updated_at: "2025-01-02T00:00:00Z".to_string(),
                author_response: None,
            },
        ];

        let stats = ReviewStats::from_reviews(&reviews);
        assert_eq!(stats.total_reviews, 2);
        assert_eq!(stats.average_rating, 4.0);
        assert_eq!(stats.rating_distribution.get(&5), Some(&1));
        assert_eq!(stats.rating_distribution.get(&3), Some(&1));
    }
}

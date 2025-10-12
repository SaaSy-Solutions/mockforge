//! Template Marketplace for orchestration templates
//!
//! Provides a marketplace for sharing and discovering chaos orchestration templates
//! with ratings, categories, and version management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Orchestration template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub author_email: String,
    pub version: String,
    pub category: TemplateCategory,
    pub tags: Vec<String>,
    pub content: serde_json::Value,
    pub readme: String,
    pub example_usage: Option<String>,
    pub requirements: Vec<String>,
    pub compatibility: CompatibilityInfo,
    pub stats: TemplateStats,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published: bool,
}

/// Template category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateCategory {
    NetworkChaos,
    ServiceFailure,
    LoadTesting,
    ResilienceTesting,
    SecurityTesting,
    DataCorruption,
    MultiProtocol,
    CustomScenario,
}

/// Compatibility information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    pub min_version: String,
    pub max_version: Option<String>,
    pub required_features: Vec<String>,
    pub protocols: Vec<String>,
}

/// Template statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStats {
    pub downloads: u64,
    pub stars: u64,
    pub forks: u64,
    pub rating: f64,
    pub rating_count: u64,
}

/// Template review/rating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateReview {
    pub id: String,
    pub template_id: String,
    pub user_id: String,
    pub user_name: String,
    pub rating: u8,
    pub comment: String,
    pub created_at: DateTime<Utc>,
    pub helpful_count: u64,
}

/// Search filters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateSearchFilters {
    pub category: Option<TemplateCategory>,
    pub tags: Vec<String>,
    pub min_rating: Option<f64>,
    pub author: Option<String>,
    pub query: Option<String>,
    pub sort_by: TemplateSortBy,
    pub limit: usize,
    pub offset: usize,
}

/// Sort options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSortBy {
    Popular,
    Newest,
    TopRated,
    MostDownloaded,
    RecentlyUpdated,
}

impl Default for TemplateSortBy {
    fn default() -> Self {
        Self::Popular
    }
}

/// Template marketplace
pub struct TemplateMarketplace {
    templates: HashMap<String, OrchestrationTemplate>,
    reviews: HashMap<String, Vec<TemplateReview>>,
}

impl TemplateMarketplace {
    /// Create a new marketplace
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            reviews: HashMap::new(),
        }
    }

    /// Publish a template
    pub fn publish_template(&mut self, template: OrchestrationTemplate) -> Result<(), String> {
        if template.id.is_empty() {
            return Err("Template ID cannot be empty".to_string());
        }

        if template.name.is_empty() {
            return Err("Template name cannot be empty".to_string());
        }

        if template.version.is_empty() {
            return Err("Template version cannot be empty".to_string());
        }

        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Get a template by ID
    pub fn get_template(&self, template_id: &str) -> Option<&OrchestrationTemplate> {
        self.templates.get(template_id)
    }

    /// Search templates
    pub fn search_templates(&self, filters: TemplateSearchFilters) -> Vec<OrchestrationTemplate> {
        let mut results: Vec<_> = self
            .templates
            .values()
            .filter(|t| t.published)
            .filter(|t| {
                // Category filter
                if let Some(ref category) = filters.category {
                    if &t.category != category {
                        return false;
                    }
                }

                // Tags filter
                if !filters.tags.is_empty() && !filters.tags.iter().any(|tag| t.tags.contains(tag))
                {
                    return false;
                }

                // Min rating filter
                if let Some(min_rating) = filters.min_rating {
                    if t.stats.rating < min_rating {
                        return false;
                    }
                }

                // Author filter
                if let Some(ref author) = filters.author {
                    if !t.author.to_lowercase().contains(&author.to_lowercase()) {
                        return false;
                    }
                }

                // Query filter (search in name and description)
                if let Some(ref query) = filters.query {
                    let query_lower = query.to_lowercase();
                    if !t.name.to_lowercase().contains(&query_lower)
                        && !t.description.to_lowercase().contains(&query_lower)
                    {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort results
        match filters.sort_by {
            TemplateSortBy::Popular => {
                results.sort_by(|a, b| {
                    let score_a = a.stats.downloads + a.stats.stars * 2;
                    let score_b = b.stats.downloads + b.stats.stars * 2;
                    score_b.cmp(&score_a)
                });
            }
            TemplateSortBy::Newest => {
                results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
            TemplateSortBy::TopRated => {
                results.sort_by(|a, b| {
                    b.stats.rating.partial_cmp(&a.stats.rating).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            TemplateSortBy::MostDownloaded => {
                results.sort_by(|a, b| b.stats.downloads.cmp(&a.stats.downloads));
            }
            TemplateSortBy::RecentlyUpdated => {
                results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
        }

        // Apply pagination
        results.into_iter().skip(filters.offset).take(filters.limit).collect()
    }

    /// Download a template
    pub fn download_template(
        &mut self,
        template_id: &str,
    ) -> Result<OrchestrationTemplate, String> {
        if let Some(template) = self.templates.get_mut(template_id) {
            template.stats.downloads += 1;
            Ok(template.clone())
        } else {
            Err(format!("Template '{}' not found", template_id))
        }
    }

    /// Star a template
    pub fn star_template(&mut self, template_id: &str) -> Result<(), String> {
        if let Some(template) = self.templates.get_mut(template_id) {
            template.stats.stars += 1;
            Ok(())
        } else {
            Err(format!("Template '{}' not found", template_id))
        }
    }

    /// Unstar a template
    pub fn unstar_template(&mut self, template_id: &str) -> Result<(), String> {
        if let Some(template) = self.templates.get_mut(template_id) {
            if template.stats.stars > 0 {
                template.stats.stars -= 1;
            }
            Ok(())
        } else {
            Err(format!("Template '{}' not found", template_id))
        }
    }

    /// Add a review
    pub fn add_review(&mut self, review: TemplateReview) -> Result<(), String> {
        // Validate rating
        if review.rating > 5 {
            return Err("Rating must be between 1 and 5".to_string());
        }

        // Check if template exists
        if !self.templates.contains_key(&review.template_id) {
            return Err(format!("Template '{}' not found", review.template_id));
        }

        // Add review
        self.reviews.entry(review.template_id.clone()).or_default().push(review.clone());

        // Update template rating
        self.update_template_rating(&review.template_id)?;

        Ok(())
    }

    /// Update template rating
    fn update_template_rating(&mut self, template_id: &str) -> Result<(), String> {
        if let Some(reviews) = self.reviews.get(template_id) {
            if let Some(template) = self.templates.get_mut(template_id) {
                let total: u64 = reviews.iter().map(|r| r.rating as u64).sum();
                let count = reviews.len() as u64;

                template.stats.rating = if count > 0 {
                    total as f64 / count as f64
                } else {
                    0.0
                };
                template.stats.rating_count = count;
            }
        }

        Ok(())
    }

    /// Get reviews for a template
    pub fn get_reviews(&self, template_id: &str) -> Vec<TemplateReview> {
        self.reviews.get(template_id).cloned().unwrap_or_default()
    }

    /// Get popular templates
    pub fn get_popular_templates(&self, limit: usize) -> Vec<OrchestrationTemplate> {
        let mut templates: Vec<_> =
            self.templates.values().filter(|t| t.published).cloned().collect();

        templates.sort_by(|a, b| {
            let score_a = a.stats.downloads + a.stats.stars * 2;
            let score_b = b.stats.downloads + b.stats.stars * 2;
            score_b.cmp(&score_a)
        });

        templates.into_iter().take(limit).collect()
    }

    /// Get templates by category
    pub fn get_templates_by_category(
        &self,
        category: TemplateCategory,
    ) -> Vec<OrchestrationTemplate> {
        self.templates
            .values()
            .filter(|t| t.published && t.category == category)
            .cloned()
            .collect()
    }

    /// Get user templates
    pub fn get_user_templates(&self, author_email: &str) -> Vec<OrchestrationTemplate> {
        self.templates
            .values()
            .filter(|t| t.author_email == author_email)
            .cloned()
            .collect()
    }

    /// Update template
    pub fn update_template(
        &mut self,
        template_id: &str,
        updates: OrchestrationTemplate,
    ) -> Result<(), String> {
        if let Some(template) = self.templates.get_mut(template_id) {
            *template = updates;
            template.updated_at = Utc::now();
            Ok(())
        } else {
            Err(format!("Template '{}' not found", template_id))
        }
    }

    /// Delete template
    pub fn delete_template(&mut self, template_id: &str) -> Result<(), String> {
        if self.templates.remove(template_id).is_some() {
            self.reviews.remove(template_id);
            Ok(())
        } else {
            Err(format!("Template '{}' not found", template_id))
        }
    }

    /// Get template count
    pub fn template_count(&self) -> usize {
        self.templates.values().filter(|t| t.published).count()
    }
}

impl Default for TemplateMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_template() -> OrchestrationTemplate {
        OrchestrationTemplate {
            id: "test-template-1".to_string(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            author: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            version: "1.0.0".to_string(),
            category: TemplateCategory::NetworkChaos,
            tags: vec!["test".to_string(), "network".to_string()],
            content: serde_json::json!({}),
            readme: "# Test Template".to_string(),
            example_usage: None,
            requirements: vec![],
            compatibility: CompatibilityInfo {
                min_version: "0.1.0".to_string(),
                max_version: None,
                required_features: vec![],
                protocols: vec!["http".to_string()],
            },
            stats: TemplateStats {
                downloads: 0,
                stars: 0,
                forks: 0,
                rating: 0.0,
                rating_count: 0,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            published: true,
        }
    }

    #[test]
    fn test_publish_template() {
        let mut marketplace = TemplateMarketplace::new();
        let template = create_test_template();

        marketplace.publish_template(template).unwrap();
        assert_eq!(marketplace.template_count(), 1);
    }

    #[test]
    fn test_download_template() {
        let mut marketplace = TemplateMarketplace::new();
        let template = create_test_template();
        marketplace.publish_template(template).unwrap();

        marketplace.download_template("test-template-1").unwrap();

        let downloaded = marketplace.get_template("test-template-1").unwrap();
        assert_eq!(downloaded.stats.downloads, 1);
    }

    #[test]
    fn test_star_template() {
        let mut marketplace = TemplateMarketplace::new();
        let template = create_test_template();
        marketplace.publish_template(template).unwrap();

        marketplace.star_template("test-template-1").unwrap();

        let starred = marketplace.get_template("test-template-1").unwrap();
        assert_eq!(starred.stats.stars, 1);
    }

    #[test]
    fn test_add_review() {
        let mut marketplace = TemplateMarketplace::new();
        let template = create_test_template();
        marketplace.publish_template(template).unwrap();

        let review = TemplateReview {
            id: "review-1".to_string(),
            template_id: "test-template-1".to_string(),
            user_id: "user-1".to_string(),
            user_name: "Test User".to_string(),
            rating: 5,
            comment: "Great template!".to_string(),
            created_at: Utc::now(),
            helpful_count: 0,
        };

        marketplace.add_review(review).unwrap();

        let reviews = marketplace.get_reviews("test-template-1");
        assert_eq!(reviews.len(), 1);

        let template = marketplace.get_template("test-template-1").unwrap();
        assert_eq!(template.stats.rating, 5.0);
    }

    #[test]
    fn test_search_templates() {
        let mut marketplace = TemplateMarketplace::new();
        let template = create_test_template();
        marketplace.publish_template(template).unwrap();

        let filters = TemplateSearchFilters {
            category: Some(TemplateCategory::NetworkChaos),
            tags: vec![],
            min_rating: None,
            author: None,
            query: None,
            sort_by: TemplateSortBy::Newest,
            limit: 10,
            offset: 0,
        };

        let results = marketplace.search_templates(filters);
        assert_eq!(results.len(), 1);
    }
}

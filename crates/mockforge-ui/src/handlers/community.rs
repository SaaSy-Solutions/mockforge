//! Community portal handlers
//!
//! Provides endpoints for showcase gallery, learning resources, and community features

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use crate::handlers::AdminState;
use crate::models::ApiResponse;

/// Showcase project entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowcaseProject {
    pub id: String,
    pub title: String,
    pub author: String,
    pub author_avatar: Option<String>,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub featured: bool,
    pub screenshot: Option<String>,
    pub demo_url: Option<String>,
    pub source_url: Option<String>,
    pub template_id: Option<String>,
    pub scenario_id: Option<String>,
    pub stats: ShowcaseStats,
    pub testimonials: Vec<Testimonial>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// Showcase statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowcaseStats {
    pub downloads: u64,
    pub stars: u64,
    pub forks: u64,
    pub rating: f64,
}

/// Testimonial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testimonial {
    pub author: String,
    pub company: Option<String>,
    pub text: String,
}

/// Success story
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessStory {
    pub id: String,
    pub title: String,
    pub company: String,
    pub industry: String,
    pub author: String,
    pub role: String,
    pub date: chrono::DateTime<Utc>,
    pub challenge: String,
    pub solution: String,
    pub results: Vec<String>,
    pub featured: bool,
}

/// Learning resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningResource {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub resource_type: String, // tutorial, example, video, guide
    pub difficulty: String,    // beginner, intermediate, advanced
    pub tags: Vec<String>,
    pub content_url: Option<String>,
    pub video_url: Option<String>,
    pub code_examples: Vec<CodeExample>,
    pub author: String,
    pub views: u64,
    pub rating: f64,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// Code example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub title: String,
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Get showcase projects
pub async fn get_showcase_projects(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<ShowcaseProject>>> {
    let category = params.get("category");
    let featured = params.get("featured").map(|s| s == "true");
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
    let offset = params.get("offset").and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);

    // Note: Community showcase requires database or content storage integration
    // To enable database persistence:
    // 1. Create showcase_projects table in database
    // 2. Add database accessor to handler state
    // 3. Query projects with filters (category, featured, pagination)
    // For now, returns mock data for UI development
    let projects = vec![ShowcaseProject {
        id: "ecommerce-platform".to_string(),
        title: "E-commerce Platform Mock".to_string(),
        author: "community-user".to_string(),
        author_avatar: None,
        description: "Complete e-commerce API mock with shopping carts, orders, and payments"
            .to_string(),
        category: "ecommerce".to_string(),
        tags: vec![
            "ecommerce".to_string(),
            "shopping-cart".to_string(),
            "payments".to_string(),
        ],
        featured: true,
        screenshot: Some("https://example.com/screenshot.png".to_string()),
        demo_url: Some("https://demo.mockforge.dev/ecommerce".to_string()),
        source_url: Some("https://github.com/user/ecommerce-mock".to_string()),
        template_id: Some("ecommerce-store@1.0.0".to_string()),
        scenario_id: Some("ecommerce-scenario@1.0.0".to_string()),
        stats: ShowcaseStats {
            downloads: 1250,
            stars: 45,
            forks: 12,
            rating: 4.8,
        },
        testimonials: vec![Testimonial {
            author: "John Doe".to_string(),
            company: Some("Acme Corp".to_string()),
            text: "This template saved us weeks of development time!".to_string(),
        }],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }];

    Json(ApiResponse {
        success: true,
        data: Some(projects),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Get showcase project by ID
pub async fn get_showcase_project(
    Path(project_id): Path<String>,
) -> Json<ApiResponse<ShowcaseProject>> {
    // Note: Requires database integration (see list_showcase_projects comment)
    // For now, returns mock data
    let project = ShowcaseProject {
        id: project_id.clone(),
        title: "E-commerce Platform Mock".to_string(),
        author: "community-user".to_string(),
        author_avatar: None,
        description: "Complete e-commerce API mock".to_string(),
        category: "ecommerce".to_string(),
        tags: vec!["ecommerce".to_string()],
        featured: true,
        screenshot: None,
        demo_url: None,
        source_url: None,
        template_id: None,
        scenario_id: None,
        stats: ShowcaseStats {
            downloads: 0,
            stars: 0,
            forks: 0,
            rating: 0.0,
        },
        testimonials: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Json(ApiResponse {
        success: true,
        data: Some(project),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Get showcase categories
pub async fn get_showcase_categories() -> Json<ApiResponse<Vec<String>>> {
    let categories = vec![
        "ecommerce".to_string(),
        "finance".to_string(),
        "healthcare".to_string(),
        "social".to_string(),
        "iot".to_string(),
        "gaming".to_string(),
        "education".to_string(),
        "other".to_string(),
    ];

    Json(ApiResponse {
        success: true,
        data: Some(categories),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Get success stories
pub async fn get_success_stories(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<SuccessStory>>> {
    let featured = params.get("featured").map(|s| s == "true");
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);

    // Note: Requires database integration (see list_showcase_projects comment)
    // For now, returns mock data
    let stories = vec![
        SuccessStory {
            id: "acme-corp".to_string(),
            title: "Acme Corp: Accelerating API Development".to_string(),
            company: "Acme Corporation".to_string(),
            industry: "E-commerce".to_string(),
            author: "Jane Smith".to_string(),
            role: "Lead API Developer".to_string(),
            date: Utc::now(),
            challenge: "Acme Corp needed to develop a new payment API but couldn't wait for backend services to be ready.".to_string(),
            solution: "Used MockForge to create realistic payment mocks with various scenarios (success, failure, retry).".to_string(),
            results: vec![
                "Reduced development time by 60%".to_string(),
                "Enabled parallel frontend/backend development".to_string(),
                "Improved test coverage with edge cases".to_string(),
            ],
            featured: true,
        },
    ];

    Json(ApiResponse {
        success: true,
        data: Some(stories),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Get learning resources
pub async fn get_learning_resources(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<LearningResource>>> {
    let category = params.get("category");
    let resource_type = params.get("type");
    let difficulty = params.get("difficulty");
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);

    // Note: Requires database integration (see list_showcase_projects comment)
    // For now, returns mock data or content storage
    let resources = vec![LearningResource {
        id: "getting-started".to_string(),
        title: "Getting Started with MockForge".to_string(),
        description: "Learn how to create your first mock API in minutes".to_string(),
        category: "tutorial".to_string(),
        resource_type: "tutorial".to_string(),
        difficulty: "beginner".to_string(),
        tags: vec!["getting-started".to_string(), "tutorial".to_string()],
        content_url: Some("/docs/getting-started".to_string()),
        video_url: None,
        code_examples: vec![CodeExample {
            title: "Basic REST API".to_string(),
            language: "yaml".to_string(),
            code: "http:\n  port: 3000\n  routes:\n    - path: /users\n      method: GET"
                .to_string(),
            description: Some("Simple REST endpoint".to_string()),
        }],
        author: "MockForge Team".to_string(),
        views: 1500,
        rating: 4.9,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }];

    Json(ApiResponse {
        success: true,
        data: Some(resources),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Get learning resource by ID
pub async fn get_learning_resource(
    Path(resource_id): Path<String>,
) -> Json<ApiResponse<LearningResource>> {
    // Note: Requires database integration (see list_showcase_projects comment)
    // For now, returns mock data
    let resource = LearningResource {
        id: resource_id.clone(),
        title: "Getting Started with MockForge".to_string(),
        description: "Learn how to create your first mock API".to_string(),
        category: "tutorial".to_string(),
        resource_type: "tutorial".to_string(),
        difficulty: "beginner".to_string(),
        tags: vec!["getting-started".to_string()],
        content_url: None,
        video_url: None,
        code_examples: vec![],
        author: "MockForge Team".to_string(),
        views: 0,
        rating: 0.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Json(ApiResponse {
        success: true,
        data: Some(resource),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Get learning resource categories
pub async fn get_learning_categories() -> Json<ApiResponse<Vec<String>>> {
    let categories = vec![
        "getting-started".to_string(),
        "advanced-features".to_string(),
        "integration".to_string(),
        "best-practices".to_string(),
    ];

    Json(ApiResponse {
        success: true,
        data: Some(categories),
        error: None,
        timestamp: Utc::now(),
    })
}

/// Submit a project for showcase
pub async fn submit_showcase_project(
    State(_state): State<AdminState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Note: Project submission requires database integration
    // To enable:
    // 1. Validate project data (title, description, files, etc.)
    // 2. Store project metadata in database
    // 3. Store project files in object storage (S3, etc.)
    // 4. Return created project with ID
    // For now, returns success response with mock project
    // Requires authentication and validation

    Ok(Json(ApiResponse {
        success: true,
        data: Some("Project submitted successfully".to_string()),
        error: None,
        timestamp: Utc::now(),
    }))
}

//! Community portal handlers backed by local content storage.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use uuid::Uuid;

use crate::handlers::AdminState;
use crate::models::ApiResponse;

const DEFAULT_COMMUNITY_CONTENT_FILE: &str = "community/content.json";

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
    pub resource_type: String,
    pub difficulty: String,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CommunityContentStore {
    #[serde(default)]
    showcase_projects: Vec<ShowcaseProject>,
    #[serde(default)]
    success_stories: Vec<SuccessStory>,
    #[serde(default)]
    learning_resources: Vec<LearningResource>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitShowcaseRequest {
    title: String,
    description: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    author: Option<String>,
    author_avatar: Option<String>,
    screenshot: Option<String>,
    demo_url: Option<String>,
    source_url: Option<String>,
    template_id: Option<String>,
    scenario_id: Option<String>,
}

fn content_file_path() -> PathBuf {
    std::env::var("MOCKFORGE_COMMUNITY_CONTENT_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_COMMUNITY_CONTENT_FILE))
}

async fn load_store() -> CommunityContentStore {
    let path = content_file_path();
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(_) => return CommunityContentStore::default(),
    };

    serde_json::from_slice::<CommunityContentStore>(&bytes).unwrap_or_default()
}

async fn save_store(store: &CommunityContentStore) -> std::result::Result<(), String> {
    let path = content_file_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create content directory: {}", e))?;
    }

    let json = serde_json::to_vec_pretty(store)
        .map_err(|e| format!("Failed to serialize community content: {}", e))?;

    tokio::fs::write(path, json)
        .await
        .map_err(|e| format!("Failed to write community content: {}", e))
}

fn query_bool(params: &HashMap<String, String>, key: &str) -> Option<bool> {
    params.get(key).and_then(|value| match value.as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    })
}

/// Get showcase projects
pub async fn get_showcase_projects(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<ShowcaseProject>>> {
    let category = params.get("category").map(String::as_str);
    let featured = query_bool(&params, "featured");
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
    let offset = params.get("offset").and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);

    let mut projects = load_store().await.showcase_projects;

    if let Some(category) = category {
        projects.retain(|p| p.category == category);
    }

    if let Some(featured) = featured {
        projects.retain(|p| p.featured == featured);
    }

    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let projects = projects.into_iter().skip(offset).take(limit).collect();

    Json(ApiResponse::success(projects))
}

/// Get showcase project by ID
pub async fn get_showcase_project(
    Path(project_id): Path<String>,
) -> Json<ApiResponse<ShowcaseProject>> {
    let store = load_store().await;
    let project = store.showcase_projects.into_iter().find(|p| p.id == project_id);

    match project {
        Some(project) => Json(ApiResponse::success(project)),
        None => Json(ApiResponse::error(format!("Showcase project not found: {}", project_id))),
    }
}

/// Get showcase categories
pub async fn get_showcase_categories() -> Json<ApiResponse<Vec<String>>> {
    let store = load_store().await;
    let mut categories: HashSet<String> = store
        .showcase_projects
        .iter()
        .map(|p| p.category.clone())
        .filter(|c| !c.is_empty())
        .collect();

    if categories.is_empty() {
        categories.insert("other".to_string());
    }

    let mut categories: Vec<String> = categories.into_iter().collect();
    categories.sort();

    Json(ApiResponse::success(categories))
}

/// Get success stories
pub async fn get_success_stories(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<SuccessStory>>> {
    let featured = query_bool(&params, "featured");
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);

    let mut stories = load_store().await.success_stories;
    if let Some(featured) = featured {
        stories.retain(|story| story.featured == featured);
    }

    stories.sort_by(|a, b| b.date.cmp(&a.date));
    stories.truncate(limit);

    Json(ApiResponse::success(stories))
}

/// Get learning resources
pub async fn get_learning_resources(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<LearningResource>>> {
    let category = params.get("category").map(String::as_str);
    let resource_type = params.get("type").map(String::as_str);
    let difficulty = params.get("difficulty").map(String::as_str);
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);

    let mut resources = load_store().await.learning_resources;

    if let Some(category) = category {
        resources.retain(|r| r.category == category);
    }

    if let Some(resource_type) = resource_type {
        resources.retain(|r| r.resource_type == resource_type);
    }

    if let Some(difficulty) = difficulty {
        resources.retain(|r| r.difficulty == difficulty);
    }

    resources.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    resources.truncate(limit);

    Json(ApiResponse::success(resources))
}

/// Get learning resource by ID
pub async fn get_learning_resource(
    Path(resource_id): Path<String>,
) -> Json<ApiResponse<LearningResource>> {
    let store = load_store().await;
    let resource = store.learning_resources.into_iter().find(|resource| resource.id == resource_id);

    match resource {
        Some(resource) => Json(ApiResponse::success(resource)),
        None => Json(ApiResponse::error(format!("Learning resource not found: {}", resource_id))),
    }
}

/// Get learning resource categories
pub async fn get_learning_categories() -> Json<ApiResponse<Vec<String>>> {
    let store = load_store().await;
    let mut categories: HashSet<String> = store
        .learning_resources
        .iter()
        .map(|r| r.category.clone())
        .filter(|c| !c.is_empty())
        .collect();

    if categories.is_empty() {
        categories.insert("getting-started".to_string());
    }

    let mut categories: Vec<String> = categories.into_iter().collect();
    categories.sort();

    Json(ApiResponse::success(categories))
}

/// Submit a project for showcase
pub async fn submit_showcase_project(
    State(_state): State<AdminState>,
    Json(payload): Json<SubmitShowcaseRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    if payload.title.trim().is_empty() || payload.description.trim().is_empty() {
        return Ok(Json(ApiResponse::error("title and description are required".to_string())));
    }

    let mut store = load_store().await;

    let now = Utc::now();
    let project = ShowcaseProject {
        id: Uuid::new_v4().to_string(),
        title: payload.title,
        author: payload.author.unwrap_or_else(|| "anonymous".to_string()),
        author_avatar: payload.author_avatar,
        description: payload.description,
        category: payload.category.unwrap_or_else(|| "other".to_string()),
        tags: payload.tags.unwrap_or_default(),
        featured: false,
        screenshot: payload.screenshot,
        demo_url: payload.demo_url,
        source_url: payload.source_url,
        template_id: payload.template_id,
        scenario_id: payload.scenario_id,
        stats: ShowcaseStats {
            downloads: 0,
            stars: 0,
            forks: 0,
            rating: 0.0,
        },
        testimonials: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    store.showcase_projects.push(project);

    if let Err(e) = save_store(&store).await {
        tracing::error!("Failed to persist showcase project: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ApiResponse::success("Project submitted successfully".to_string())))
}

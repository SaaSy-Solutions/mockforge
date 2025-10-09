//! Authentication handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{create_token, hash_password, verify_password},
    error::{ApiError, ApiResult},
    models::User,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let pool = state.db.pool();

    // Validate input
    if request.username.len() < 3 {
        return Err(ApiError::InvalidRequest(
            "Username must be at least 3 characters".to_string(),
        ));
    }

    if request.password.len() < 8 {
        return Err(ApiError::InvalidRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Check if user already exists
    if User::find_by_email(pool, &request.email)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
        return Err(ApiError::InvalidRequest(
            "Email already registered".to_string(),
        ));
    }

    if User::find_by_username(pool, &request.username)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
        return Err(ApiError::InvalidRequest(
            "Username already taken".to_string(),
        ));
    }

    // Hash password
    let password_hash = hash_password(&request.password)
        .map_err(|e| ApiError::Internal(e))?;

    // Create user
    let user = User::create(pool, &request.username, &request.email, &password_hash)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Generate JWT token
    let token = create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json(LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let pool = state.db.pool();

    // Find user
    let user = User::find_by_email(pool, &request.email)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Invalid email or password".to_string()))?;

    // Verify password
    let valid = verify_password(&request.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(e))?;

    if !valid {
        return Err(ApiError::InvalidRequest(
            "Invalid email or password".to_string(),
        ));
    }

    // Generate JWT token
    let token = create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

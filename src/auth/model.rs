use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Admin user stored in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Admin {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub refresh_token: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

/// Admin info for API responses (without sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminInfo {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<Admin> for AdminInfo {
    fn from(admin: Admin) -> Self {
        Self {
            id: admin.id,
            username: admin.username,
            display_name: admin.display_name,
            created_at: admin.created_at,
        }
    }
}

/// Login request payload
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Token response after successful login
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    /// True if this is first-time setup with default credentials
    pub setup_mode: bool,
}

/// Refresh token request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Create admin request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAdminRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // admin id
    pub username: String,
    pub exp: usize,         // expiration time
    pub iat: usize,         // issued at
    pub token_type: String, // "access" or "refresh"
}

/// Auth status response
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthStatusResponse {
    pub has_admins: bool,
    pub setup_required: bool,
}

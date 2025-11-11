use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, sqlx::FromRow)]
pub struct Asset {
    #[schema(example = "a1b2c3d4-e5f6-7890-1234-567890abcdef")]
    pub id: Uuid,
    #[schema(example = "My Cool Image")]
    pub name: String,
    #[schema(example = "image.png")]
    pub filename: String,
    #[schema(example = "https://example.com/assets/image.png")]
    pub url: String,
    #[schema(example = "This is an example image asset.")]
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Asset {
    pub fn new(name: String, filename: String, url: String, description: Option<String>) -> Self {
        Asset {
            id: Uuid::new_v4(),
            name,
            filename,
            url,
            description,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}

use chrono::{NaiveDate, DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, sqlx::FromRow)]
pub struct Post {
    #[schema(example = "f1e2d3c4-b5a6-7890-1234-567890abcdef")]
    pub id: Uuid,
    #[schema(example = "Judul Posting")]
    pub title: String,
    #[schema(example = "Kategori Posting")]
    pub category: String,
    #[schema(example = "2025-11-05")]
    pub date: NaiveDate,
    #[schema(example = "Ini adalah ringkasan postingan.")]
    pub excerpt: String,
    #[schema(example = "posts/f1e2d3c4-b5a6-7890-1234-567890abcdef")]
    pub folder_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PostWithAssets {
    pub id: Uuid,
    pub title: String,
    pub category: String,
    pub date: NaiveDate,
    pub excerpt: String,
    pub folder_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub asset_ids: Vec<Uuid>,
}



#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]

pub struct CreatePostingRequest {

    pub title: String,

    pub category: String,

    pub excerpt: String,

}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePostingRequest {
    #[schema(example = "Judul Posting Diperbarui")]
    pub title: Option<String>,
    #[schema(example = "Kategori Posting Diperbarui")]
    pub category: Option<String>,
    #[schema(example = "Ini adalah ringkasan postingan yang sudah diperbarui.")]
    pub excerpt: Option<String>,
    #[schema(example = "posts/f1e2d3c4-b5a6-7890-1234-567890abcdef")]
    pub folder_id: Option<String>,
}



impl Post {
    pub fn new(title: String, category: String, excerpt: String, folder_id: Option<String>) -> Self {
        Post {
            id: Uuid::new_v4(),
            title,
            category,
            date: chrono::Local::now().date_naive(),
            excerpt,
            folder_id,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}

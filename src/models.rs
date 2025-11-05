use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{NaiveDate};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Asset {
    #[schema(example = "a1b2c3d4-e5f6-7890-1234-567890abcdef")]
    pub id: Uuid,
    #[schema(example = "image.png")]
    pub filename: String,
    #[schema(example = "https://example.com/assets/image.png")]
    pub url: String,
    #[schema(example = "This is an example image asset.")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Posting {
    #[schema(example = "f1e2d3c4-b5a6-7890-1234-567890abcdef")]
    pub id: Uuid,
    #[schema(example = "Judul Posting")]
    pub judul: String,
    #[schema(example = "2025-11-05")]
    pub tanggal: NaiveDate,
    #[schema(example = "## Detail Posting\n\nIni adalah detail posting dengan gambar: ![gambar](https://example.com/assets/image.png)")]
    pub detail: String, 
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePostingRequest {
    #[schema(example = "Judul Posting Baru")]
    pub judul: String,
    #[schema(example = "## Detail Posting Baru\n\nIni adalah detail posting baru.")]
    pub detail: String,
    pub assets: Option<Vec<Asset>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePostingRequest {
    #[schema(example = "Judul Posting Diperbarui")]
    pub judul: Option<String>,
    #[schema(example = "## Detail Posting Diperbarui\n\nIni adalah detail posting yang sudah diperbarui.")]
    pub detail: Option<String>,
    pub assets: Option<Vec<Asset>>,
}

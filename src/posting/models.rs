use chrono::{NaiveDate, DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Posting {
    #[schema(example = "f1e2d3c4-b5a6-7890-1234-567890abcdef")]
    pub id: Uuid,
    #[schema(example = "Judul Posting")]
    pub judul: String,
    #[schema(example = "2025-11-05")]
    pub tanggal: NaiveDate,
    #[schema(
        example = "## Detail Posting\n\nIni adalah detail posting dengan gambar: ![gambar](https://example.com/assets/image.png)"
    )]
    pub detail: String,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePostingRequest {
    #[schema(example = "Judul Posting Baru")]
    pub judul: String,
    #[schema(example = "## Detail Posting Baru\n\nIni adalah detail posting baru.")]
    pub detail: String,
    pub asset_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePostingRequest {
    #[schema(example = "Judul Posting Diperbarui")]
    pub judul: Option<String>,
    #[schema(
        example = "## Detail Posting Diperbarui\n\nIni adalah detail posting yang sudah diperbarui."
    )]
    pub detail: Option<String>,
    pub asset_ids: Option<Vec<Uuid>>,
}

// #[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
// pub struct PostingAsset {
//     pub posting_id: Uuid,
//     pub asset_id: Uuid,
// }

impl Posting {
    pub fn new(judul: String, detail: String, asset_ids: Vec<Uuid>) -> Self {
        Posting {
            id: Uuid::new_v4(),
            judul,
            tanggal: chrono::Local::now().date_naive(),
            detail,
            asset_ids,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}

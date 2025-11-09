use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
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
}

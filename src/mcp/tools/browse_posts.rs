//! MCP Tools for browsing posts/postings.
//!
//! These tools allow AI agents to browse posts on the Kelurahan Cakung Barat website
//! with filtering, sorting, and pagination capabilities.
//!
//! All tools use cache-first strategy - same cache as REST endpoints to avoid
//! double database traffic.

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::registry::ToolDescriptor;

// =============================================================================
// Tool Names
// =============================================================================

pub const LIST_POSTINGS_TOOL: &str = "list_postings";
pub const GET_POSTING_DETAIL_TOOL: &str = "get_posting_detail";
pub const LIST_CATEGORIES_TOOL: &str = "list_categories";

// =============================================================================
// Tool Descriptors
// =============================================================================

pub fn list_postings_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: LIST_POSTINGS_TOOL.to_string(),
        description: concat!(
            "Melihat daftar postingan, berita, dan informasi terbaru di Kelurahan Cakung Barat. ",
            "Gunakan tool ini untuk mendapatkan update terkini mengenai kegiatan dan pengumuman kelurahan. ",
            "Hasil bisa difilter berdasarkan kategori dan diurutkan berdasarkan tanggal. ",
            "Gunakan tool ini untuk: ",
            "(1) Melihat berita terbaru, ",
            "(2) Mencari informasi berdasarkan kategori tertentu, ",
            "(3) Melihat daftar posting dengan pagination."
        )
        .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "Filter berdasarkan kategori (opsional). Gunakan list_categories untuk melihat kategori yang tersedia."
                },
                "sort_by": {
                    "type": "string",
                    "enum": ["latest", "oldest"],
                    "description": "Urutan hasil (default: latest)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Jumlah maksimal hasil (default: 10, max: 50)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Offset untuk pagination (default: 0)"
                }
            }
        }),
    }
}

pub fn get_posting_detail_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: GET_POSTING_DETAIL_TOOL.to_string(),
        description: concat!(
            "Melihat detail lengkap satu postingan atau berita berdasarkan ID. ",
            "Gunakan tool ini untuk membaca isi lengkap informasi terbaru Kelurahan Cakung Barat ",
            "setelah menemukan ID posting dari list_postings."
        )
        .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "ID postingan (format UUID)"
                }
            },
            "required": ["id"]
        }),
    }
}

pub fn list_categories_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: LIST_CATEGORIES_TOOL.to_string(),
        description: concat!(
            "Melihat daftar semua kategori postingan yang tersedia. ",
            "Gunakan tool ini untuk mengetahui kategori apa saja yang bisa ",
            "digunakan sebagai filter di list_postings."
        )
        .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ListPostingsRequest {
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_sort_by() -> String {
    "latest".to_string()
}

fn default_limit() -> i32 {
    10
}

impl ListPostingsRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.limit < 1 {
            return Err("Limit harus lebih dari 0".to_string());
        }
        if self.limit > 50 {
            return Err("Limit maksimal adalah 50".to_string());
        }
        if self.offset < 0 {
            return Err("Offset tidak boleh negatif".to_string());
        }
        if self.sort_by != "latest" && self.sort_by != "oldest" {
            return Err("sort_by harus 'latest' atau 'oldest'".to_string());
        }
        Ok(())
    }

    pub fn is_sort_latest(&self) -> bool {
        self.sort_by == "latest"
    }
}

#[derive(Debug, Deserialize)]
pub struct GetPostingDetailRequest {
    pub id: String,
}

impl GetPostingDetailRequest {
    pub fn validate(&self) -> Result<uuid::Uuid, String> {
        if self.id.trim().is_empty() {
            return Err("ID postingan tidak boleh kosong".to_string());
        }
        uuid::Uuid::parse_str(&self.id)
            .map_err(|_| format!("ID '{}' bukan format UUID yang valid", self.id))
    }
}

/// Response for a single post in list
#[derive(Debug, Serialize)]
pub struct PostListItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub date: String,
    pub excerpt: String,
    pub image_url: Option<String>,
}

/// Response for list_postings tool
#[derive(Debug, Serialize)]
pub struct ListPostingsResponse {
    pub posts: Vec<PostListItem>,
    pub total: usize,
    pub limit: i32,
    pub offset: i32,
    pub has_more: bool,
}

/// Response for get_posting_detail tool
#[derive(Debug, Serialize)]
pub struct PostDetailResponse {
    pub id: String,
    pub title: String,
    pub category: String,
    pub date: String,
    pub excerpt: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub image_urls: Vec<String>,
}

/// Response for list_categories tool
#[derive(Debug, Serialize)]
pub struct ListCategoriesResponse {
    pub categories: Vec<String>,
    pub count: usize,
}

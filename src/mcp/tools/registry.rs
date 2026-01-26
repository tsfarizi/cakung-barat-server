//! Tool registry - central routing for MCP tools.
//!
//! Provides `list_tools()` and `call_tool()` / `call_tool_async()` functionality per MCP spec.

use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::AppState;
use crate::mcp::content::{ContentItem, ToolResult};
use crate::mcp::generators::{
    GeneratedDocument, GeneratorError, SuratKprGenerator, SuratKprRequest, SuratNibNpwpGenerator,
    SuratNibNpwpRequest, SuratTidakMampuGenerator, SuratTidakMampuRequest,
};

use super::browse_posts::{
    self, GetPostingDetailRequest, ListCategoriesResponse, ListPostingsRequest,
    ListPostingsResponse, PostDetailResponse, PostListItem,
};
use super::organization;
use super::surat_kpr;
use super::surat_nib_npwp;
use super::surat_tidak_mampu;

/// Tool descriptor conforming to MCP specification.
#[derive(Debug, Serialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Central registry for all MCP tools.
pub struct ToolRegistry {
    surat_tidak_mampu: SuratTidakMampuGenerator,
    surat_kpr: SuratKprGenerator,
    surat_nib_npwp: SuratNibNpwpGenerator,
}

impl ToolRegistry {
    /// Create a new registry with all generators initialized.
    pub fn new() -> Result<Self, GeneratorError> {
        Ok(Self {
            surat_tidak_mampu: SuratTidakMampuGenerator::new()?,
            surat_kpr: SuratKprGenerator::new()?,
            surat_nib_npwp: SuratNibNpwpGenerator::new()?,
        })
    }

    /// List all available tools per MCP spec.
    pub fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            // Document generation tools
            surat_tidak_mampu::descriptor(),
            surat_kpr::descriptor(),
            surat_nib_npwp::descriptor(),
            // Post browsing tools
            browse_posts::list_postings_descriptor(),
            browse_posts::get_posting_detail_descriptor(),
            browse_posts::list_categories_descriptor(),
            // Organization tools
            organization::get_organization_structure_descriptor(),
        ]
    }

    /// Call a tool by name with the given arguments (async version).
    /// Handles both sync document tools and async database tools.
    pub async fn call_tool_async(
        &self,
        name: &str,
        arguments: Option<Value>,
        app_state: &web::Data<AppState>,
    ) -> ToolResult {
        match name {
            // Sync document generation tools
            surat_tidak_mampu::TOOL_NAME => self.call_surat_tidak_mampu(arguments),
            surat_kpr::TOOL_NAME => self.call_surat_kpr(arguments),
            surat_nib_npwp::TOOL_NAME => self.call_surat_nib_npwp(arguments),

            // Async database tools
            browse_posts::LIST_POSTINGS_TOOL => self.call_list_postings(arguments, app_state).await,
            browse_posts::GET_POSTING_DETAIL_TOOL => {
                self.call_get_posting_detail(arguments, app_state).await
            }
            browse_posts::LIST_CATEGORIES_TOOL => self.call_list_categories(app_state).await,
            organization::GET_ORGANIZATION_STRUCTURE_TOOL => {
                self.call_get_organization_structure(app_state).await
            }

            _ => ToolResult::error(format!(
                "Tool '{}' tidak tersedia. Tools yang tersedia: {}, {}, {}, {}, {}, {}, {}",
                name,
                surat_tidak_mampu::TOOL_NAME,
                surat_kpr::TOOL_NAME,
                surat_nib_npwp::TOOL_NAME,
                browse_posts::LIST_POSTINGS_TOOL,
                browse_posts::GET_POSTING_DETAIL_TOOL,
                browse_posts::LIST_CATEGORIES_TOOL,
                organization::GET_ORGANIZATION_STRUCTURE_TOOL,
            )),
        }
    }

    /// Call a tool by name with the given arguments (sync version for backward compatibility).
    pub fn call_tool(&self, name: &str, arguments: Option<Value>) -> ToolResult {
        match name {
            surat_tidak_mampu::TOOL_NAME => self.call_surat_tidak_mampu(arguments),
            surat_kpr::TOOL_NAME => self.call_surat_kpr(arguments),
            surat_nib_npwp::TOOL_NAME => self.call_surat_nib_npwp(arguments),
            _ => ToolResult::error(format!(
                "Tool '{}' tidak tersedia. Tools yang tersedia: {}, {}, {}",
                name,
                surat_tidak_mampu::TOOL_NAME,
                surat_kpr::TOOL_NAME,
                surat_nib_npwp::TOOL_NAME
            )),
        }
    }

    // =========================================================================
    // Sync document generation tools
    // =========================================================================

    fn call_surat_tidak_mampu(&self, arguments: Option<Value>) -> ToolResult {
        let request = match parse_arguments::<SuratTidakMampuRequest>(arguments) {
            Ok(req) => req,
            Err(err) => return ToolResult::error(err),
        };

        // Validate input before processing
        if let Err(validation_error) = request.validate() {
            return ToolResult::error(validation_error);
        }

        match self.surat_tidak_mampu.generate(request) {
            Ok(doc) => self.success_result(doc, "Surat Pernyataan Tidak Mampu"),
            Err(err) => ToolResult::error(format!("Gagal membuat surat: {}", err)),
        }
    }

    fn call_surat_kpr(&self, arguments: Option<Value>) -> ToolResult {
        let request = match parse_arguments::<SuratKprRequest>(arguments) {
            Ok(req) => req,
            Err(err) => return ToolResult::error(err),
        };

        // Validate input before processing
        if let Err(validation_error) = request.validate() {
            return ToolResult::error(validation_error);
        }

        match self.surat_kpr.generate(request) {
            Ok(doc) => self.success_result(doc, "Surat Pernyataan Belum Memiliki Rumah"),
            Err(err) => ToolResult::error(format!("Gagal membuat surat: {}", err)),
        }
    }

    fn call_surat_nib_npwp(&self, arguments: Option<Value>) -> ToolResult {
        let request = match parse_arguments::<SuratNibNpwpRequest>(arguments) {
            Ok(req) => req,
            Err(err) => return ToolResult::error(err),
        };

        // Validate input before processing
        if let Err(validation_error) = request.validate() {
            return ToolResult::error(validation_error);
        }

        match self.surat_nib_npwp.generate(request) {
            Ok(doc) => self.success_result(doc, "Surat Pernyataan Akan Mengurus NIB & NPWP"),
            Err(err) => ToolResult::error(format!("Gagal membuat surat: {}", err)),
        }
    }

    fn success_result(&self, doc: GeneratedDocument, surat_type: &str) -> ToolResult {
        let text = format!(
            "{} berhasil dibuat.\nFile: {}\nTanggal: {}",
            surat_type, doc.filename, doc.tanggal
        );

        ToolResult::success(vec![
            ContentItem::text(text),
            ContentItem::resource(&doc.pdf, "application/pdf", &doc.filename),
        ])
    }

    // =========================================================================
    // Async database tools for browsing posts
    // =========================================================================

    async fn call_list_postings(
        &self,
        arguments: Option<Value>,
        app_state: &web::Data<AppState>,
    ) -> ToolResult {
        let request = match parse_arguments::<ListPostingsRequest>(arguments) {
            Ok(req) => req,
            Err(err) => return ToolResult::error(err),
        };

        if let Err(validation_error) = request.validate() {
            return ToolResult::error(validation_error);
        }

        // Get filtered posts from cache-first database layer
        let posts = match app_state
            .get_posts_filtered(
                request.category.as_deref(),
                request.is_sort_latest(),
                request.limit,
                request.offset,
            )
            .await
        {
            Ok(posts) => posts,
            Err(err) => {
                return ToolResult::error(format!("Gagal mengambil data postingan: {}", err))
            }
        };

        // Get total count for pagination info
        let total = match app_state
            .count_posts_filtered(request.category.as_deref())
            .await
        {
            Ok(count) => count,
            Err(err) => {
                return ToolResult::error(format!("Gagal menghitung total postingan: {}", err))
            }
        };

        // Enrich posts with image URLs
        let mut posts_with_images = Vec::new();
        for post in posts {
            let mut image_url = None;
            if let Some(folder_name) = &post.folder_id {
                // Try to get assets for the post's folder
                if let Ok(Some(asset_ids)) = app_state.get_folder_contents(folder_name).await {
                    if let Some(first_id) = asset_ids.first() {
                        if let Ok(Some(asset)) = app_state.get_asset_by_id(first_id).await {
                            image_url = Some(asset.url);
                        }
                    }
                }
            }

            posts_with_images.push(PostListItem {
                id: post.id.to_string(),
                title: post.title,
                category: post.category,
                date: post.date.to_string(),
                excerpt: post.excerpt,
                image_url,
            });
        }

        let response = ListPostingsResponse {
            posts: posts_with_images,
            total,
            limit: request.limit,
            offset: request.offset,
            has_more: (request.offset as usize + request.limit as usize) < total,
        };

        let json_text =
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string());

        ToolResult::success(vec![ContentItem::text(json_text)])
    }

    async fn call_get_posting_detail(
        &self,
        arguments: Option<Value>,
        app_state: &web::Data<AppState>,
    ) -> ToolResult {
        let request = match parse_arguments::<GetPostingDetailRequest>(arguments) {
            Ok(req) => req,
            Err(err) => return ToolResult::error(err),
        };

        let uuid = match request.validate() {
            Ok(id) => id,
            Err(err) => return ToolResult::error(err),
        };

        // Get post by ID with assets
        let post_with_assets = match app_state.get_posting_by_id_with_assets(&uuid).await {
            Ok(Some(post)) => post,
            Ok(None) => {
                return ToolResult::error(format!("Postingan dengan ID '{}' tidak ditemukan", uuid))
            }
            Err(err) => {
                return ToolResult::error(format!("Gagal mengambil data postingan: {}", err))
            }
        };

        // Fetch actual asset URLs
        let mut image_urls = Vec::new();
        if !post_with_assets.asset_ids.is_empty() {
            if let Ok(assets) = app_state
                .get_assets_by_ids(&post_with_assets.asset_ids)
                .await
            {
                image_urls = assets.into_iter().map(|a| a.url).collect();
            }
        }

        let response = PostDetailResponse {
            id: post_with_assets.id.to_string(),
            title: post_with_assets.title,
            category: post_with_assets.category,
            date: post_with_assets.date.to_string(),
            excerpt: post_with_assets.excerpt,
            created_at: post_with_assets.created_at.map(|dt| dt.to_rfc3339()),
            updated_at: post_with_assets.updated_at.map(|dt| dt.to_rfc3339()),
            image_urls,
        };

        let json_text =
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string());

        ToolResult::success(vec![ContentItem::text(json_text)])
    }

    async fn call_list_categories(&self, app_state: &web::Data<AppState>) -> ToolResult {
        let categories = match app_state.get_distinct_categories().await {
            Ok(cats) => cats,
            Err(err) => {
                return ToolResult::error(format!("Gagal mengambil daftar kategori: {}", err))
            }
        };

        let response = ListCategoriesResponse {
            count: categories.len(),
            categories,
        };

        let json_text =
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string());

        ToolResult::success(vec![ContentItem::text(json_text)])
    }

    async fn call_get_organization_structure(&self, app_state: &web::Data<AppState>) -> ToolResult {
        let members = match app_state.get_organization_structure().await {
            Ok(m) => m,
            Err(err) => {
                return ToolResult::error(format!("Gagal mengambil struktur organisasi: {}", err))
            }
        };

        let json_text =
            serde_json::to_string_pretty(&members).unwrap_or_else(|_| "{}".to_string());

        ToolResult::success(vec![ContentItem::text(json_text)])
    }
}

fn parse_arguments<T: for<'de> Deserialize<'de>>(arguments: Option<Value>) -> Result<T, String> {
    let value = arguments.unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|err| format!("Argumen tidak valid: {}", err))
}
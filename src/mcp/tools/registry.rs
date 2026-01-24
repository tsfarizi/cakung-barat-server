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

            _ => ToolResult::error(format!(
                "Tool '{}' tidak tersedia. Tools yang tersedia: {}, {}, {}, {}, {}, {}",
                name,
                surat_tidak_mampu::TOOL_NAME,
                surat_kpr::TOOL_NAME,
                surat_nib_npwp::TOOL_NAME,
                browse_posts::LIST_POSTINGS_TOOL,
                browse_posts::GET_POSTING_DETAIL_TOOL,
                browse_posts::LIST_CATEGORIES_TOOL,
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

        let response = ListPostingsResponse {
            posts: posts.into_iter().map(PostListItem::from).collect(),
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

        // Get post by ID
        let post = match app_state.get_post_by_id(&uuid).await {
            Ok(Some(post)) => post,
            Ok(None) => {
                return ToolResult::error(format!("Postingan dengan ID '{}' tidak ditemukan", uuid))
            }
            Err(err) => {
                return ToolResult::error(format!("Gagal mengambil data postingan: {}", err))
            }
        };

        let response = PostDetailResponse::from(post);
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
}

fn parse_arguments<T: for<'de> Deserialize<'de>>(arguments: Option<Value>) -> Result<T, String> {
    let value = arguments.unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|err| format!("Argumen tidak valid: {}", err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================================================
    // ToolRegistry initialization tests
    // ========================================================================

    #[test]
    fn test_registry_new_success() {
        let result = ToolRegistry::new();
        assert!(result.is_ok(), "Registry should initialize successfully");
    }

    // ========================================================================
    // tools/list tests - MCP specification compliance
    // ========================================================================

    #[test]
    fn test_list_tools_returns_six_tools() {
        let registry = ToolRegistry::new().unwrap();
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 6, "Should return exactly 6 tools");
    }

    #[test]
    fn test_list_tools_has_correct_names() {
        let registry = ToolRegistry::new().unwrap();
        let tools = registry.list_tools();
        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();

        // Document generation tools
        assert!(names.contains(&"generate_surat_tidak_mampu"));
        assert!(names.contains(&"generate_surat_kpr_belum_punya_rumah"));
        assert!(names.contains(&"generate_surat_nib_npwp"));
        // Browse posts tools
        assert!(names.contains(&"list_postings"));
        assert!(names.contains(&"get_posting_detail"));
        assert!(names.contains(&"list_categories"));
    }

    #[test]
    fn test_list_tools_has_descriptions() {
        let registry = ToolRegistry::new().unwrap();
        let tools = registry.list_tools();

        for tool in &tools {
            assert!(
                !tool.description.is_empty(),
                "Tool {} should have description",
                tool.name
            );
            assert!(
                tool.description.len() > 20,
                "Description should be meaningful"
            );
        }
    }

    #[test]
    fn test_list_tools_has_input_schema() {
        let registry = ToolRegistry::new().unwrap();
        let tools = registry.list_tools();

        for tool in &tools {
            assert!(
                tool.input_schema.is_object(),
                "inputSchema should be object for {}",
                tool.name
            );
            let schema = tool.input_schema.as_object().unwrap();
            assert!(
                schema.contains_key("type"),
                "Schema should have 'type' field"
            );
            assert!(
                schema.contains_key("properties"),
                "Schema should have 'properties' field"
            );
        }
    }

    // ========================================================================
    // tools/call tests - Unknown tool handling
    // ========================================================================

    #[test]
    fn test_call_unknown_tool_returns_error() {
        let registry = ToolRegistry::new().unwrap();
        let result = registry.call_tool("unknown_tool", None);

        assert!(result.is_error, "Unknown tool should return error");
        assert!(!result.content.is_empty(), "Error should have content");

        let error_text = &result.content[0].text.as_ref().unwrap();
        assert!(
            error_text.contains("tidak tersedia"),
            "Error should mention tool not available"
        );
    }

    // ========================================================================
    // tools/call tests - Validation error scenarios
    // ========================================================================

    #[test]
    fn test_call_sktm_with_empty_arguments_returns_validation_error() {
        let registry = ToolRegistry::new().unwrap();
        // Provide minimal structure so serde can deserialize, but with empty values
        let args = json!({
            "pengisi": {
                "nama": "",
                "nik": "",
                "ttl": "",
                "jk": "",
                "agama": "",
                "pekerjaan": "",
                "alamat": "",
                "telp": ""
            },
            "meta": {
                "kelurahan": ""
            }
        });
        let result = registry.call_tool("generate_surat_tidak_mampu", Some(args));

        assert!(result.is_error, "Empty field values should fail validation");
        let error_text = result.content[0].text.as_ref().unwrap();
        assert!(
            error_text.contains("Validasi gagal") || error_text.contains("tidak boleh kosong"),
            "Should show validation error, got: {}",
            error_text
        );
    }

    #[test]
    fn test_call_sktm_with_invalid_nik_returns_descriptive_error() {
        let registry = ToolRegistry::new().unwrap();
        let args = json!({
            "pengisi": {
                "nama": "Test User",
                "nik": "12345",  // Invalid: should be 16 digits
                "ttl": "Jakarta, 1 Januari 1990",
                "jk": "Laki-laki",
                "agama": "Islam",
                "pekerjaan": "Karyawan",
                "alamat": "Jl. Test No. 1",
                "telp": "08123456789"
            },
            "meta": {
                "kelurahan": "Cakung Barat"
            }
        });

        let result = registry.call_tool("generate_surat_tidak_mampu", Some(args));

        assert!(result.is_error);
        let error_text = result.content[0].text.as_ref().unwrap();
        assert!(
            error_text.contains("16 digit"),
            "Should mention 16 digit requirement"
        );
        assert!(
            error_text.contains("pengisi.nik"),
            "Should identify which field failed"
        );
    }

    #[test]
    fn test_call_sktm_with_invalid_gender_returns_descriptive_error() {
        let registry = ToolRegistry::new().unwrap();
        let args = json!({
            "pengisi": {
                "nama": "Test User",
                "nik": "3171234567890123",
                "ttl": "Jakarta, 1 Januari 1990",
                "jk": "Unknown",  // Invalid gender
                "agama": "Islam",
                "pekerjaan": "Karyawan",
                "alamat": "Jl. Test No. 1",
                "telp": "08123456789"
            },
            "meta": {
                "kelurahan": "Cakung Barat"
            }
        });

        let result = registry.call_tool("generate_surat_tidak_mampu", Some(args));

        assert!(result.is_error);
        let error_text = result.content[0].text.as_ref().unwrap();
        assert!(
            error_text.contains("Jenis kelamin"),
            "Should mention gender issue"
        );
        assert!(
            error_text.contains("Laki-laki") || error_text.contains("Perempuan"),
            "Should suggest valid options"
        );
    }

    #[test]
    fn test_call_kpr_with_missing_bank_returns_error() {
        let registry = ToolRegistry::new().unwrap();
        let args = json!({
            "data": {
                "nama": "Test User",
                "nik": "3171234567890123",
                "ttl": "Jakarta, 1 Januari 1990",
                "jk": "Laki-laki",
                "agama": "Islam",
                "pekerjaan": "Karyawan",
                "alamat": "Jl. Test No. 1",
                "telp": "08123456789"
            },
            "meta": {
                "kelurahan": "Cakung Barat",
                "bank_tujuan": ""  // Empty - should fail validation
            }
        });

        let result = registry.call_tool("generate_surat_kpr_belum_punya_rumah", Some(args));

        assert!(result.is_error);
        let error_text = result.content[0].text.as_ref().unwrap();
        assert!(
            error_text.contains("Bank Tujuan KPR") || error_text.contains("tidak boleh kosong"),
            "Should mention missing bank, got: {}",
            error_text
        );
    }

    #[test]
    fn test_call_nib_npwp_with_missing_business_data_returns_error() {
        let registry = ToolRegistry::new().unwrap();
        let args = json!({
            "data": {
                "nama": "Test Pelaku Usaha",
                "nik": "3171234567890123",
                "jabatan": "",
                "bidang_usaha": "",
                "kegiatan_usaha": "",
                "jenis_usaha": "",
                "alamat_usaha": ""
            }
        });

        let result = registry.call_tool("generate_surat_nib_npwp", Some(args));

        assert!(result.is_error);
        let error_text = result.content[0].text.as_ref().unwrap();
        assert!(
            error_text.contains("Validasi gagal") || error_text.contains("tidak boleh kosong"),
            "Should show validation error, got: {}",
            error_text
        );
    }

    // ========================================================================
    // tools/call tests - Multiple validation errors
    // ========================================================================

    #[test]
    fn test_validation_collects_multiple_errors() {
        let registry = ToolRegistry::new().unwrap();
        let args = json!({
            "pengisi": {
                "nama": "",           // Error 1: empty
                "nik": "invalid",     // Error 2: not 16 digits
                "ttl": "no comma",   // Error 3: invalid format
                "jk": "X",            // Error 4: invalid gender
                "agama": "",          // Error 5: empty
                "pekerjaan": "",      // Error 6: empty
                "alamat": "",         // Error 7: empty
                "telp": "123"         // Error 8: too short
            },
            "meta": {
                "kelurahan": ""       // Error 9: empty
            }
        });

        let result = registry.call_tool("generate_surat_tidak_mampu", Some(args));

        assert!(result.is_error);
        let error_text = result.content[0].text.as_ref().unwrap();
        // Should report multiple errors
        assert!(error_text.contains("kesalahan ditemukan"));
    }

    // ========================================================================
    // tools/call tests - JSON parsing error handling
    // ========================================================================

    #[test]
    fn test_call_with_malformed_arguments() {
        let registry = ToolRegistry::new().unwrap();
        // Pass wrong type - string instead of object
        let args = json!("not an object");

        let result = registry.call_tool("generate_surat_tidak_mampu", Some(args));

        assert!(result.is_error);
        let error_text = result.content[0].text.as_ref().unwrap();
        assert!(error_text.contains("Argumen tidak valid"));
    }

    // ========================================================================
    // ToolResult structure tests
    // ========================================================================

    #[test]
    fn test_tool_result_error_format() {
        let result = ToolResult::error("Test error message");

        assert!(result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert_eq!(
            result.content[0].text,
            Some("Test error message".to_string())
        );
    }

    #[test]
    fn test_content_item_text() {
        let item = ContentItem::text("Hello world");

        assert_eq!(item.content_type, "text");
        assert_eq!(item.text, Some("Hello world".to_string()));
        assert!(item.data.is_none());
        assert!(item.mime_type.is_none());
    }

    #[test]
    fn test_content_item_resource() {
        let data = b"PDF content";
        let item = ContentItem::resource(data, "application/pdf", "test.pdf");

        assert_eq!(item.content_type, "resource");
        assert!(item.text.as_ref().unwrap().contains("test.pdf"));
        assert!(item.data.is_some());
        assert_eq!(item.mime_type, Some("application/pdf".to_string()));
    }
}

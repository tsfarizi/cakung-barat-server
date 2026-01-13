//! Core content types for MCP tool responses.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Metadata for file content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    /// Original filename with extension
    pub filename: String,
    /// MIME type (e.g., "application/pdf")
    pub mime_type: String,
    /// File size in bytes
    pub size_bytes: usize,
    /// Creation timestamp in ISO8601 format
    pub created_at: String,
}

impl FileMetadata {
    /// Create new file metadata with current timestamp.
    pub fn new(
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        size_bytes: usize,
    ) -> Self {
        Self {
            filename: filename.into(),
            mime_type: mime_type.into(),
            size_bytes,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// File content with metadata and base64-encoded data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    /// File metadata
    pub metadata: FileMetadata,
    /// Base64-encoded file data
    pub data: String,
}

impl FileContent {
    /// Create new file content from raw bytes.
    pub fn new(filename: impl Into<String>, mime_type: impl Into<String>, data: &[u8]) -> Self {
        let filename = filename.into();
        let mime_type = mime_type.into();
        Self {
            metadata: FileMetadata::new(&filename, &mime_type, data.len()),
            data: BASE64.encode(data),
        }
    }

    /// Create PDF file content.
    pub fn pdf(filename: impl Into<String>, data: &[u8]) -> Self {
        Self::new(filename, "application/pdf", data)
    }

    /// Create PNG image content.
    pub fn png(filename: impl Into<String>, data: &[u8]) -> Self {
        Self::new(filename, "image/png", data)
    }

    /// Create JPEG image content.
    pub fn jpeg(filename: impl Into<String>, data: &[u8]) -> Self {
        Self::new(filename, "image/jpeg", data)
    }

    /// Create JSON file content.
    pub fn json(filename: impl Into<String>, data: &[u8]) -> Self {
        Self::new(filename, "application/json", data)
    }

    /// Create CSV file content.
    pub fn csv(filename: impl Into<String>, data: &[u8]) -> Self {
        Self::new(filename, "text/csv", data)
    }

    /// Create plain text file content.
    pub fn text_file(filename: impl Into<String>, data: &[u8]) -> Self {
        Self::new(filename, "text/plain", data)
    }

    /// Decode base64 data back to bytes.
    pub fn decode_data(&self) -> Result<Vec<u8>, base64::DecodeError> {
        BASE64.decode(&self.data)
    }
}

/// Content type enumeration for tool outputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentType {
    /// Plain text content
    Text { text: String },
    /// File content with metadata
    File { file: FileContent },
    /// Structured JSON data
    Json { data: serde_json::Value },
}

impl ContentType {
    /// Create text content.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create file content.
    pub fn file(content: FileContent) -> Self {
        Self::File { file: content }
    }

    /// Create JSON content.
    pub fn json(data: serde_json::Value) -> Self {
        Self::Json { data }
    }
}

/// Content item in tool result (MCP spec compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    /// Content type identifier
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content (for text type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded data (for resource type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// MIME type (for resource type)
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// File metadata (extended field for richer file info)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FileMetadata>,
}

impl ContentItem {
    /// Create text content item.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: Some(text.into()),
            data: None,
            mime_type: None,
            metadata: None,
        }
    }

    /// Create resource content item (legacy format for MCP compatibility).
    pub fn resource(data: &[u8], mime_type: &str, filename: &str) -> Self {
        let metadata = FileMetadata::new(filename, mime_type, data.len());
        Self {
            content_type: "resource".to_string(),
            text: Some(format!("Generated file: {}", filename)),
            data: Some(BASE64.encode(data)),
            mime_type: Some(mime_type.to_string()),
            metadata: Some(metadata),
        }
    }

    /// Create resource from FileContent.
    pub fn from_file_content(file: FileContent) -> Self {
        Self {
            content_type: "resource".to_string(),
            text: Some(format!("Generated file: {}", file.metadata.filename)),
            data: Some(file.data),
            mime_type: Some(file.metadata.mime_type.clone()),
            metadata: Some(file.metadata),
        }
    }
}

/// Result of a tool call (MCP spec compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Content items in the result
    pub content: Vec<ContentItem>,
    /// Whether this result represents an error
    #[serde(rename = "isError")]
    pub is_error: bool,
}

impl ToolResult {
    /// Create successful result.
    pub fn success(content: Vec<ContentItem>) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    /// Create error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(message)],
            is_error: true,
        }
    }

    /// Create success with text message.
    pub fn success_text(message: impl Into<String>) -> Self {
        Self::success(vec![ContentItem::text(message)])
    }

    /// Create success with file.
    pub fn success_file(file: FileContent, message: Option<String>) -> Self {
        let mut content = Vec::new();
        if let Some(msg) = message {
            content.push(ContentItem::text(msg));
        }
        content.push(ContentItem::from_file_content(file));
        Self::success(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata::new("test.pdf", "application/pdf", 1024);
        assert_eq!(metadata.filename, "test.pdf");
        assert_eq!(metadata.mime_type, "application/pdf");
        assert_eq!(metadata.size_bytes, 1024);
        assert!(!metadata.created_at.is_empty());
    }

    #[test]
    fn test_file_content_pdf() {
        let data = b"PDF content";
        let file = FileContent::pdf("test.pdf", data);

        assert_eq!(file.metadata.filename, "test.pdf");
        assert_eq!(file.metadata.mime_type, "application/pdf");
        assert_eq!(file.metadata.size_bytes, data.len());

        let decoded = file.decode_data().unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_content_item_text() {
        let item = ContentItem::text("Hello world");
        assert_eq!(item.content_type, "text");
        assert_eq!(item.text, Some("Hello world".to_string()));
        assert!(item.data.is_none());
    }

    #[test]
    fn test_content_item_resource() {
        let data = b"test data";
        let item = ContentItem::resource(data, "text/plain", "test.txt");

        assert_eq!(item.content_type, "resource");
        assert!(item.text.unwrap().contains("test.txt"));
        assert!(item.data.is_some());
        assert_eq!(item.mime_type, Some("text/plain".to_string()));
        assert!(item.metadata.is_some());
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success_text("Operation completed");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Something went wrong");
        assert!(result.is_error);
        assert_eq!(
            result.content[0].text,
            Some("Something went wrong".to_string())
        );
    }

    #[test]
    fn test_content_type_serialization() {
        let content = ContentType::text("Hello");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let parsed: ContentType = serde_json::from_str(&json).unwrap();
        assert_eq!(content, parsed);
    }
}

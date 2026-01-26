use cakung_barat_server::mcp::content::builder::{ContentBuilder, success_text, success_pdf, error};
use cakung_barat_server::mcp::content::types::{FileMetadata, FileContent, ContentItem, ToolResult, ContentType};
use cakung_barat_server::mcp::content::file::{FileExtension, detect_mime_type, detect_mime_from_bytes, generate_filename};

// Tests from src/mcp/content/builder.rs

#[test]
fn test_builder_text() {
    let result = ContentBuilder::new().text("Hello world").build();

    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
    assert_eq!(result.content[0].content_type, "text");
}

#[test]
fn test_builder_file() {
    let data = b"PDF content";
    let result = ContentBuilder::new()
        .text("File generated")
        .pdf(data, "test.pdf")
        .build();

    assert!(!result.is_error);
    assert_eq!(result.content.len(), 2);
    assert_eq!(result.content[0].content_type, "text");
    assert_eq!(result.content[1].content_type, "resource");
}

#[test]
fn test_builder_error() {
    let result = ContentBuilder::new()
        .text("Something went wrong")
        .error()
        .build();

    assert!(result.is_error);
}

#[test]
fn test_builder_multiple_files() {
    let result = ContentBuilder::new()
        .text("Multiple files generated")
        .pdf(b"pdf1", "doc1.pdf")
        .pdf(b"pdf2", "doc2.pdf")
        .png(b"image", "chart.png")
        .build();

    assert!(!result.is_error);
    assert_eq!(result.content.len(), 4);
}

#[test]
fn test_convenience_success_text() {
    let result = success_text("Operation completed");
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[test]
fn test_convenience_success_pdf() {
    let result = success_pdf(b"pdf data", "test.pdf", Some("PDF created"));
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 2);
}

#[test]
fn test_convenience_error() {
    let result = error("Failed to process");
    assert!(result.is_error);
}

// Tests from src/mcp/content/types.rs

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

// Tests from src/mcp/content/file.rs

#[test]
fn test_file_extension_from_str() {
    assert_eq!(FileExtension::from_str("pdf"), FileExtension::Pdf);
    assert_eq!(FileExtension::from_str("PDF"), FileExtension::Pdf);
    assert_eq!(FileExtension::from_str("png"), FileExtension::Png);
    assert_eq!(FileExtension::from_str("unknown"), FileExtension::Unknown);
}

#[test]
fn test_file_extension_from_filename() {
    assert_eq!(
        FileExtension::from_filename("document.pdf"),
        FileExtension::Pdf
    );
    assert_eq!(
        FileExtension::from_filename("image.PNG"),
        FileExtension::Png
    );
    assert_eq!(
        FileExtension::from_filename("noextension"),
        FileExtension::Unknown
    );
}

#[test]
fn test_mime_type() {
    assert_eq!(FileExtension::Pdf.mime_type(), "application/pdf");
    assert_eq!(FileExtension::Png.mime_type(), "image/png");
    assert_eq!(FileExtension::Json.mime_type(), "application/json");
}

#[test]
fn test_detect_mime_type() {
    assert_eq!(detect_mime_type("file.pdf"), "application/pdf");
    assert_eq!(detect_mime_type("image.jpg"), "image/jpeg");
    assert_eq!(detect_mime_type("data.json"), "application/json");
}

#[test]
fn test_detect_mime_from_bytes() {
    assert_eq!(detect_mime_from_bytes(b"%PDF-1.4"), Some("application/pdf"));
    assert_eq!(
        detect_mime_from_bytes(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]),
        Some("image/png")
    );
    assert_eq!(
        detect_mime_from_bytes(b"{\"key\": \"value\"}"),
        Some("application/json")
    );
    assert_eq!(detect_mime_from_bytes(b"ab"), None);
}

#[test]
fn test_generate_filename() {
    let filename = generate_filename("surat", "pdf");
    assert!(filename.starts_with("surat_"));
    assert!(filename.ends_with(".pdf"));
}

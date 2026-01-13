//! Builder pattern for constructing tool responses.

use super::types::{ContentItem, FileContent, ToolResult};

/// Builder for constructing ToolResult with fluent API.
#[derive(Debug, Default)]
pub struct ContentBuilder {
    items: Vec<ContentItem>,
    is_error: bool,
}

impl ContentBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a text message.
    pub fn text(mut self, message: impl Into<String>) -> Self {
        self.items.push(ContentItem::text(message));
        self
    }

    /// Add a file from raw bytes.
    pub fn file(mut self, data: &[u8], mime_type: &str, filename: &str) -> Self {
        self.items
            .push(ContentItem::resource(data, mime_type, filename));
        self
    }

    /// Add a FileContent.
    pub fn file_content(mut self, file: FileContent) -> Self {
        self.items.push(ContentItem::from_file_content(file));
        self
    }

    /// Add a PDF file.
    pub fn pdf(self, data: &[u8], filename: &str) -> Self {
        self.file(data, "application/pdf", filename)
    }

    /// Add a PNG image.
    pub fn png(self, data: &[u8], filename: &str) -> Self {
        self.file(data, "image/png", filename)
    }

    /// Add a JPEG image.
    pub fn jpeg(self, data: &[u8], filename: &str) -> Self {
        self.file(data, "image/jpeg", filename)
    }

    /// Add a JSON file.
    pub fn json_file(self, data: &[u8], filename: &str) -> Self {
        self.file(data, "application/json", filename)
    }

    /// Add a CSV file.
    pub fn csv(self, data: &[u8], filename: &str) -> Self {
        self.file(data, "text/csv", filename)
    }

    /// Mark this result as an error.
    pub fn error(mut self) -> Self {
        self.is_error = true;
        self
    }

    /// Build the final ToolResult.
    pub fn build(self) -> ToolResult {
        ToolResult {
            content: self.items,
            is_error: self.is_error,
        }
    }
}

/// Convenience function to create a success response with text.
#[allow(dead_code)]
pub fn success_text(message: impl Into<String>) -> ToolResult {
    ContentBuilder::new().text(message).build()
}

/// Convenience function to create a success response with file.
#[allow(dead_code)]
pub fn success_file(
    data: &[u8],
    mime_type: &str,
    filename: &str,
    message: Option<&str>,
) -> ToolResult {
    let mut builder = ContentBuilder::new();
    if let Some(msg) = message {
        builder = builder.text(msg);
    }
    builder.file(data, mime_type, filename).build()
}

/// Convenience function to create a success response with PDF.
#[allow(dead_code)]
pub fn success_pdf(data: &[u8], filename: &str, message: Option<&str>) -> ToolResult {
    success_file(data, "application/pdf", filename, message)
}

/// Convenience function to create an error response.
#[allow(dead_code)]
pub fn error(message: impl Into<String>) -> ToolResult {
    ContentBuilder::new().text(message).error().build()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

//! File utilities and MIME type detection.

use std::path::Path;

/// Common file extensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileExtension {
    Pdf,
    Png,
    Jpeg,
    Jpg,
    Json,
    Csv,
    Txt,
    Html,
    Xml,
    Unknown,
}

impl FileExtension {
    /// Get MIME type for this extension.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Png => "image/png",
            Self::Jpeg | Self::Jpg => "image/jpeg",
            Self::Json => "application/json",
            Self::Csv => "text/csv",
            Self::Txt => "text/plain",
            Self::Html => "text/html",
            Self::Xml => "application/xml",
            Self::Unknown => "application/octet-stream",
        }
    }

    /// Parse from extension string.
    pub fn from_str(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" => Self::Pdf,
            "png" => Self::Png,
            "jpeg" => Self::Jpeg,
            "jpg" => Self::Jpg,
            "json" => Self::Json,
            "csv" => Self::Csv,
            "txt" => Self::Txt,
            "html" | "htm" => Self::Html,
            "xml" => Self::Xml,
            _ => Self::Unknown,
        }
    }

    /// Parse from filename.
    pub fn from_filename(filename: &str) -> Self {
        Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_str)
            .unwrap_or(Self::Unknown)
    }
}

/// Detect MIME type from filename.
pub fn detect_mime_type(filename: &str) -> &'static str {
    FileExtension::from_filename(filename).mime_type()
}

/// Detect MIME type from file content magic bytes.
#[allow(dead_code)]
pub fn detect_mime_from_bytes(data: &[u8]) -> Option<&'static str> {
    if data.len() < 4 {
        return None;
    }

    // PDF magic bytes: %PDF
    if data.starts_with(b"%PDF") {
        return Some("application/pdf");
    }

    // PNG magic bytes
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some("image/png");
    }

    // JPEG magic bytes
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }

    // JSON (starts with { or [)
    if data.starts_with(b"{") || data.starts_with(b"[") {
        return Some("application/json");
    }

    // XML/HTML
    if data.starts_with(b"<?xml") || data.starts_with(b"<!DOCTYPE") || data.starts_with(b"<html") {
        return Some("text/html");
    }

    None
}

/// Generate a unique filename with timestamp.
#[allow(dead_code)]
pub fn generate_filename(prefix: &str, extension: &str) -> String {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    format!("{}_{}.{}", prefix, timestamp, extension)
}

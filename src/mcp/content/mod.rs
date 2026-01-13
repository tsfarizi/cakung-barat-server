//! Content types module for MCP tool responses.
//!
//! This module provides standardized content types for tool outputs,
//! supporting multiple file formats with consistent metadata.

mod builder;
mod file;
mod types;

pub use builder::ContentBuilder;
pub use file::{FileExtension, detect_mime_type};
pub use types::{ContentItem, ContentType, FileContent, FileMetadata, ToolResult};

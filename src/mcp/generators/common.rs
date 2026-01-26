//! Common utilities for document generation.
//!
//! Shared helpers for template rendering, date formatting, and PDF compilation.

use chrono::{Datelike, Local};
use std::path::Path;

/// Format current date in Indonesian format (e.g., "30 Desember 2025").
pub fn format_indonesian_date() -> String {
    let now = Local::now().date_naive();
    let months = [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
    ];

    let day = now.day();
    let month = months[(now.month0() as usize).min(months.len() - 1)];
    let year = now.year();

    format!("{day} {month} {year}")
}

/// Escape special characters for Typst strings.
pub fn escape_typst_string(value: &str) -> String {
    value
        .replace('\\', r"\\")
        .replace('"', r#"\""#)
        .replace('\n', r"\n")
}

/// Sanitize a string for use in filenames.
pub fn sanitize_filename(name: &str, fallback: &str) -> String {
    let mut result = String::new();
    let mut last_dash = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !last_dash && !result.is_empty() {
                result.push('-');
                last_dash = true;
            }
        }
    }

    if result.is_empty() {
        return fallback.to_string();
    }

    result.trim_matches('-').to_string()
}

/// Get the static assets directory path.
pub fn get_static_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_typst_string() {
        assert_eq!(
            escape_typst_string(r#"Hello "World""#),
            r#"Hello \"World\""#
        );
        assert_eq!(escape_typst_string("Line1\nLine2"), r"Line1\nLine2");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("John Doe", "fallback"), "john-doe");
        assert_eq!(sanitize_filename("  Spaces  ", "fallback"), "spaces");
        assert_eq!(sanitize_filename("", "fallback"), "fallback");
        assert_eq!(sanitize_filename("Test--Name", "fb"), "test-name");
    }

    #[test]
    fn test_format_indonesian_date() {
        let date = format_indonesian_date();
        // Should contain year
        assert!(date.contains("2025") || date.contains("2024") || date.contains("2026"));
    }
}
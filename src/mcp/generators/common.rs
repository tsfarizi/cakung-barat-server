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
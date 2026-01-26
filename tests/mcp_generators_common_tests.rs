use cakung_barat_server::mcp::generators::common::{escape_typst_string, sanitize_filename, format_indonesian_date};

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


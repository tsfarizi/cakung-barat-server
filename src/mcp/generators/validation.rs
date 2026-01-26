//! Input validation module for document generators.
//!
//! Provides clear, descriptive validation errors that are easy to understand
//! for both AI (MCP server) and human users.

use std::fmt;

/// Validation error with detailed, user-friendly messages.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The field that failed validation
    pub field: String,
    /// Human-readable error message in Indonesian
    pub message: String,
    /// Suggestion for how to fix the error
    pub suggestion: Option<String>,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Create error for empty required field
    pub fn empty_field(field: &str, label: &str) -> Self {
        Self::new(field, format!("{} tidak boleh kosong", label)).with_suggestion(format!(
            "Mohon isi {} dengan data yang valid",
            label.to_lowercase()
        ))
    }

    /// Create error for invalid NIK format
    pub fn invalid_nik(field: &str) -> Self {
        Self::new(field, "NIK harus terdiri dari 16 digit angka")
            .with_suggestion("Periksa kembali NIK sesuai KTP, contoh: 3171234567890123")
    }

    /// Create error for invalid phone number
    pub fn invalid_phone(field: &str) -> Self {
        Self::new(field, "Nomor telepon tidak valid")
            .with_suggestion("Gunakan format nomor telepon Indonesia, contoh: 08123456789")
    }

    /// Create error for invalid date format
    pub fn invalid_date_format(field: &str, value: &str) -> Self {
        Self::new(field, format!("Format tanggal '{}' tidak valid", value)).with_suggestion(
            "Gunakan format: Tempat, DD Bulan YYYY (contoh: Jakarta, 15 Januari 1990)",
        )
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.field, self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, ". {}", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Collection of validation errors with formatted output.
#[derive(Debug, Default)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Get formatted error message suitable for MCP response
    pub fn to_mcp_message(&self) -> String {
        if self.errors.is_empty() {
            return String::new();
        }

        let mut parts = vec![format!(
            "❌ Validasi gagal: {} kesalahan ditemukan\n",
            self.errors.len()
        )];

        for (i, error) in self.errors.iter().enumerate() {
            parts.push(format!("{}. {}", i + 1, error));
        }

        parts.push(String::new());
        parts.push("Mohon perbaiki data di atas dan coba lagi.".to_string());

        parts.join("\n")
    }

    /// Convert to Result - Ok if no errors, Err with formatted message if errors exist
    pub fn into_result(self) -> Result<(), String> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(self.to_mcp_message())
        }
    }
}

// ============================================================================
// Validation functions
// ============================================================================

/// Validate that a string is not empty after trimming
pub fn validate_required(value: &str, field: &str, label: &str, errors: &mut ValidationErrors) {
    if value.trim().is_empty() {
        errors.add(ValidationError::empty_field(field, label));
    }
}

/// Validate NIK format (16 digits)
pub fn validate_nik(value: &str, field: &str, errors: &mut ValidationErrors) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        errors.add(ValidationError::empty_field(field, "NIK"));
        return;
    }

    if trimmed.len() != 16 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        errors.add(ValidationError::invalid_nik(field));
    }
}

/// Validate NIK format (16 digits) - optional, only validate if provided
pub fn validate_nik_optional(value: &str, field: &str, errors: &mut ValidationErrors) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return; // Optional, so empty is OK
    }

    if trimmed.len() != 16 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        errors.add(ValidationError::invalid_nik(field));
    }
}

/// Validate phone number format
pub fn validate_phone(value: &str, field: &str, errors: &mut ValidationErrors) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        errors.add(ValidationError::empty_field(field, "Nomor Telepon"));
        return;
    }

    // Remove common separators
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    // Indonesian phone numbers should be 10-13 digits
    if digits.len() < 10 || digits.len() > 13 {
        errors.add(ValidationError::invalid_phone(field));
    }
}

/// Validate tempat tanggal lahir format
pub fn validate_ttl(value: &str, field: &str, errors: &mut ValidationErrors) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        errors.add(ValidationError::empty_field(field, "Tempat, Tanggal Lahir"));
        return;
    }

    // Should contain a comma separating place and date
    if !trimmed.contains(',') {
        errors.add(ValidationError::invalid_date_format(field, trimmed));
    }
}

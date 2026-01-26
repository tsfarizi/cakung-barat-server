use cakung_barat_server::mcp::generators::validation::{ValidationErrors, ValidationError, validate_required, validate_nik};

#[test]
fn test_validate_required_empty() {
    let mut errors = ValidationErrors::new();
    validate_required("", "nama", "Nama Lengkap", &mut errors);
    assert_eq!(errors.len(), 1);
    assert!(
        errors
            .to_mcp_message()
            .contains("Nama Lengkap tidak boleh kosong")
    );
}

#[test]
fn test_validate_required_valid() {
    let mut errors = ValidationErrors::new();
    validate_required("John Doe", "nama", "Nama Lengkap", &mut errors);
    assert!(errors.is_empty());
}

#[test]
fn test_validate_nik_valid() {
    let mut errors = ValidationErrors::new();
    validate_nik("3171234567890123", "nik", &mut errors);
    assert!(errors.is_empty());
}

#[test]
fn test_validate_nik_invalid_length() {
    let mut errors = ValidationErrors::new();
    validate_nik("123456", "nik", &mut errors);
    assert_eq!(errors.len(), 1);
    assert!(errors.to_mcp_message().contains("16 digit"));
}

#[test]
fn test_validation_errors_message() {
    let mut errors = ValidationErrors::new();
    errors.add(ValidationError::empty_field("nama", "Nama"));
    errors.add(ValidationError::invalid_nik("nik"));

    let msg = errors.to_mcp_message();
    assert!(msg.contains("2 kesalahan"));
    assert!(msg.contains("Nama tidak boleh kosong"));
    assert!(msg.contains("16 digit"));
}

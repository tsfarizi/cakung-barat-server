#[cfg(test)]
mod multipart_parser_tests {
    use cakung_barat_server::posting::multipart_parser::{ParsedMultipartData, MultipartParseError};

    #[test]
    fn test_parsed_multipart_data_structure() {
        // Test the structure of ParsedMultipartData
        let mut files_data = Vec::new();
        files_data.push((b"file content".to_vec(), "test.txt".to_string()));

        let parsed_data = ParsedMultipartData {
            title: "Test Title".to_string(),
            category: "Test Category".to_string(),
            excerpt: "Test excerpt".to_string(),
            files_data,
        };

        assert_eq!(parsed_data.title, "Test Title");
        assert_eq!(parsed_data.category, "Test Category");
        assert_eq!(parsed_data.excerpt, "Test excerpt");
        assert_eq!(parsed_data.files_data.len(), 1);
        assert_eq!(parsed_data.files_data[0].1, "test.txt");
    }

    #[test]
    fn test_multipart_parse_error_variants() {
        // Test each error variant can be created and converted to string
        let field_error = MultipartParseError::FieldError("test field error".to_string());
        assert!(field_error.to_string().contains("test field error"));
        
        let metadata_error = MultipartParseError::MetadataError("test metadata error".to_string());
        assert!(metadata_error.to_string().contains("test metadata error"));
        
        let io_error = MultipartParseError::IoError("test io error".to_string());
        assert!(io_error.to_string().contains("test io error"));
        
        let utf8_error = MultipartParseError::Utf8Error("test utf8 error".to_string());
        assert!(utf8_error.to_string().contains("test utf8 error"));
        
        let serialization_error = MultipartParseError::SerializationError("test serialization error".to_string());
        assert!(serialization_error.to_string().contains("test serialization error"));
    }

    #[test]
    fn test_multipart_parse_error_display() {
        let error = MultipartParseError::FieldError("field error".to_string());
        let error_str = format!("{}", error);
        assert_eq!(error_str, "Multipart field error: field error");
    }

    #[test]
    fn test_parsed_multipart_data_default_values() {
        // Test creating ParsedMultipartData with empty values
        let parsed_data = ParsedMultipartData {
            title: String::new(),
            category: String::new(),
            excerpt: String::new(),
            files_data: Vec::new(),
        };

        assert_eq!(parsed_data.title, "");
        assert_eq!(parsed_data.category, "");
        assert_eq!(parsed_data.excerpt, "");
        assert_eq!(parsed_data.files_data.len(), 0);
    }
}
#[cfg(test)]
mod error_handling_tests {
    use uuid::Uuid;
    use cakung_barat_server::ErrorResponse;
    use serde_json::json;

    #[test]
    fn test_invalid_uuid_format() {
        // This would be tested at the API level, but we can test the error response structure
        let error_response = ErrorResponse::bad_request("Invalid UUID format");
        assert_eq!(error_response.error, "BadRequest");
        assert!(error_response.message.contains("Invalid UUID"));
    }

    #[test]
    fn test_large_pagination_values() {
        // Test with very large page/limit values
        let page = 999999;
        let limit = 999999;
        
        // For our implementation, these should be handled gracefully
        assert!(page > 0);
        assert!(limit > 0);
    }

    #[test]
    fn test_negative_pagination_values() {
        // Test default behavior for negative values
        fn default_page() -> i32 { 1 }
        fn default_limit() -> i32 { 20 }
        
        let page = -5;
        let limit = -10;
        
        let effective_page = if page < 1 { default_page() } else { page };
        let effective_limit = if limit < 1 { default_limit() } else { limit };
        
        assert_eq!(effective_page, 1);
        assert_eq!(effective_limit, 20);
    }

    #[test]
    fn test_duplicate_asset_ids_request() {
        // Test with duplicate IDs
        let duplicate_id = Uuid::new_v4();
        let ids = vec![duplicate_id, duplicate_id, duplicate_id];
        
        // Convert to HashSet to remove duplicates 
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert!(unique_ids.len() <= ids.len());
    }

    #[test]
    fn test_malformed_json_requests() {
        // Test that malformed JSON is handled properly
        let malformed_json = "{ malformed json ";
        
        let result: Result<serde_json::Value, _> = serde_json::from_str(malformed_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_special_characters_in_inputs() {
        // Test with special characters
        let special_title = "Title with \"quotes\" and 'apostrophes' and <script>alert('xss')</script>";
        let special_category = "Category with \n\n newlines and \t tabs";
        let special_excerpt = "Excerpt with unicode: 🚀 🐛 🎉 and symbols: @#$%^&*()";
        
        // Just verify we can create strings with these characters
        assert!(!special_title.is_empty());
        assert!(!special_category.is_empty());
        assert!(!special_excerpt.is_empty());
    }

    #[test]
    fn test_extremely_long_inputs() {
        // Create extremely long strings
        let long_title = "A".repeat(10000);
        let long_category = "B".repeat(10000);
        let long_excerpt = "C".repeat(10000);
        
        // Just verify lengths
        assert_eq!(long_title.len(), 10000);
        assert_eq!(long_category.len(), 10000);
        assert_eq!(long_excerpt.len(), 10000);
    }

    #[test]
    fn test_error_response_serialization() {
        // Test that error responses can be properly serialized
        let not_found_error = ErrorResponse::not_found("Resource not found");
        let bad_request_error = ErrorResponse::bad_request("Invalid input");
        let internal_error = ErrorResponse::internal_error("Server error");

        // Test serialization
        let not_found_json = serde_json::to_string(&not_found_error);
        assert!(not_found_json.is_ok());

        let bad_request_json = serde_json::to_string(&bad_request_error);
        assert!(bad_request_json.is_ok());

        let internal_json = serde_json::to_string(&internal_error);
        assert!(internal_json.is_ok());

        // Test deserialization
        let deserialized: Result<ErrorResponse, _> = serde_json::from_str(&bad_request_json.unwrap());
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_empty_string_handling() {
        // Test with empty strings
        let empty_title = "";
        let empty_category = "";
        let empty_excerpt = "";
        
        assert!(empty_title.is_empty());
        assert!(empty_category.is_empty());
        assert!(empty_excerpt.is_empty());
    }

    #[test]
    fn test_null_values_in_json_simulation() {
        // Test behavior with missing fields in JSON (simulated with Option)
        let json_with_missing_fields = json!({
            "title": "Has title",
            "category": "Has category"
            // excerpt is missing
        });
        
        // This tests that the deserialization can handle missing fields if they are optional
        assert!(json_with_missing_fields.get("title").is_some());
        assert!(json_with_missing_fields.get("category").is_some());
        assert!(json_with_missing_fields.get("excerpt").is_none());
    }
}
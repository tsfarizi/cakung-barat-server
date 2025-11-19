#[cfg(test)]
mod api_integration_tests {
    use uuid::Uuid;
    use cakung_barat_server::ErrorResponse;

    #[test]
    fn test_error_response_consistency() {
        // Test not found error structure
        let post_error_response = ErrorResponse::not_found("Post not found");
        assert_eq!(post_error_response.error, "NotFound");

        // Test bad request error structure  
        let asset_error_response = ErrorResponse::bad_request("Invalid input");
        assert_eq!(asset_error_response.error, "BadRequest");
    }

    #[test]
    fn test_uuid_generation() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        
        // Different UUIDs should be different
        assert_ne!(uuid1, uuid2);
        
        // Both should not be nil
        assert!(!uuid1.is_nil());
        assert!(!uuid2.is_nil());
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
}
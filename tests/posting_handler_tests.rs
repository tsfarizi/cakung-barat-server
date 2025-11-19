#[cfg(test)]
mod posting_handler_tests {
    use cakung_barat_server::{
        posting::models::{CreatePostingRequest},
        ErrorResponse,
    };

    #[test]
    fn test_posting_request_models() {
        // Test CreatePostingRequest model
        let create_request = CreatePostingRequest {
            title: "Test Title".to_string(),
            category: "Test Category".to_string(),
            excerpt: "Test excerpt".to_string(),
        };

        assert_eq!(create_request.title, "Test Title");
        assert_eq!(create_request.category, "Test Category");
        assert_eq!(create_request.excerpt, "Test excerpt");
    }

    #[test]
    fn test_error_response_struct() {
        // Test that error responses are properly formatted
        let error_response = ErrorResponse::new("TestError", "Test message");
        assert_eq!(error_response.error, "TestError");
        assert_eq!(error_response.message, "Test message");
        assert!(!error_response.timestamp.is_empty());
    }
}
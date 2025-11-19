#[cfg(test)]
mod asset_handler_tests {
    use uuid::Uuid;
    use cakung_barat_server::ErrorResponse;
    use cakung_barat_server::asset::handlers::GetAssetsByIdsRequest;

    #[test]
    fn test_error_response_struct() {
        // Test that error responses are properly formatted
        let error_response = ErrorResponse::new("TestError", "Test message");
        assert_eq!(error_response.error, "TestError");
        assert_eq!(error_response.message, "Test message");
        assert!(!error_response.timestamp.is_empty());
    }

    #[test]
    fn test_get_assets_by_ids_request() {
        // Test the GetAssetsByIdsRequest struct directly
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let request = GetAssetsByIdsRequest { ids: ids.clone() };

        assert_eq!(request.ids.len(), 2);
        assert_eq!(request.ids, ids);
    }

    #[test]
    fn test_get_assets_by_ids_request_empty() {
        let request = GetAssetsByIdsRequest { ids: vec![] };
        assert_eq!(request.ids.len(), 0);
    }
}
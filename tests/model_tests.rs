#[cfg(test)]
mod model_tests {
    use cakung_barat_server::asset::models::Asset;
    use cakung_barat_server::posting::models::{Post, PostWithAssets, CreatePostingRequest, UpdatePostingRequest};
    use cakung_barat_server::storage::FolderContent;
    use cakung_barat_server::ErrorResponse;
    use uuid::Uuid;
    use chrono::{NaiveDate, Utc};

    #[test]
    fn test_asset_creation() {
        let name = "Test Asset".to_string();
        let filename = "test_file.jpg".to_string();
        let url = "/assets/serve/test_file.jpg".to_string();
        let description = Some("A test asset".to_string());

        let asset = Asset::new(name.clone(), filename.clone(), url.clone(), description.clone());

        // Check that the asset was created with the correct values
        assert_eq!(asset.name, name);
        assert_eq!(asset.filename, filename);
        assert_eq!(asset.url, url);
        assert_eq!(asset.description, description);

        // Check that the ID is not nil (ensuring Uuid::new_v4() worked)
        assert!(!asset.id.is_nil());

        // Check that timestamps are set
        assert!(asset.created_at.is_some());
        assert!(asset.updated_at.is_some());
    }

    #[test]
    fn test_asset_creation_without_description() {
        let name = "Test Asset".to_string();
        let filename = "test_file.jpg".to_string();
        let url = "/assets/serve/test_file.jpg".to_string();
        let description = None;

        let asset = Asset::new(name.clone(), filename.clone(), url.clone(), description);

        assert_eq!(asset.name, name);
        assert_eq!(asset.filename, filename);
        assert_eq!(asset.url, url);
        assert_eq!(asset.description, None);

        assert!(!asset.id.is_nil());
        assert!(asset.created_at.is_some());
        assert!(asset.updated_at.is_some());
    }

    #[test]
    fn test_post_creation() {
        let title = "Test Title".to_string();
        let category = "Test Category".to_string();
        let excerpt = "Test excerpt".to_string();
        let folder_id = Some("posts/some-folder-id".to_string());

        let post = Post::new(title.clone(), category.clone(), excerpt.clone(), folder_id.clone());

        // Check that the post was created with the correct values
        assert_eq!(post.title, title);
        assert_eq!(post.category, category);
        assert_eq!(post.excerpt, excerpt);
        assert_eq!(post.folder_id, folder_id);

        // Check that the ID is not nil (ensuring Uuid::new_v4() worked)
        assert!(!post.id.is_nil());

        // Check that dates and timestamps are set
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }

    #[test]
    fn test_post_creation_without_folder_id() {
        let title = "Test Title".to_string();
        let category = "Test Category".to_string();
        let excerpt = "Test excerpt".to_string();
        let folder_id = None;

        let post = Post::new(title.clone(), category.clone(), excerpt.clone(), folder_id);

        assert_eq!(post.title, title);
        assert_eq!(post.category, category);
        assert_eq!(post.excerpt, excerpt);
        assert_eq!(post.folder_id, None);

        assert!(!post.id.is_nil());
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }

    #[test]
    fn test_post_with_assets_creation() {
        let post_with_assets = PostWithAssets {
            id: Uuid::new_v4(),
            title: "Test Post With Assets".to_string(),
            category: "Test Category".to_string(),
            date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            excerpt: "Test excerpt".to_string(),
            folder_id: Some("posts/test".to_string()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            asset_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };

        assert!(!post_with_assets.id.is_nil());
        assert_eq!(post_with_assets.title, "Test Post With Assets");
        assert_eq!(post_with_assets.asset_ids.len(), 2);
    }

    #[test]
    fn test_create_posting_request_serialization() {
        let request = CreatePostingRequest {
            title: "Test Title".to_string(),
            category: "Test Category".to_string(),
            excerpt: "Test Excerpt".to_string(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("Test Title"));
        assert!(serialized.contains("Test Category"));
        assert!(serialized.contains("Test Excerpt"));

        // Test deserialization
        let deserialized: CreatePostingRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.title, "Test Title");
        assert_eq!(deserialized.category, "Test Category");
        assert_eq!(deserialized.excerpt, "Test Excerpt");
    }

    #[test]
    fn test_update_posting_request_partial_updates() {
        // Test with some fields set
        let partial_request = UpdatePostingRequest {
            title: Some("Updated Title".to_string()),
            category: None, // This should not update
            excerpt: Some("Updated Excerpt".to_string()),
            folder_id: None, // This should not update
        };

        assert_eq!(partial_request.title, Some("Updated Title".to_string()));
        assert!(partial_request.category.is_none());
        assert_eq!(partial_request.excerpt, Some("Updated Excerpt".to_string()));
        assert!(partial_request.folder_id.is_none());

        // Test with all fields set to None (no updates)
        let empty_request = UpdatePostingRequest {
            title: None,
            category: None,
            excerpt: None,
            folder_id: None,
        };

        assert!(empty_request.title.is_none());
        assert!(empty_request.category.is_none());
        assert!(empty_request.excerpt.is_none());
        assert!(empty_request.folder_id.is_none());
    }

    #[test]
    fn test_folder_content_creation() {
        let folder_content = FolderContent {
            name: "test_file.jpg".to_string(),
            is_file: true,
            size: Some(1024),
        };

        assert_eq!(folder_content.name, "test_file.jpg");
        assert_eq!(folder_content.is_file, true);
        assert_eq!(folder_content.size, Some(1024));

        // Test with no size
        let folder_content_no_size = FolderContent {
            name: "test_folder".to_string(),
            is_file: false,
            size: None,
        };

        assert_eq!(folder_content_no_size.name, "test_folder");
        assert_eq!(folder_content_no_size.is_file, false);
        assert_eq!(folder_content_no_size.size, None);
    }

    #[test]
    fn test_error_response_creation() {
        // Test not found error
        let not_found_error = ErrorResponse::not_found("Item not found");
        assert_eq!(not_found_error.error, "NotFound");
        assert_eq!(not_found_error.message, "Item not found");
        assert!(!not_found_error.timestamp.is_empty());

        // Test bad request error
        let bad_request_error = ErrorResponse::bad_request("Invalid input");
        assert_eq!(bad_request_error.error, "BadRequest");
        assert_eq!(bad_request_error.message, "Invalid input");

        // Test internal error
        let internal_error = ErrorResponse::internal_error("Server error");
        assert_eq!(internal_error.error, "InternalServerError");
        assert_eq!(internal_error.message, "Server error");

        // Test generic error creation
        let generic_error = ErrorResponse::new("CustomError", "Custom message");
        assert_eq!(generic_error.error, "CustomError");
        assert_eq!(generic_error.message, "Custom message");
    }

    #[test]
    fn test_error_response_serialization() {
        let error_response = ErrorResponse::new("TestError", "Test message");
        let serialized = serde_json::to_string(&error_response).unwrap();

        assert!(serialized.contains("TestError"));
        assert!(serialized.contains("Test message"));
        assert!(serialized.contains("timestamp"));

        // Test deserialization
        let deserialized: ErrorResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.error, "TestError");
        assert_eq!(deserialized.message, "Test message");
    }

    #[test]
    fn test_post_dates() {
        let post = Post::new(
            "Test".to_string(),
            "Category".to_string(),
            "Excerpt".to_string(),
            None,
        );

        // Check that the date is today's date (or close to it)
        let now = chrono::Local::now().date_naive();
        assert_eq!(post.date, now);
    }

    #[test]
    fn test_asset_equality() {
        let asset1 = Asset::new(
            "Test Asset".to_string(),
            "test_file.jpg".to_string(),
            "/assets/serve/test_file.jpg".to_string(),
            Some("A test asset".to_string())
        );

        let asset2 = Asset {
            id: asset1.id,
            name: "Test Asset".to_string(),
            filename: "test_file.jpg".to_string(),
            url: "/assets/serve/test_file.jpg".to_string(),
            description: Some("A test asset".to_string()),
            created_at: asset1.created_at,
            updated_at: asset1.updated_at,
        };

        // Assets with same ID should be considered equal in terms of core data
        assert_eq!(asset1.name, asset2.name);
        assert_eq!(asset1.filename, asset2.filename);
        assert_eq!(asset1.url, asset2.url);
        assert_eq!(asset1.description, asset2.description);
    }

    #[test]
    fn test_uuid_generation_for_different_assets() {
        let asset1 = Asset::new(
            "Test Asset 1".to_string(),
            "test_file1.jpg".to_string(),
            "/assets/serve/test_file1.jpg".to_string(),
            Some("First test asset".to_string())
        );

        let asset2 = Asset::new(
            "Test Asset 2".to_string(),
            "test_file2.jpg".to_string(),
            "/assets/serve/test_file2.jpg".to_string(),
            Some("Second test asset".to_string())
        );

        // Different assets should have different UUIDs
        assert_ne!(asset1.id, asset2.id);
        
        // Both UUIDs should be valid (not nil)
        assert!(!asset1.id.is_nil());
        assert!(!asset2.id.is_nil());
    }
}
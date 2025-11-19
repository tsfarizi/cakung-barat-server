#[cfg(test)]
mod database_integration_tests {
    use cakung_barat_server::db::AppState;
    use cakung_barat_server::asset::models::Asset;
    use cakung_barat_server::posting::models::{Post, PostWithAssets};
    use cakung_barat_server::storage::ObjectStorage;
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::NaiveDate;
    use sqlx::PgPool;
    use tokio;

    // Test helper functions moved into this module
    async fn setup_test_db() -> PgPool {
        // Check if we have a database URL available
        let database_url = if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
            url
        } else if let Ok(url) = std::env::var("SUPABASE_DATABASE_URL") {
            url
        } else {
            // If no database URL is set, return a fake connection that won't be used
            // Actually, we should create a temporary database for testing
            // For now, let's make a simple check and allow the test to fail gracefully
            "postgres://test_user:test_password@localhost/test_cakung_barat".to_string()
        };

        // Connect to the database
        let pool = match PgPool::connect(&database_url).await {
            Ok(pool) => {
                // Ensure the uuid-ossp extension is available
                sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";").execute(&pool).await.unwrap();

                // Run the schema to ensure test tables exist
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS assets (
                        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                        name TEXT NOT NULL,
                        filename TEXT NOT NULL,
                        url TEXT NOT NULL,
                        description TEXT,
                        created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                        updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                    );"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS posts (
                        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                        title TEXT NOT NULL,
                        category TEXT NOT NULL,
                        date DATE NOT NULL,
                        excerpt TEXT NOT NULL,
                        folder_id TEXT,
                        created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                        updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                    );"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS folders (
                        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                        name TEXT UNIQUE NOT NULL,
                        created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                    );"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS asset_folders (
                        asset_id UUID REFERENCES assets(id) ON DELETE CASCADE,
                        folder_id UUID REFERENCES folders(id) ON DELETE CASCADE,
                        created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                        PRIMARY KEY (asset_id, folder_id)
                    );"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_assets_filename ON assets(filename);"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_asset_folders_asset_id ON asset_folders(asset_id);"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_asset_folders_folder_id ON asset_folders(folder_id);"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE OR REPLACE FUNCTION update_updated_at_column()
                    RETURNS TRIGGER AS $$
                    BEGIN
                        NEW.updated_at = NOW();
                        RETURN NEW;
                    END;
                    $$ language 'plpgsql';"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE TRIGGER IF NOT EXISTS update_assets_updated_at
                        BEFORE UPDATE ON assets
                        FOR EACH ROW
                        EXECUTE FUNCTION update_updated_at_column();"
                ).execute(&pool).await.unwrap();

                sqlx::query(
                    "CREATE TRIGGER IF NOT EXISTS update_posts_updated_at
                        BEFORE UPDATE ON posts
                        FOR EACH ROW
                        EXECUTE FUNCTION update_updated_at_column();"
                ).execute(&pool).await.unwrap();

                pool
            },
            Err(e) => {
                // For the purpose of this example, we'll panic to show that real database connection is needed
                // In a real project, you might want to have a different approach for CI environments
                panic!("Failed to connect to test database: {}. Make sure you have a PostgreSQL database running or set the TEST_DATABASE_URL environment variable.", e);
            }
        };

        pool
    }

    // Helper to clean up test data
    async fn cleanup_test_data(pool: &PgPool) {
        // Truncate all tables that might have been created during tests
        let _ = sqlx::query!("TRUNCATE TABLE posts, assets, folders, asset_folders RESTART IDENTITY CASCADE")
            .execute(pool)
            .await;
    }

    // Mock implementation of ObjectStorage for testing
    struct MockObjectStorage {
        files: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
    }

    impl MockObjectStorage {
        fn new() -> Self {
            Self {
                files: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl ObjectStorage for MockObjectStorage {
        async fn upload_file(&self, filename: &str, file_data: &[u8]) -> Result<(), String> {
            let mut files = self.files.lock().await;
            files.insert(filename.to_string(), file_data.to_vec());
            Ok(())
        }

        async fn delete_file(&self, filename: &str) -> Result<(), String> {
            let mut files = self.files.lock().await;
            files.remove(filename);
            Ok(())
        }

        async fn create_folder(&self, _folder_name: &str) -> Result<(), String> {
            // No-op for mock implementation
            Ok(())
        }

        async fn list_folder_contents(&self, _folder_name: &str) -> Result<Vec<cakung_barat_server::storage::FolderContent>, String> {
            // Return empty list for mock implementation
            Ok(Vec::new())
        }

        fn get_asset_url(&self, filename: &str) -> String {
            format!("http://test.example.com/{}", filename)
        }
    }

    #[tokio::test]
    async fn test_asset_crud_operations_with_cleanup() {
        // Setup test database
        let pool = setup_test_db().await;
        let mock_storage = Arc::new(MockObjectStorage::new());
        let app_state = AppState::new_with_pool_and_storage(pool.clone(), mock_storage).await.unwrap();

        // Create a test asset
        let test_asset = Asset::new(
            "Test Asset".to_string(),
            "test_file.jpg".to_string(),
            "/assets/serve/test_file.jpg".to_string(),
            Some("A test asset description".to_string())
        );

        // Test CREATE (Insert)
        let insert_result = app_state.insert_asset(&test_asset).await;
        assert!(insert_result.is_ok());

        // Test READ (Get by ID)
        let retrieved_asset = app_state.get_asset_by_id(&test_asset.id).await.unwrap();
        assert!(retrieved_asset.is_some());
        assert_eq!(retrieved_asset.unwrap().name, "Test Asset");

        // Test READ (Get all)
        let all_assets = app_state.get_all_assets().await.unwrap();
        assert!(!all_assets.is_empty());

        // Test UPDATE (For assets, this is done by re-inserting with same ID)
        let updated_asset = Asset {
            id: test_asset.id,
            name: "Updated Test Asset".to_string(),
            filename: "test_file.jpg".to_string(),
            url: "/assets/serve/test_file.jpg".to_string(),
            description: Some("Updated description".to_string()),
            created_at: test_asset.created_at,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_result = app_state.insert_asset(&updated_asset).await;
        assert!(update_result.is_ok());

        // Verify update worked
        let updated_retrieved = app_state.get_asset_by_id(&test_asset.id).await.unwrap();
        assert_eq!(updated_retrieved.unwrap().name, "Updated Test Asset");

        // Test DELETE
        let delete_result = app_state.delete_asset(&test_asset.id).await;
        assert!(delete_result.is_ok());

        // Verify deletion
        let deleted_asset = app_state.get_asset_by_id(&test_asset.id).await.unwrap();
        assert!(deleted_asset.is_none());

        // Cleanup test data
        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_post_crud_operations_with_cleanup() {
        // Setup test database
        let pool = setup_test_db().await;
        let mock_storage = Arc::new(MockObjectStorage::new());
        let app_state = AppState::new_with_pool_and_storage(pool.clone(), mock_storage).await.unwrap();

        // Create a test post
        let test_post = Post {
            id: Uuid::new_v4(),
            title: "Test Post".to_string(),
            category: "Test Category".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            excerpt: "Test excerpt".to_string(),
            folder_id: Some("test_folder".to_string()),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        // Test CREATE (Insert)
        let insert_result = app_state.insert_post(&test_post).await;
        assert!(insert_result.is_ok());

        // Test READ (Get by ID)
        let retrieved_post = app_state.get_post_by_id(&test_post.id).await.unwrap();
        assert!(retrieved_post.is_some());
        assert_eq!(retrieved_post.unwrap().title, "Test Post");

        // Test READ (Get all)
        let all_posts = app_state.get_all_posts().await.unwrap();
        assert!(!all_posts.is_empty());

        // Test UPDATE
        let updated_post = Post {
            id: test_post.id,
            title: "Updated Test Post".to_string(),
            category: "Updated Category".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            excerpt: "Updated excerpt".to_string(),
            folder_id: Some("updated_folder".to_string()),
            created_at: test_post.created_at,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_result = app_state.update_post(&updated_post).await;
        assert!(update_result.is_ok());

        // Verify update worked
        let updated_retrieved = app_state.get_post_by_id(&test_post.id).await.unwrap();
        assert_eq!(updated_retrieved.unwrap().title, "Updated Test Post");

        // Test DELETE
        let delete_result = app_state.delete_post(&test_post.id).await;
        assert!(delete_result.is_ok());

        // Verify deletion
        let deleted_post = app_state.get_post_by_id(&test_post.id).await.unwrap();
        assert!(deleted_post.is_none());

        // Cleanup test data
        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_folder_operations_with_cleanup() {
        // Setup test database
        let pool = setup_test_db().await;
        let mock_storage = Arc::new(MockObjectStorage::new());
        let app_state = AppState::new_with_pool_and_storage(pool.clone(), mock_storage).await.unwrap();

        // Create some test assets to work with folders
        let asset1 = Asset::new(
            "Asset 1".to_string(),
            "asset1.jpg".to_string(),
            "/assets/serve/asset1.jpg".to_string(),
            None
        );
        let asset2 = Asset::new(
            "Asset 2".to_string(),
            "asset2.jpg".to_string(),
            "/assets/serve/asset2.jpg".to_string(),
            None
        );

        // Insert assets
        app_state.insert_asset(&asset1).await.unwrap();
        app_state.insert_asset(&asset2).await.unwrap();

        let folder_name = "test_folder_integration";
        let asset_ids = vec![asset1.id, asset2.id];

        // Test folder creation and asset association
        let insert_result = app_state.insert_folder_contents(folder_name, &asset_ids).await;
        assert!(insert_result.is_ok());

        // Test reading folder contents
        let folder_contents = app_state.get_folder_contents(folder_name).await.unwrap();
        assert!(folder_contents.is_some());
        assert_eq!(folder_contents.unwrap().len(), 2);

        // Cleanup test data
        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_post_with_assets_operations_with_cleanup() {
        // Setup test database
        let pool = setup_test_db().await;
        let mock_storage = Arc::new(MockObjectStorage::new());
        let app_state = AppState::new_with_pool_and_storage(pool.clone(), mock_storage).await.unwrap();

        // Create test assets
        let asset1 = Asset::new(
            "Post Asset 1".to_string(),
            "post_asset1.jpg".to_string(),
            "/assets/serve/post_asset1.jpg".to_string(),
            None
        );
        let asset2 = Asset::new(
            "Post Asset 2".to_string(),
            "post_asset2.jpg".to_string(),
            "/assets/serve/post_asset2.jpg".to_string(),
            None
        );

        app_state.insert_asset(&asset1).await.unwrap();
        app_state.insert_asset(&asset2).await.unwrap();

        // Create a test post
        let test_post = Post {
            id: Uuid::new_v4(),
            title: "Test Post With Assets".to_string(),
            category: "Test Category".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            excerpt: "Test excerpt with assets".to_string(),
            folder_id: Some(format!("posts/{}", Uuid::new_v4())),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        app_state.insert_post(&test_post).await.unwrap();

        // Create PostWithAssets
        let post_with_assets = PostWithAssets {
            id: test_post.id,
            title: test_post.title.clone(),
            category: test_post.category.clone(),
            date: test_post.date,
            excerpt: test_post.excerpt.clone(),
            folder_id: test_post.folder_id.clone(),
            created_at: test_post.created_at,
            updated_at: test_post.updated_at,
            asset_ids: vec![asset1.id, asset2.id],
        };

        // Test upsert with assets
        let upsert_result = app_state.upsert_posting_with_assets(&post_with_assets).await;
        assert!(upsert_result.is_ok());

        // Test get post with assets
        let retrieved_post_with_assets = app_state.get_posting_by_id_with_assets(&test_post.id).await.unwrap();
        assert!(retrieved_post_with_assets.is_some());
        let retrieved = retrieved_post_with_assets.unwrap();
        assert_eq!(retrieved.title, "Test Post With Assets");
        assert_eq!(retrieved.asset_ids.len(), 2);

        // Cleanup test data
        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_multiple_operations_with_cleanup() {
        // Setup test database
        let pool = setup_test_db().await;
        let mock_storage = Arc::new(MockObjectStorage::new());
        let app_state = AppState::new_with_pool_and_storage(pool.clone(), mock_storage).await.unwrap();

        // Test multiple CRUD operations in one test
        // Create multiple assets
        let asset1 = Asset::new(
            "Batch Test Asset 1".to_string(),
            "batch_asset1.jpg".to_string(),
            "/assets/serve/batch_asset1.jpg".to_string(),
            Some("First batch asset".to_string())
        );
        let asset2 = Asset::new(
            "Batch Test Asset 2".to_string(),
            "batch_asset2.jpg".to_string(),
            "/assets/serve/batch_asset2.jpg".to_string(),
            Some("Second batch asset".to_string())
        );

        // Insert assets
        app_state.insert_asset(&asset1).await.unwrap();
        app_state.insert_asset(&asset2).await.unwrap();

        // Create multiple posts
        let post1 = Post {
            id: Uuid::new_v4(),
            title: "Batch Test Post 1".to_string(),
            category: "Batch Category 1".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            excerpt: "Batch test excerpt 1".to_string(),
            folder_id: Some("batch_folder_1".to_string()),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        let post2 = Post {
            id: Uuid::new_v4(),
            title: "Batch Test Post 2".to_string(),
            category: "Batch Category 2".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            excerpt: "Batch test excerpt 2".to_string(),
            folder_id: Some("batch_folder_2".to_string()),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        // Insert posts
        app_state.insert_post(&post1).await.unwrap();
        app_state.insert_post(&post2).await.unwrap();

        // Verify all were created correctly
        let all_assets = app_state.get_all_assets().await.unwrap();
        assert!(all_assets.len() >= 2);

        let all_posts = app_state.get_all_posts().await.unwrap();
        assert!(all_posts.len() >= 2);

        // Test batch retrieval by IDs
        let asset_ids = vec![asset1.id, asset2.id];
        let retrieved_assets = app_state.get_assets_by_ids(&asset_ids).await.unwrap();
        assert_eq!(retrieved_assets.len(), 2);

        // Cleanup test data
        cleanup_test_data(&pool).await;
    }
}
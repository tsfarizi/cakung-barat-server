use cakung_barat_server::db::AppState;
use sqlx::PgPool;
use tokio;

/// Test helper to create a test database pool
pub async fn setup_test_db() -> PgPool {
    // Use a test environment variable or mock for testing
    // For now, we'll return a mock pool that won't be used in actual tests
    // In real tests, you would need to set up a real test database
    dotenvy::dotenv().ok(); // Load environment variables if available
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test_user:test_password@localhost/test_cakung_barat".to_string());

    // This would be replaced with a real test database setup
    // For the purpose of unit testing without a real database,
    // we'll need to adjust our approach in the test files
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database (for real tests)")
}

/// Test helper to create a test AppState
pub async fn setup_test_app_state() -> AppState {
    // Mock database pool creation would be complex, so we'll implement differently in tests
    // This function is mainly for documentation purposes now
    unimplemented!("setup_test_app_state is not implemented for integration tests")
}

/// Mock implementation of ObjectStorage for testing
pub struct MockObjectStorage {
    // In-memory storage for testing
    files: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

impl MockObjectStorage {
    pub fn new() -> Self {
        Self {
            files: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn has_file(&self, filename: &str) -> bool {
        let files = self.files.lock().await;
        files.contains_key(filename)
    }
}

#[async_trait::async_trait]
impl cakung_barat_server::storage::ObjectStorage for MockObjectStorage {
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

/// Helper function to execute a test with a clean database state
pub async fn with_clean_test_db<F, Fut>() -> Fut::Output
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future,
{
    // This is for real integration tests with a database
    // For unit tests without a database, we won't use this approach
    unimplemented!("with_clean_test_db is not implemented for unit tests without database")
}

/// Helper to clean up test data
pub async fn cleanup_test_data(pool: &PgPool) {
    // Truncate all tables that might have been created during tests
    let _ = sqlx::query("TRUNCATE TABLE posts, assets, folders, asset_folders RESTART IDENTITY CASCADE")
        .execute(pool)
        .await;
}
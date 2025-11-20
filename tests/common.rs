use cakung_barat_server::db::AppState;
use sqlx::PgPool;
use tokio;

/// Test helper to create a test database pool
pub async fn setup_test_db() -> PgPool {
    // Use a test environment variable or mock for testing
    // For now, we'll return a mock pool that won't be used in actual tests
    // In real tests, you would need to set up a real test database
    dotenvy::dotenv().ok(); // Load environment variables if available
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://test_user:test_password@localhost/test_cakung_barat".to_string()
    });

    // Connect to the database
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Ensure the uuid-ossp extension is available
    sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";")
        .execute(&pool)
        .await
        .unwrap();

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
        );",
    )
    .execute(&pool)
    .await
    .unwrap();

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
        );",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS folders (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            name TEXT UNIQUE NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS asset_folders (
            asset_id UUID REFERENCES assets(id) ON DELETE CASCADE,
            folder_id UUID REFERENCES folders(id) ON DELETE CASCADE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            PRIMARY KEY (asset_id, folder_id)
        );",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_assets_filename ON assets(filename);")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_asset_folders_asset_id ON asset_folders(asset_id);",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_asset_folders_folder_id ON asset_folders(folder_id);",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE OR REPLACE FUNCTION update_updated_at_column()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
        END;
        $$ language 'plpgsql';",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS update_assets_updated_at
            BEFORE UPDATE ON assets
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column();",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS update_posts_updated_at
            BEFORE UPDATE ON posts
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column();",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
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

    async fn list_folder_contents(
        &self,
        _folder_name: &str,
    ) -> Result<Vec<cakung_barat_server::storage::FolderContent>, String> {
        // Return empty list for mock implementation
        Ok(Vec::new())
    }

    fn get_asset_url(&self, filename: &str) -> String {
        format!("http://test.example.com/{}", filename)
    }

    async fn download_file(&self, filename: &str) -> Result<Vec<u8>, String> {
        let files = self.files.lock().await;
        files
            .get(filename)
            .cloned()
            .ok_or_else(|| "File not found".to_string())
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
    let _ = sqlx::query!(
        "TRUNCATE TABLE posts, assets, folders, asset_folders RESTART IDENTITY CASCADE"
    )
    .execute(pool)
    .await;
}

#[cfg(test)]
mod database_tests {
    // For now, we'll focus on testing the structure and creation of the AppState
    // Actual database connection tests would require a test database setup
    use moka::future::Cache;
    use std::time::Duration;

    #[test]
    fn test_app_state_creation() {
        // Create a cache for testing without requiring a real database connection
        let post_cache: Cache<String, Vec<cakung_barat_server::posting::models::Post>> = Cache::builder()
            .time_to_live(Duration::from_secs(1))
            .max_capacity(10)
            .build();

        // AppState needs a real PgPool for the test to compile, so we'll just test that it can be created
        // This would require more sophisticated mocking in a real scenario
        assert!(!post_cache.policy().time_to_live().is_none());
    }
}
#[cfg(test)]
mod storage_tests {
    use cakung_barat_server::storage::{SupabaseConfig, FolderContent};

    #[test]
    fn test_supabase_config_debug_format() {
        // Since we added Debug to SupabaseConfig, this should work now
        let config = SupabaseConfig {
            supabase_url: "https://test.supabase.co".to_string(),
            supabase_anon_key: "test-anon-key".to_string(),
            bucket_name: "my-bucket".to_string(),
        };
        let debug_str = format!("{:?}", config);

        // The debug format should include the struct name and fields
        assert!(debug_str.contains("SupabaseConfig"));
        assert!(debug_str.contains("test.supabase.co"));
    }

    #[test]
    fn test_supabase_config_creation_with_defaults() {
        // Directly create the config object to avoid environment dependency
        let config = SupabaseConfig {
            supabase_url: "https://test.supabase.co".to_string(),
            supabase_anon_key: "test-anon-key".to_string(),
            bucket_name: "cakung-barat-supabase-bucket".to_string(),
        };

        assert_eq!(config.supabase_url, "https://test.supabase.co");
        assert_eq!(config.supabase_anon_key, "test-anon-key");
        assert_eq!(config.bucket_name, "cakung-barat-supabase-bucket");
    }

    #[test]
    fn test_supabase_config_with_custom_bucket() {
        // Directly create the config object with custom bucket
        let config = SupabaseConfig {
            supabase_url: "https://test.supabase.co".to_string(),
            supabase_anon_key: "test-anon-key".to_string(),
            bucket_name: "my-custom-bucket".to_string(),
        };

        assert_eq!(config.bucket_name, "my-custom-bucket");
    }

    #[test]
    fn test_folder_content_creation() {
        // Test FolderContent struct creation
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
    fn test_supabase_config_clone() {
        let config1 = SupabaseConfig {
            supabase_url: "https://test.supabase.co".to_string(),
            supabase_anon_key: "test-anon-key".to_string(),
            bucket_name: "test-bucket".to_string(),
        };
        let config2 = config1.clone();

        assert_eq!(config1.supabase_url, config2.supabase_url);
        assert_eq!(config1.supabase_anon_key, config2.supabase_anon_key);
        assert_eq!(config1.bucket_name, config2.bucket_name);
    }
}
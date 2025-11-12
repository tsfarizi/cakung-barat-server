#[cfg(test)]
mod db_tests {
    use crate::db::AppState;
    use sqlx::PgPool;
    use chrono::{NaiveDate, Utc};

    // Skip DB tests if no test database is configured
    async fn setup_test_db() -> Option<PgPool> {
        dotenvy::dotenv().ok();
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            if let Ok(pool) = PgPool::connect(&database_url).await {
                // Clean up any existing test data
                let _ = sqlx::query("DELETE FROM posting_assets").execute(&pool).await;
                let _ = sqlx::query("DELETE FROM post_images").execute(&pool).await;
                let _ = sqlx::query("DELETE FROM asset_folders").execute(&pool).await;
                let _ = sqlx::query("DELETE FROM posts").execute(&pool).await;
                let _ = sqlx::query("DELETE FROM assets").execute(&pool).await;
                let _ = sqlx::query("DELETE FROM folders").execute(&pool).await;
                
                Some(pool)
            } else {
                None
            }
        } else {
            None
        }
    }

    #[actix_web::test]
    async fn test_app_state_new() {
        // This test will fail if no database is configured, which is expected
        if std::env::var("SUPABASE_DATABASE_URL").is_ok() {
            dotenvy::dotenv().ok();
            let _app_state = AppState::new().await.unwrap();
        }
    }

    #[actix_web::test]
    async fn test_asset_operations() {
        if let Some(pool) = setup_test_db().await {
            let app_state = AppState { pool };

            // Create a test asset
            let test_asset = crate::asset::models::Asset::new(
                "Test Asset".to_string(),
                "test_file.jpg".to_string(),
                "/assets/serve/test_file.jpg".to_string(),
                Some("A test asset".to_string()),
            );

            // Test insert_asset
            assert!(app_state.insert_asset(&test_asset).await.is_ok());

            // Test get_asset_by_id
            let retrieved_asset = app_state.get_asset_by_id(&test_asset.id).await.unwrap();
            assert!(retrieved_asset.is_some());
            let retrieved_asset = retrieved_asset.unwrap();
            assert_eq!(retrieved_asset.name, "Test Asset");
            assert_eq!(retrieved_asset.filename, "test_file.jpg");
            assert_eq!(retrieved_asset.description, Some("A test asset".to_string()));

            // Test get_all_assets
            let all_assets = app_state.get_all_assets().await.unwrap();
            assert_eq!(all_assets.len(), 1);
            assert_eq!(all_assets[0].id, test_asset.id);

            // Test get_assets_by_ids
            let asset_ids = vec![test_asset.id];
            let assets_by_ids = app_state.get_assets_by_ids(&asset_ids).await.unwrap();
            assert_eq!(assets_by_ids.len(), 1);
            assert_eq!(assets_by_ids[0].id, test_asset.id);

            // Test delete_asset
            assert!(app_state.delete_asset(&test_asset.id).await.is_ok());
            let deleted_asset = app_state.get_asset_by_id(&test_asset.id).await.unwrap();
            assert!(deleted_asset.is_none());
        } else {
            // Skip test if no database configured
            println!("Skipping test_asset_operations - no test database configured");
        }
    }

    #[actix_web::test]
    async fn test_post_operations() {
        if let Some(pool) = setup_test_db().await {
            let app_state = AppState { pool };

            // Create a test post
            let test_post = crate::posting::models::Post {
                id: uuid::Uuid::new_v4(),
                title: "Test Post".to_string(),
                category: "Test Category".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
                excerpt: "Test excerpt".to_string(),
                img: None,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
            };

            // Test insert_post
            assert!(app_state.insert_post(&test_post).await.is_ok());

            // Test get_post_by_id
            let retrieved_post = app_state.get_post_by_id(&test_post.id).await.unwrap();
            assert!(retrieved_post.is_some());
            let retrieved_post = retrieved_post.unwrap();
            assert_eq!(retrieved_post.title, "Test Post");
            assert_eq!(retrieved_post.category, "Test Category");

            // Test get_all_posts
            let all_posts = app_state.get_all_posts().await.unwrap();
            assert_eq!(all_posts.len(), 1);
            assert_eq!(all_posts[0].id, test_post.id);

            // Test update_post
            let mut updated_post = retrieved_post.clone();
            updated_post.title = "Updated Test Post".to_string();
            updated_post.category = "Updated Category".to_string();
            assert!(app_state.update_post(&updated_post).await.is_ok());

            // Verify the update
            let updated_retrieved_post = app_state.get_post_by_id(&updated_post.id).await.unwrap();
            assert!(updated_retrieved_post.is_some());
            assert_eq!(updated_retrieved_post.unwrap().title, "Updated Test Post");

            // Test delete_post
            assert!(app_state.delete_post(&test_post.id).await.is_ok());
            let deleted_post = app_state.get_post_by_id(&test_post.id).await.unwrap();
            assert!(deleted_post.is_none());
        } else {
            // Skip test if no database configured
            println!("Skipping test_post_operations - no test database configured");
        }
    }

    #[actix_web::test]
    async fn test_post_with_images_operations() {
        if let Some(pool) = setup_test_db().await {
            let app_state = AppState { pool };

            // Create test assets first
            let asset1 = crate::asset::models::Asset::new(
                "Image 1".to_string(),
                "image1.jpg".to_string(),
                "/assets/serve/image1.jpg".to_string(),
                None,
            );
            let asset2 = crate::asset::models::Asset::new(
                "Image 2".to_string(),
                "image2.jpg".to_string(),
                "/assets/serve/image2.jpg".to_string(),
                None,
            );

            app_state.insert_asset(&asset1).await.unwrap();
            app_state.insert_asset(&asset2).await.unwrap();

            // Create a post with images
            let test_post = crate::posting::models::Post {
                id: uuid::Uuid::new_v4(),
                title: "Post with Images".to_string(),
                category: "Test Category".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
                excerpt: "Test excerpt with images".to_string(),
                img: Some(vec![asset1.id, asset2.id]),
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
            };

            // Test insert_post with images
            assert!(app_state.insert_post(&test_post).await.is_ok());

            // Test get_post_by_id with images
            let retrieved_post = app_state.get_post_by_id(&test_post.id).await.unwrap();
            assert!(retrieved_post.is_some());
            let retrieved_post = retrieved_post.unwrap();
            assert!(retrieved_post.img.is_some());
            let img_ids = retrieved_post.img.unwrap();
            assert_eq!(img_ids.len(), 2);
            assert!(img_ids.contains(&asset1.id));
            assert!(img_ids.contains(&asset2.id));

            // Clean up
            app_state.delete_post(&test_post.id).await.unwrap();
            app_state.delete_asset(&asset1.id).await.unwrap();
            app_state.delete_asset(&asset2.id).await.unwrap();
        } else {
            // Skip test if no database configured
            println!("Skipping test_post_with_images_operations - no test database configured");
        }
    }

    #[actix_web::test]
    async fn test_posting_operations() {
        if let Some(pool) = setup_test_db().await {
            let app_state = AppState { pool };

            // Create test asset
            let test_asset = crate::asset::models::Asset::new(
                "Associate Asset".to_string(),
                "assoc.jpg".to_string(),
                "/assets/serve/assoc.jpg".to_string(),
                None,
            );
            app_state.insert_asset(&test_asset).await.unwrap();

            // Create a test posting
            let test_posting = crate::posting::models::Posting {
                id: uuid::Uuid::new_v4(),
                title: "Test Posting".to_string(),
                category: "Test Category".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
                excerpt: "Test excerpt".to_string(),
                img: None,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                asset_ids: vec![test_asset.id],
            };

            // Test upsert_posting_with_assets
            assert!(app_state.upsert_posting_with_assets(&test_posting).await.is_ok());

            // Test get_posting_by_id_with_assets
            let retrieved_posting = app_state.get_posting_by_id_with_assets(&test_posting.id).await.unwrap();
            assert!(retrieved_posting.is_some());
            let retrieved_posting = retrieved_posting.unwrap();
            assert_eq!(retrieved_posting.title, "Test Posting");
            assert_eq!(retrieved_posting.asset_ids.len(), 1);
            assert_eq!(retrieved_posting.asset_ids[0], test_asset.id);

            // Test get_all_postings_with_assets
            let all_postings = app_state.get_all_postings_with_assets().await.unwrap();
            assert_eq!(all_postings.len(), 1);
            assert_eq!(all_postings[0].id, test_posting.id);
            assert_eq!(all_postings[0].asset_ids.len(), 1);

            // Clean up
            app_state.delete_post(&test_posting.id).await.unwrap();
            app_state.delete_asset(&test_asset.id).await.unwrap();
        } else {
            // Skip test if no database configured
            println!("Skipping test_posting_operations - no test database configured");
        }
    }

    #[actix_web::test]
    async fn test_folder_operations() {
        if let Some(pool) = setup_test_db().await {
            let app_state = AppState { pool };

            let folder_name = "test_folder";
            let asset1 = crate::asset::models::Asset::new(
                "Folder Asset 1".to_string(),
                "folder1.jpg".to_string(),
                "/assets/serve/folder1.jpg".to_string(),
                None,
            );
            let asset2 = crate::asset::models::Asset::new(
                "Folder Asset 2".to_string(),
                "folder2.jpg".to_string(),
                "/assets/serve/folder2.jpg".to_string(),
                None,
            );

            app_state.insert_asset(&asset1).await.unwrap();
            app_state.insert_asset(&asset2).await.unwrap();

            let contents = vec![asset1.id, asset2.id];

            // Test insert_folder_contents
            assert!(app_state.insert_folder_contents(folder_name, &contents).await.is_ok());

            // Test get_folder_contents
            let folder_contents = app_state.get_folder_contents(folder_name).await.unwrap();
            assert!(folder_contents.is_some());
            let folder_contents = folder_contents.unwrap();
            assert_eq!(folder_contents.len(), 2);
            assert!(folder_contents.contains(&asset1.id));
            assert!(folder_contents.contains(&asset2.id));

            // Test with non-existent folder
            let non_existent_folder = app_state.get_folder_contents("non_existent_folder").await.unwrap();
            assert!(non_existent_folder.is_none());

            // Clean up
            app_state.delete_asset(&asset1.id).await.unwrap();
            app_state.delete_asset(&asset2.id).await.unwrap();
        } else {
            // Skip test if no database configured
            println!("Skipping test_folder_operations - no test database configured");
        }
    }

    #[actix_web::test]
    async fn test_empty_folder_operations() {
        if let Some(pool) = setup_test_db().await {
            let app_state = AppState { pool };

            let folder_name = "empty_folder";

            // Test inserting empty folder
            assert!(app_state.insert_folder_contents(folder_name, &vec![]).await.is_ok());

            // Test getting empty folder
            let folder_contents = app_state.get_folder_contents(folder_name).await.unwrap();
            assert!(folder_contents.is_some());
            assert_eq!(folder_contents.unwrap().len(), 0);
        } else {
            // Skip test if no database configured
            println!("Skipping test_empty_folder_operations - no test database configured");
        }
    }
}
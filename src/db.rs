use dotenvy::dotenv;
use std::env;
use sqlx::PgPool;
use uuid::Uuid;
use log;
use moka::future::Cache;
use std::time::Duration;
use reqwest;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub post_cache: Cache<String, Vec<crate::posting::models::Post>>,
    pub http_client: reqwest::Client,
    pub storage: Arc<dyn crate::storage::ObjectStorage + Send + Sync>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        let database_url = env::var("SUPABASE_DATABASE_URL")
            .expect("SUPABASE_DATABASE_URL must be set");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(100)
            .min_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(900))
            .max_lifetime(std::time::Duration::from_secs(1800))
            .connect(&database_url)
            .await?;

        let post_cache = Cache::builder()
            .time_to_live(Duration::from_secs(10 * 60))
            .max_capacity(100)
            .build();

        let http_client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(900))
            .user_agent("cakung-barat-server/1.0")
            .build()
            .expect("Failed to create reqwest client");

        let supabase_config = crate::storage::SupabaseConfig::from_env()?;
        let storage = Arc::new(crate::storage::SupabaseStorage::new(supabase_config, http_client.clone()));

        Ok(AppState { pool, post_cache, http_client, storage })
    }

    pub async fn new_with_pool_and_storage(
        pool: sqlx::PgPool,
        storage: Arc<dyn crate::storage::ObjectStorage + Send + Sync>
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let post_cache = Cache::builder()
            .time_to_live(Duration::from_secs(10 * 60))
            .max_capacity(100)
            .build();

        let http_client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(900))
            .user_agent("cakung-barat-server/1.0")
            .build()
            .expect("Failed to create reqwest client");

        Ok(AppState { pool, post_cache, http_client, storage })
    }

    pub async fn get_asset_by_id(&self, id: &Uuid) -> Result<Option<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as!(crate::asset::models::Asset, "SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_all_assets(&self) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as!(crate::asset::models::Asset, "SELECT id, name, filename, url, description, created_at, updated_at FROM assets ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_assets_by_ids(&self, ids: &Vec<Uuid>) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as!(crate::asset::models::Asset, "SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = ANY($1)", ids)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn insert_asset(&self, asset: &crate::asset::models::Asset) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO assets (id, name, filename, url, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE
             SET name = $2, filename = $3, url = $4, description = $5, updated_at = $7
            "#,
            asset.id,
            &asset.name,
            &asset.filename,
            &asset.url,
            asset.description.as_deref(),
            asset.created_at,
            asset.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_asset(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM assets WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_post_by_id(&self, id: &Uuid) -> Result<Option<crate::posting::models::Post>, sqlx::Error> {

        sqlx::query_as!(
            crate::posting::models::Post,
            "SELECT id, title, category, date, excerpt, folder_id, created_at, updated_at FROM posts WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error getting post by id: {:?}", e);
            e
        })
    }

    pub async fn get_all_posts_cached(&self) -> Result<Vec<crate::posting::models::Post>, sqlx::Error> {
        let key = "all_posts";
        if let Some(posts) = self.post_cache.get(key).await {
            log::info!("Cache hit for all_posts");
            return Ok(posts);
        }

        log::info!("Cache miss for all_posts");
        let posts = self.get_all_posts().await?;
        self.post_cache.insert(key.to_string(), posts.clone()).await;
        Ok(posts)
    }

    pub async fn get_posts_paginated(&self, limit: i32, offset: i32) -> Result<Vec<crate::posting::models::Post>, sqlx::Error> {

        sqlx::query_as!(
            crate::posting::models::Post,
            "SELECT p.id, p.title, p.category, p.date, p.excerpt, p.folder_id, p.created_at, p.updated_at
             FROM posts p
             ORDER BY p.created_at DESC
             LIMIT $1 OFFSET $2",
            i64::from(limit),
            i64::from(offset)
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error getting paginated posts: {:?}", e);
            e
        })
    }

    pub async fn get_all_posts(&self) -> Result<Vec<crate::posting::models::Post>, sqlx::Error> {
        sqlx::query_as!(
            crate::posting::models::Post,
            "SELECT p.id, p.title, p.category, p.date, p.excerpt, p.folder_id, p.created_at, p.updated_at
             FROM posts p
             ORDER BY p.created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error getting all posts: {:?}", e);
            e
        })
    }

    pub async fn insert_post(&self, post: &crate::posting::models::Post) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO posts (id, title, category, date, excerpt, folder_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            post.id,
            &post.title,
            &post.category,
            post.date,
            &post.excerpt,
            post.folder_id.as_deref(),
            post.created_at,
            post.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error inserting post record: {:?}", e);
            e
        })?;

        Ok(())
    }

    pub async fn update_post(&self, post: &crate::posting::models::Post) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE posts
             SET title = $2, category = $3, date = $4, excerpt = $5, folder_id = $6, updated_at = $7
             WHERE id = $1
            "#,
            post.id,
            &post.title,
            &post.category,
            post.date,
            &post.excerpt,
            post.folder_id.as_deref(),
            post.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error updating post record: {:?}", e);
            e
        })?;

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }

    pub async fn delete_post(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM posts WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error deleting post: {:?}", e);
                e
            })?;

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }



    pub async fn get_folder_contents(
        &self,
        folder_name: &str,
    ) -> Result<Option<Vec<Uuid>>, sqlx::Error> {
        log::debug!("Attempting to get contents for folder: {}", folder_name);

        let folder_row = sqlx::query!("SELECT id FROM folders WHERE name = $1", folder_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error getting folder: {:?}", e);
                e
            })?;

        if let Some(folder_record) = folder_row {
            let asset_rows = sqlx::query!("SELECT asset_id FROM asset_folders WHERE folder_id = $1", folder_record.id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    log::error!("Error getting folder assets: {:?}", e);
                    e
                })?;

            let asset_ids: Vec<Uuid> = asset_rows.into_iter().map(|row| row.asset_id).collect();

            log::info!("Retrieved {} assets from folder: {}", asset_ids.len(), folder_name);
            Ok(Some(asset_ids))
        } else {
            log::debug!("Folder not found: {}", folder_name);
            Ok(None)
        }
    }

    pub async fn insert_folder_contents(
        &self,
        folder_name: &str,
        contents: &Vec<Uuid>,
    ) -> Result<(), sqlx::Error> {
        log::debug!("Attempting to insert folder contents for folder: {}, with {} assets", folder_name, contents.len());

        let folder_record = sqlx::query!("INSERT INTO folders (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id", folder_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error upserting folder: {:?}", e);
                e
            })?;
        let folder_id = folder_record.id;
        log::debug!("Got/created folder with ID: {} for name: {}", folder_id, folder_name);

        let mut tx = self.pool.begin().await
            .map_err(|e| {
                log::error!("Error beginning transaction: {:?}", e);
                e
            })?;

        sqlx::query!("DELETE FROM asset_folders WHERE folder_id = $1", folder_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                log::error!("Error deleting asset folders: {:?}", e);
                e
            })?;

        for asset_id in contents {
            sqlx::query!("INSERT INTO asset_folders (folder_id, asset_id) VALUES ($1, $2)", folder_id, asset_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    log::error!("Error inserting asset folder: {:?}", e);
                    e
                })?;
            log::debug!("Associated asset ID: {} with folder ID: {}", asset_id, folder_id);
        }

        tx.commit().await
            .map_err(|e| {
                log::error!("Error committing transaction: {:?}", e);
                e
            })?;
        log::info!("Successfully updated folder contents for folder: {}, with {} assets", folder_name, contents.len());
        Ok(())
    }

    pub async fn get_posting_by_id_with_assets(&self, id: &Uuid) -> Result<Option<crate::posting::models::PostWithAssets>, sqlx::Error> {
        let post = sqlx::query_as!(
            crate::posting::models::Post,
            "SELECT id, title, category, date, excerpt, folder_id, created_at, updated_at FROM posts WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error getting post by id: {:?}", e);
            e
        })?;

        if let Some(post) = post {
            let mut asset_ids = Vec::new();

            if let Some(folder_name) = &post.folder_id {
                if let Some(folder_asset_ids) = self.get_folder_contents(folder_name).await? {
                    asset_ids = folder_asset_ids;
                }
            }

            Ok(Some(crate::posting::models::PostWithAssets {
                id: post.id,
                title: post.title,
                category: post.category,
                date: post.date,
                excerpt: post.excerpt,
                folder_id: post.folder_id,
                created_at: post.created_at,
                updated_at: post.updated_at,
                asset_ids,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn upsert_posting_with_assets(&self, post: &crate::posting::models::PostWithAssets) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO posts (id, title, category, date, excerpt, folder_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id)
             DO UPDATE SET title = $2, category = $3, date = $4, excerpt = $5, folder_id = $6, updated_at = $7
            "#,
            post.id,
            &post.title,
            &post.category,
            post.date,
            &post.excerpt,
            post.folder_id.as_deref(),
            post.created_at,
            post.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error upserting post record: {:?}", e);
            e
        })?;

        if let Some(folder_name) = &post.folder_id {
            if !post.asset_ids.is_empty() {
                self.insert_folder_contents(folder_name, &post.asset_ids).await?;
            }
        }

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }

    pub async fn get_all_postings_with_assets(&self) -> Result<Vec<crate::posting::models::PostWithAssets>, sqlx::Error> {
        let posts = self.get_all_posts().await?;

        let mut result = Vec::new();
        for post in posts {
            let mut asset_ids = Vec::new();
            if let Some(folder_name) = &post.folder_id {
                if let Some(folder_asset_ids) = self.get_folder_contents(folder_name).await? {
                    asset_ids = folder_asset_ids;
                }
            }

            result.push(crate::posting::models::PostWithAssets {
                id: post.id,
                title: post.title.clone(),
                category: post.category.clone(),
                date: post.date,
                excerpt: post.excerpt.clone(),
                folder_id: post.folder_id.clone(),
                created_at: post.created_at,
                updated_at: post.updated_at,
                asset_ids,
            });
        }

        Ok(result)
    }
}
use dotenvy::dotenv;
use std::env;
use sqlx::{PgPool, Row};
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

    pub async fn get_asset_by_id(&self, id: &Uuid) -> Result<Option<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as("SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_all_assets(&self) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as("SELECT id, name, filename, url, description, created_at, updated_at FROM assets ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_assets_by_ids(&self, ids: &Vec<Uuid>) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as("SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = ANY($1)")
            .bind(ids)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn insert_asset(&self, asset: &crate::asset::models::Asset) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO assets (id, name, filename, url, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE
             SET name = $2, filename = $3, url = $4, description = $5, updated_at = $7"
        )
        .bind(asset.id)
        .bind(&asset.name)
        .bind(&asset.filename)
        .bind(&asset.url)
        .bind(&asset.description)
        .bind(asset.created_at)
        .bind(asset.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_asset(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM assets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Posting-related functions
    pub async fn get_post_by_id(&self, id: &Uuid) -> Result<Option<crate::posting::models::Post>, sqlx::Error> {
        // First get the post details
        let post_row = sqlx::query("SELECT id, title, category, date, excerpt, folder_id, created_at, updated_at FROM posts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error getting post by id: {:?}", e);
                e
            })?;

        if let Some(row) = post_row {
            // Get the folder_id for the post
            let folder_id: Option<String> = row.get("folder_id");

            Ok(Some(crate::posting::models::Post {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                folder_id,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
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
        // Fetch posts with their folder_id
        let rows = sqlx::query(
            "SELECT p.id, p.title, p.category, p.date, p.excerpt, p.folder_id, p.created_at, p.updated_at
             FROM posts p
             ORDER BY p.created_at DESC
             LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error getting paginated posts: {:?}", e);
            e
        })?;

        let mut result = Vec::new();
        for row in rows {
            let folder_id: Option<String> = row.get("folder_id");

            result.push(crate::posting::models::Post {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                folder_id,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(result)
    }

    pub async fn get_all_posts(&self) -> Result<Vec<crate::posting::models::Post>, sqlx::Error> {
        // Fetch posts with their folder_id
        let rows = sqlx::query(
            "SELECT p.id, p.title, p.category, p.date, p.excerpt, p.folder_id, p.created_at, p.updated_at
             FROM posts p
             ORDER BY p.created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error getting all posts: {:?}", e);
            e
        })?;

        let mut result = Vec::new();
        for row in rows {
            let folder_id: Option<String> = row.get("folder_id");

            result.push(crate::posting::models::Post {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                folder_id,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(result)
    }

    pub async fn insert_post(&self, post: &crate::posting::models::Post) -> Result<(), sqlx::Error> {
        // Insert the post record with folder_id
        sqlx::query(
            "INSERT INTO posts (id, title, category, date, excerpt, folder_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(post.id)
        .bind(&post.title)
        .bind(&post.category)
        .bind(post.date)
        .bind(&post.excerpt)
        .bind(&post.folder_id)
        .bind(post.created_at)
        .bind(post.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error inserting post record: {:?}", e);
            e
        })?;

        Ok(())
    }

    pub async fn update_post(&self, post: &crate::posting::models::Post) -> Result<(), sqlx::Error> {
        // Update the post record with folder_id
        sqlx::query(
            "UPDATE posts
             SET title = $2, category = $3, date = $4, excerpt = $5, folder_id = $6, updated_at = $7
             WHERE id = $1"
        )
        .bind(post.id)
        .bind(&post.title)
        .bind(&post.category)
        .bind(post.date)
        .bind(&post.excerpt)
        .bind(&post.folder_id)
        .bind(post.updated_at)
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
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error deleting post: {:?}", e);
                e
            })?;

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }



    // Folder-related functions
    pub async fn get_folder_contents(
        &self,
        folder_name: &str,
    ) -> Result<Option<Vec<Uuid>>, sqlx::Error> {
        log::debug!("Attempting to get contents for folder: {}", folder_name);

        let folder_row = sqlx::query("SELECT id FROM folders WHERE name = $1")
            .bind(folder_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error getting folder: {:?}", e);
                e
            })?;

        if let Some(folder_record) = folder_row {
            let folder_id_val: Uuid = folder_record.get::<Uuid, _>("id");
            let asset_rows = sqlx::query("SELECT asset_id FROM asset_folders WHERE folder_id = $1")
                .bind(folder_id_val)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    log::error!("Error getting folder assets: {:?}", e);
                    e
                })?;

            let asset_ids: Vec<Uuid> = asset_rows.into_iter().map(|row| row.get::<Uuid, _>("asset_id")).collect();

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

        let folder_record: (Uuid,) = sqlx::query_as("INSERT INTO folders (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id")
            .bind(folder_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error upserting folder: {:?}", e);
                e
            })?;
        let folder_id = folder_record.0;
        log::debug!("Got/created folder with ID: {} for name: {}", folder_id, folder_name);

        let mut tx = self.pool.begin().await
            .map_err(|e| {
                log::error!("Error beginning transaction: {:?}", e);
                e
            })?;

        sqlx::query("DELETE FROM asset_folders WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                log::error!("Error deleting asset folders: {:?}", e);
                e
            })?;

        for asset_id in contents {
            sqlx::query("INSERT INTO asset_folders (folder_id, asset_id) VALUES ($1, $2)")
                .bind(folder_id)
                .bind(asset_id)
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

    // Methods related to posts with assets
    pub async fn get_posting_by_id_with_assets(&self, id: &Uuid) -> Result<Option<crate::posting::models::PostWithAssets>, sqlx::Error> {
        // First get the post details
        let post_row = sqlx::query("SELECT id, title, category, date, excerpt, folder_id, created_at, updated_at FROM posts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                log::error!("Error getting post by id: {:?}", e);
                e
            })?;

        if let Some(row) = post_row {
            // Get associated asset IDs for this post by looking at folder contents if the post has a folder_id
            let folder_id: Option<String> = row.get("folder_id");
            let mut asset_ids = Vec::new();

            if let Some(folder_name) = &folder_id {
                if let Some(folder_asset_ids) = self.get_folder_contents(folder_name).await? {
                    asset_ids = folder_asset_ids;
                }
            }

            Ok(Some(crate::posting::models::PostWithAssets {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                folder_id,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                asset_ids,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn upsert_posting_with_assets(&self, post: &crate::posting::models::PostWithAssets) -> Result<(), sqlx::Error> {
        // First, update the post record
        sqlx::query(
            "INSERT INTO posts (id, title, category, date, excerpt, folder_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id)
             DO UPDATE SET title = $2, category = $3, date = $4, excerpt = $5, folder_id = $6, updated_at = $7"
        )
        .bind(post.id)
        .bind(&post.title)
        .bind(&post.category)
        .bind(post.date)
        .bind(&post.excerpt)
        .bind(&post.folder_id)
        .bind(post.created_at)
        .bind(post.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Error upserting post record: {:?}", e);
            e
        })?;

        // Update folder contents if a folder_id exists and there are asset IDs
        if let Some(folder_name) = &post.folder_id {
            if !post.asset_ids.is_empty() {
                self.insert_folder_contents(folder_name, &post.asset_ids).await?;
            }
        }

        // Invalidate cache
        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }

    pub async fn get_all_postings_with_assets(&self) -> Result<Vec<crate::posting::models::PostWithAssets>, sqlx::Error> {
        // Get all posts
        let posts = self.get_all_posts().await?;

        // Create PostWithAssets for each post by adding their associated asset IDs
        let mut result = Vec::new();
        for post in posts {
            // Get associated asset IDs for this post by looking at folder contents if the post has a folder_id
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
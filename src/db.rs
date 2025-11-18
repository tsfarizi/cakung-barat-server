use dotenvy::dotenv;
use std::env;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use log;
use moka::future::Cache;
use std::time::Duration;
use reqwest;

pub struct AppState {
    pub pool: PgPool,
    pub post_cache: Cache<String, Vec<crate::posting::models::Post>>,
    pub http_client: reqwest::Client,
    pub supabase_config: crate::storage::SupabaseConfig,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        let database_url = env::var("SUPABASE_DATABASE_URL")
            .expect("SUPABASE_DATABASE_URL must be set");

        // Configure and create database pool with optimized settings
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(50)
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

        // Create a reusable HTTP client with connection pooling
        let http_client = reqwest::Client::builder()
            .use_rustls_tls()
            .pool_idle_timeout(std::time::Duration::from_secs(900))
            .user_agent("cakung-barat-server/1.0")
            .build()
            .expect("Failed to create reqwest client");

        // Create Supabase configuration cached once from environment
        let supabase_config = crate::storage::SupabaseConfig::from_env()?;

        Ok(AppState { pool, post_cache, http_client, supabase_config })
    }

    // Asset-related functions
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
             SET name = $2, filename = $3, url = $4, description = $5, updated_at = $7")
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
        let post_row = sqlx::query("SELECT id, title, category, date, excerpt, created_at, updated_at FROM posts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(row) = post_row {
            // Get associated asset IDs for images
            let img_ids: Vec<Uuid> = sqlx::query("SELECT asset_id FROM post_images WHERE post_id = $1 ORDER BY sort_order")
                .bind(id)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|r| r.get::<Uuid, _>("asset_id"))
                .collect();
                
            let img = if img_ids.is_empty() { None } else { Some(img_ids) };
            
            Ok(Some(crate::posting::models::Post {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                img,
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
        // Use a single query with LEFT JOIN to fetch posts and associated image IDs with pagination
        let rows = sqlx::query(
            "SELECT p.id, p.title, p.category, p.date, p.excerpt, p.created_at, p.updated_at,
                    COALESCE(array_agg(pi.asset_id ORDER BY pi.sort_order) FILTER (WHERE pi.asset_id IS NOT NULL), '{}') as img_ids
             FROM posts p
             LEFT JOIN post_images pi ON p.id = pi.post_id
             GROUP BY p.id
             ORDER BY p.created_at DESC
             LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let img_ids: Vec<Uuid> = row.get("img_ids");
            let img = if img_ids.is_empty() { None } else { Some(img_ids) };

            result.push(crate::posting::models::Post {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                img,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(result)
    }

    pub async fn get_all_posts(&self) -> Result<Vec<crate::posting::models::Post>, sqlx::Error> {
        // Use a single query with LEFT JOIN to fetch posts and associated image IDs
        let rows = sqlx::query(
            "SELECT p.id, p.title, p.category, p.date, p.excerpt, p.created_at, p.updated_at,
                    COALESCE(array_agg(pi.asset_id ORDER BY pi.sort_order) FILTER (WHERE pi.asset_id IS NOT NULL), '{}') as img_ids
             FROM posts p
             LEFT JOIN post_images pi ON p.id = pi.post_id
             GROUP BY p.id
             ORDER BY p.created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let img_ids: Vec<Uuid> = row.get("img_ids");
            let img = if img_ids.is_empty() { None } else { Some(img_ids) };

            result.push(crate::posting::models::Post {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                date: row.get("date"),
                excerpt: row.get("excerpt"),
                img,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(result)
    }

    pub async fn insert_post(&self, post: &crate::posting::models::Post) -> Result<(), sqlx::Error> {
        // Insert the post record
        sqlx::query(
            "INSERT INTO posts (id, title, category, date, excerpt, created_at, updated_at) 
             VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(post.id)
        .bind(&post.title)
        .bind(&post.category)
        .bind(post.date)
        .bind(&post.excerpt)
        .bind(post.created_at)
        .bind(post.updated_at)
        .execute(&self.pool)
        .await?;

        // Insert associated image asset IDs if any
        if let Some(img_ids) = &post.img {
            for (index, img_id) in img_ids.iter().enumerate() {
                sqlx::query("INSERT INTO post_images (post_id, asset_id, sort_order) VALUES ($1, $2, $3)")
                    .bind(post.id)
                    .bind(img_id)
                    .bind(index as i32)
                    .execute(&self.pool)
                    .await?;
            }
        }

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }

    pub async fn update_post(&self, post: &crate::posting::models::Post) -> Result<(), sqlx::Error> {
        // Update the post record
        sqlx::query(
            "UPDATE posts 
             SET title = $2, category = $3, date = $4, excerpt = $5, updated_at = $6 
             WHERE id = $1")
        .bind(post.id)
        .bind(&post.title)
        .bind(&post.category)
        .bind(post.date)
        .bind(&post.excerpt)
        .bind(post.updated_at)
        .execute(&self.pool)
        .await?;

        // Remove existing image associations
        sqlx::query("DELETE FROM post_images WHERE post_id = $1")
            .bind(post.id)
            .execute(&self.pool)
            .await?;

        // Insert new image associations if any
        if let Some(img_ids) = &post.img {
            for (index, img_id) in img_ids.iter().enumerate() {
                sqlx::query("INSERT INTO post_images (post_id, asset_id, sort_order) VALUES ($1, $2, $3)")
                    .bind(post.id)
                    .bind(img_id)
                    .bind(index as i32)
                    .execute(&self.pool)
                    .await?;
            }
        }

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }

    pub async fn delete_post(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        self.post_cache.invalidate("all_posts").await;
        Ok(())
    }

    // Posting with assets-related functions
    pub async fn get_posting_by_id_with_assets(&self, id: &Uuid) -> Result<Option<crate::posting::models::Posting>, sqlx::Error> {
        // First, get the post
        let post = self.get_post_by_id(id).await?;
        
        if let Some(post) = post {
            // Get the associated asset IDs
            let asset_ids = sqlx::query("SELECT asset_id FROM posting_assets WHERE posting_id = $1")
                .bind(id)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| row.get::<Uuid, _>("asset_id"))
                .collect();
            
            // Create and return the posting with asset_ids
            let posting = crate::posting::models::Posting {
                id: post.id,
                title: post.title,
                category: post.category,
                date: post.date,
                excerpt: post.excerpt,
                img: post.img,
                created_at: post.created_at,
                updated_at: post.updated_at,
                asset_ids,
            };
            
            Ok(Some(posting))
        } else {
            Ok(None)
        }
    }

    pub async fn upsert_posting_with_assets(&self, posting: &crate::posting::models::Posting) -> Result<(), sqlx::Error> {
        // First, save or update the post itself
        match self.get_post_by_id(&posting.id).await {
            Ok(Some(_)) => {
                // Post exists, update it
                let post = crate::posting::models::Post {
                    id: posting.id,
                    title: posting.title.clone(),
                    category: posting.category.clone(),
                    date: posting.date,
                    excerpt: posting.excerpt.clone(),
                    img: posting.img.clone(),
                    created_at: posting.created_at,
                    updated_at: Some(chrono::Utc::now()),
                };
                self.update_post(&post).await?;
            }
            Ok(None) | Err(_) => {
                // Post doesn't exist, insert it
                let post = crate::posting::models::Post {
                    id: posting.id,
                    title: posting.title.clone(),
                    category: posting.category.clone(),
                    date: posting.date,
                    excerpt: posting.excerpt.clone(),
                    img: posting.img.clone(),
                    created_at: Some(chrono::Utc::now()),
                    updated_at: Some(chrono::Utc::now()),
                };
                self.insert_post(&post).await?;
            }
        }

        // Update the asset associations
        // First delete existing associations
        sqlx::query("DELETE FROM posting_assets WHERE posting_id = $1")
            .bind(&posting.id)
            .execute(&self.pool)
            .await?;

        // Then insert new associations
        for asset_id in &posting.asset_ids {
            sqlx::query("INSERT INTO posting_assets (posting_id, asset_id) VALUES ($1, $2)")
                .bind(&posting.id)
                .bind(asset_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn get_all_postings_with_assets(&self) -> Result<Vec<crate::posting::models::Posting>, sqlx::Error> {
        // Get all posts first
        let posts = self.get_all_posts().await?;

        // For each post, get associated asset IDs
        let mut postings = Vec::new();
        for post in posts {
            let asset_ids = sqlx::query("SELECT asset_id FROM posting_assets WHERE posting_id = $1")
                .bind(&post.id)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| row.get::<Uuid, _>("asset_id"))
                .collect();
            
            let posting = crate::posting::models::Posting {
                id: post.id,
                title: post.title,
                category: post.category,
                date: post.date,
                excerpt: post.excerpt,
                img: post.img,
                created_at: post.created_at,
                updated_at: post.updated_at,
                asset_ids,
            };
            
            postings.push(posting);
        }

        Ok(postings)
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
            .await?;

        if let Some(folder_record) = folder_row {
            let folder_id_val: Uuid = folder_record.get::<Uuid, _>("id");
            let asset_rows = sqlx::query("SELECT asset_id FROM asset_folders WHERE folder_id = $1")
                .bind(folder_id_val)
                .fetch_all(&self.pool)
                .await?;

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
            .await?;
        let folder_id = folder_record.0;
        log::debug!("Got/created folder with ID: {} for name: {}", folder_id, folder_name);

        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM asset_folders WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&mut *tx)
            .await?;

        for asset_id in contents {
            sqlx::query("INSERT INTO asset_folders (folder_id, asset_id) VALUES ($1, $2)")
                .bind(folder_id)
                .bind(asset_id)
                .execute(&mut *tx)
                .await?;
            log::debug!("Associated asset ID: {} with folder ID: {}", asset_id, folder_id);
        }

        tx.commit().await?;
        log::info!("Successfully updated folder contents for folder: {}, with {} assets", folder_name, contents.len());
        Ok(())
    }
}
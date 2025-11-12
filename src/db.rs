use dotenvy::dotenv;
use std::env;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use log;

pub struct AppState {
    pub pool: PgPool,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        let database_url = env::var("SUPABASE_DATABASE_URL")
            .expect("SUPABASE_DATABASE_URL must be set");

        let pool = PgPool::connect(&database_url).await?;

        Ok(AppState { pool })
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

    pub async fn get_all_posts(&self) -> Result<Vec<crate::posting::models::Post>, sqlx::Error> {
        // First get all posts
        let posts = sqlx::query("SELECT id, title, category, date, excerpt, created_at, updated_at FROM posts ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
            
        let mut result = Vec::new();
        for post_row in posts {
            let post_id: Uuid = post_row.get("id");
            
            // Get associated asset IDs for images
            let img_ids: Vec<Uuid> = sqlx::query("SELECT asset_id FROM post_images WHERE post_id = $1 ORDER BY sort_order")
                .bind(&post_id)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|r| r.get::<Uuid, _>("asset_id"))
                .collect();
                
            let img = if img_ids.is_empty() { None } else { Some(img_ids) };
            
            result.push(crate::posting::models::Post {
                id: post_row.get("id"),
                title: post_row.get("title"),
                category: post_row.get("category"),
                date: post_row.get("date"),
                excerpt: post_row.get("excerpt"),
                img,
                created_at: post_row.get("created_at"),
                updated_at: post_row.get("updated_at"),
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

        Ok(())
    }

    pub async fn delete_post(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

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
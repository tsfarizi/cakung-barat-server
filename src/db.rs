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
    pub async fn get_asset_by_id(&self, _id: &Uuid) -> Result<Option<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as("SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = $1")
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_all_assets(&self) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as("SELECT id, name, filename, url, description, created_at, updated_at FROM assets ORDER BY created_at DESC")
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
    pub async fn get_posting_by_id_with_assets(&self, id: &Uuid) -> Result<Option<crate::posting::models::Posting>, sqlx::Error> {
        let mut posting: crate::posting::models::Posting = match sqlx::query_as("SELECT id, judul, tanggal, detail, created_at, updated_at FROM postings WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let asset_rows = sqlx::query("SELECT asset_id FROM posting_assets WHERE posting_id = $1")
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

        let asset_ids: Vec<Uuid> = asset_rows.into_iter().map(|row| row.get::<Uuid, _>("asset_id")).collect();
        
        posting.asset_ids = asset_ids;
        Ok(Some(posting))
    }

    pub async fn get_all_postings_with_assets(&self) -> Result<Vec<crate::posting::models::Posting>, sqlx::Error> {
        let mut postings: Vec<crate::posting::models::Posting> = sqlx::query_as("SELECT id, judul, tanggal, detail, created_at, updated_at FROM postings ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        
        let relation_rows = sqlx::query("SELECT posting_id, asset_id FROM posting_assets")
            .fetch_all(&self.pool)
            .await?;
        
        // Map relations to postings
        for posting in postings.iter_mut() {
            posting.asset_ids = relation_rows.iter()
                .filter(|rel| rel.get::<Uuid, _>("posting_id") == posting.id)
                .map(|rel| rel.get::<Uuid, _>("asset_id"))
                .collect();
        }
        Ok(postings)
    }

    pub async fn upsert_posting_with_assets(&self, posting: &crate::posting::models::Posting) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO postings (id, judul, tanggal, detail, created_at, updated_at) 
             VALUES ($1, $2, $3, $4, $5, $6) 
             ON CONFLICT (id) DO UPDATE 
             SET judul = $2, tanggal = $3, detail = $4, updated_at = $6")
        .bind(posting.id)
        .bind(&posting.judul)
        .bind(posting.tanggal)
        .bind(&posting.detail)
        .bind(posting.created_at)
        .bind(posting.updated_at)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM posting_assets WHERE posting_id = $1")
            .bind(posting.id)
            .execute(&mut *tx)
            .await?;
        
        for asset_id in &posting.asset_ids {
            sqlx::query("INSERT INTO posting_assets (posting_id, asset_id) VALUES ($1, $2)")
                .bind(posting.id)
                .bind(asset_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await
    }

    pub async fn delete_posting(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM posting_assets WHERE posting_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM postings WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await
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
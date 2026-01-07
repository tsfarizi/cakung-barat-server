//! Database module - AppState and database operations
//!
//! This module is split into submodules for better separation of concerns:
//! - `asset` - Asset-related database operations
//! - `posting` - Post/Posting-related database operations  
//! - `admin` - Admin authentication database operations

mod admin;
mod asset;
mod posting;

use dotenvy::dotenv;
use moka::future::Cache;
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub post_cache: Cache<String, Vec<crate::posting::models::Post>>,
    pub organization_cache: Cache<String, Vec<crate::organization::model::OrganizationMember>>,
    pub http_client: reqwest::Client,
    pub storage: Arc<dyn crate::storage::ObjectStorage + Send + Sync>,
    pub organization_persist_sender:
        mpsc::Sender<Vec<crate::organization::model::OrganizationMember>>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok(); // Load .env file
        let supabase_config = crate::storage::SupabaseConfig::from_env()?;
        Self::new_with_config(supabase_config).await
    }

    pub async fn new_with_config(
        supabase_config: crate::storage::SupabaseConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        let database_url =
            env::var("SUPABASE_DATABASE_URL").expect("SUPABASE_DATABASE_URL must be set");

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

        let organization_cache = Cache::builder()
            .time_to_live(Duration::from_secs(10 * 60))
            .max_capacity(10)
            .build();

        let http_client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(900))
            .user_agent("cakung-barat-server/1.0")
            .build()
            .expect("Failed to create reqwest client");

        let storage = Arc::new(crate::storage::SupabaseStorage::new(
            supabase_config,
            http_client.clone(),
        ));

        // Create channel for organization persistence worker
        let (organization_persist_sender, receiver) = mpsc::channel(100);

        // Spawn background persistence worker
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            crate::organization::persistence::start_persistence_worker(receiver, storage_clone)
                .await;
        });

        Ok(AppState {
            pool,
            post_cache,
            organization_cache,
            http_client,
            storage,
            organization_persist_sender,
        })
    }

    pub async fn new_with_pool_and_storage(
        pool: sqlx::PgPool,
        storage: Arc<dyn crate::storage::ObjectStorage + Send + Sync>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let post_cache = Cache::builder()
            .time_to_live(Duration::from_secs(10 * 60))
            .max_capacity(100)
            .build();

        let organization_cache = Cache::builder()
            .time_to_live(Duration::from_secs(10 * 60))
            .max_capacity(10)
            .build();

        let http_client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(900))
            .user_agent("cakung-barat-server/1.0")
            .build()
            .expect("Failed to create reqwest client");

        // Create channel for organization persistence worker
        let (organization_persist_sender, receiver) = mpsc::channel(100);

        // Spawn background persistence worker
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            crate::organization::persistence::start_persistence_worker(receiver, storage_clone)
                .await;
        });

        Ok(AppState {
            pool,
            post_cache,
            organization_cache,
            http_client,
            storage,
            organization_persist_sender,
        })
    }
}

use sanitize_filename::sanitize;
use reqwest;
use serde_json::Value;
use log;
use mime_guess;

#[derive(serde::Serialize, serde::Deserialize, Debug, utoipa::ToSchema)]
pub struct FolderContent {
    pub name: String,
    pub is_file: bool,
    pub size: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub bucket_name: String,
}

impl SupabaseConfig {
    pub fn from_env() -> Result<Self, String> {
        log::debug!("Loading Supabase configuration from environment");
        let supabase_url = std::env::var("SUPABASE_URL")
            .map_err(|_| "SUPABASE_URL must be set".to_string())?;
        let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY")
            .map_err(|_| "SUPABASE_ANON_KEY must be set".to_string())?;
        let bucket_name = std::env::var("BUCKET_NAME")
            .unwrap_or_else(|_| "cakung-barat-supabase-bucket".to_string());

        log::debug!("Supabase configuration loaded successfully for bucket: {}", bucket_name);
        Ok(SupabaseConfig { supabase_url, supabase_anon_key, bucket_name })
    }
}

#[async_trait::async_trait]
pub trait ObjectStorage {
    async fn upload_file(&self, filename: &str, file_data: &[u8]) -> Result<(), String>;
    async fn delete_file(&self, filename: &str) -> Result<(), String>;
    async fn create_folder(&self, folder_name: &str) -> Result<(), String>;
    async fn list_folder_contents(&self, folder_name: &str) -> Result<Vec<FolderContent>, String>;
    fn get_asset_url(&self, filename: &str) -> String;
}

pub struct SupabaseStorage {
    pub config: SupabaseConfig,
    pub client: reqwest::Client,
}

impl SupabaseStorage {
    pub fn new(config: SupabaseConfig, client: reqwest::Client) -> Self {
        Self { config, client }
    }
}

#[async_trait::async_trait]
impl ObjectStorage for SupabaseStorage {
    async fn upload_file(&self, filename: &str, file_data: &[u8]) -> Result<(), String> {
        upload_file_to_supabase(filename, file_data, &self.client, &self.config).await
    }

    async fn delete_file(&self, filename: &str) -> Result<(), String> {
        delete_asset_file(filename, &self.client, &self.config).await
    }

    async fn create_folder(&self, folder_name: &str) -> Result<(), String> {
        create_folder(folder_name, &self.client, &self.config).await
    }

    async fn list_folder_contents(&self, folder_name: &str) -> Result<Vec<FolderContent>, String> {
        list_folder_contents(folder_name, &self.client, &self.config).await
    }

    fn get_asset_url(&self, filename: &str) -> String {
        get_supabase_asset_url(filename, &self.config)
    }
}



pub async fn upload_file_to_supabase(filename: &str, file_data: &[u8], client: &reqwest::Client, config: &SupabaseConfig) -> Result<(), String> {
    log::info!("Attempting to upload asset file to Supabase storage: {}", filename);
    log::debug!("Uploading file data to Supabase storage: {}", filename);

    let upload_url = format!("{}/storage/v1/object/{}/{}", config.supabase_url, config.bucket_name, filename);
    log::debug!("Supabase upload URL: {}", upload_url);

    // Determine content type based on file extension for better compatibility
    let content_type = mime_guess::from_path(filename).first_or_octet_stream().to_string();

    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", config.supabase_anon_key))
        .header("apikey", &config.supabase_anon_key)
        .header("Content-Type", content_type) // Use appropriate content type based on file extension
        .body(file_data.to_vec())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        log::info!("Successfully uploaded asset file to Supabase storage: {}", filename);
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        log::error!("Upload failed for file {} with status: {}: {}", filename, status, error_text);
        Err(format!("Upload failed with status: {}", status))
    }
}

pub async fn delete_asset_file(filename: &str, client: &reqwest::Client, config: &SupabaseConfig) -> Result<(), String> {
    log::info!("Attempting to delete asset file from Supabase storage: {}", filename);

    let delete_url = format!("{}/storage/v1/object/{}/{}", config.supabase_url, config.bucket_name, filename);
    log::debug!("Supabase delete URL: {}", delete_url);

    let response = client
        .delete(&delete_url)
        .header("Authorization", format!("Bearer {}", config.supabase_anon_key))
        .header("apikey", &config.supabase_anon_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        log::info!("Successfully deleted asset file from Supabase storage: {}", filename);
        Ok(())
    } else {
        log::error!("Delete failed for file {} with status: {}", filename, response.status());
        Err(format!("Delete failed with status: {}", response.status()))
    }
}

pub fn get_supabase_asset_url(filename: &str, config: &SupabaseConfig) -> String {
    log::debug!("Generating Supabase asset URL for file: {}", filename);
    let url = format!("{}/storage/v1/object/public/{}/{}", config.supabase_url, config.bucket_name, filename);
    log::debug!("Generated Supabase asset URL: {}", url);
    url
}

pub async fn create_folder(folder_name: &str, client: &reqwest::Client, config: &SupabaseConfig) -> Result<(), String> {
    log::info!("Attempting to create folder in Supabase storage: {}", folder_name);

    let placeholder_filename = format!("{}/placeholder.txt", sanitize(folder_name));
    let placeholder_data = b"Folder placeholder";
    log::debug!("Creating folder with placeholder file: {}", placeholder_filename);

    let upload_url = format!("{}/storage/v1/object/{}/{}", config.supabase_url, config.bucket_name, placeholder_filename);
    log::debug!("Supabase folder creation URL: {}", upload_url);

    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", config.supabase_anon_key))
        .header("apikey", &config.supabase_anon_key)
        .body(placeholder_data.to_vec())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        log::info!("Successfully created folder in Supabase storage: {}", folder_name);
        Ok(())
    } else {
        log::error!("Folder creation failed for {} with status: {}", folder_name, response.status());
        Err(format!("Folder creation failed with status: {}", response.status()))
    }
}

#[allow(dead_code)]
pub async fn list_folder_contents(folder_name: &str, client: &reqwest::Client, config: &SupabaseConfig) -> Result<Vec<FolderContent>, String> {
    log::info!("Attempting to list contents of folder in Supabase storage: {}", folder_name);

    let list_url = format!("{}/storage/v1/object/list/{}", config.supabase_url, config.bucket_name);
    log::debug!("Supabase list URL: {}", list_url);

    let body = serde_json::json!({
        "prefix": folder_name,
        "limit": 100
    });

    let response = client
        .post(&list_url)
        .header("Authorization", format!("Bearer {}", config.supabase_anon_key))
        .header("apikey", &config.supabase_anon_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        log::info!("Successfully retrieved folder contents from Supabase storage: {}", folder_name);
        let response_text = response.text().await.map_err(|e| e.to_string())?;
        let files: Vec<Value> = serde_json::from_str(&response_text).map_err(|e| e.to_string())?;
        log::debug!("Found {} files in folder: {}", files.len(), folder_name);

        let mut contents = Vec::new();
        for file in files {
            if let Some(name) = file.get("name") {
                let is_file = file.get("id").is_some();
                let size = file.get("metadata").and_then(|m| m.get("size")).and_then(|s| s.as_u64());

                contents.push(FolderContent {
                    name: name.as_str().unwrap_or("").to_string(),
                    is_file,
                    size,
                });
            }
        }

        log::info!("Successfully listed {} items from folder: {}", contents.len(), folder_name);
        Ok(contents)
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        log::error!("List folder contents failed for {} with status: {}", folder_name, status);
        Err(format!("List failed with status {}: {}", status, error_text))
    }
}
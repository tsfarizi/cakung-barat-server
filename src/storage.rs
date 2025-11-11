use actix_multipart::Multipart;
use futures::TryStreamExt;
use sanitize_filename::sanitize;
use std::path::Path;
use uuid::Uuid;
use reqwest;
use serde_json::Value;
use log;

#[derive(serde::Serialize, serde::Deserialize, Debug, utoipa::ToSchema)]
pub struct FolderContent {
    pub name: String,
    pub is_file: bool,
    pub size: Option<u64>,
}

use dotenvy::dotenv;

fn get_supabase_config() -> (String, String, String) {
    log::debug!("Loading Supabase configuration");
    dotenv().ok();
    let supabase_url = std::env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be set");
    let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY")
        .expect("SUPABASE_ANON_KEY must be set");
    let bucket_name = std::env::var("BUCKET_NAME")
        .unwrap_or_else(|_| "cakung-barat-supabase-bucket".to_string());
    
    log::debug!("Supabase configuration loaded successfully for bucket: {}", bucket_name);
    (supabase_url, supabase_anon_key, bucket_name)
}

fn create_client() -> reqwest::Client {
    log::debug!("Creating reqwest HTTP client");
    let tls_verification = std::env::var("TLS_VERIFY")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);

    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent("cakung-barat-server/1.0");

    if !tls_verification {
        log::warn!("TLS verification is disabled for HTTP client");
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder
        .build()
        .expect("Failed to create reqwest client")
}

pub async fn save_file(
    mut payload: Multipart,
) -> Result<(String, Option<Uuid>, Vec<String>, Option<String>), String> {
    let mut _file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut posting_id: Option<Uuid> = None;
    let mut folder_names: Vec<String> = Vec::new();
    let mut asset_name: Option<String> = None;

    while let Some(mut field) = payload.try_next().await.map_err(|e| e.to_string())? {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition
            .get_name()
            .ok_or_else(|| "No field name".to_string())?;

        match field_name {
            "file" => {

                let file_name = content_disposition.get_filename().ok_or_else(|| "No filename".to_string())?;
                let sanitized_filename = sanitize(&file_name);
                
                let ext = Path::new(&sanitized_filename)
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("");
                

                let unique_filename = format!("{}_{}.{}", Uuid::new_v4(), sanitized_filename.replace(".", "_"), ext);

                let mut field_data = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| e.to_string())? {
                    field_data.extend_from_slice(&chunk);
                }


                upload_to_supabase_storage(&unique_filename, &field_data, &ext).await?;
                
                filename = Some(unique_filename);
            }
            "posting_id" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| e.to_string())? {
                    bytes.extend_from_slice(&chunk);
                }
                let value = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                posting_id = Uuid::parse_str(&value).ok();
            }
            "folders" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| e.to_string())? {
                    bytes.extend_from_slice(&chunk);
                }
                let value = String::from_utf8(bytes).map_err(|e| e.to_string())?;

                folder_names = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "name" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| e.to_string())? {
                    bytes.extend_from_slice(&chunk);
                }
                let value = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                asset_name = Some(value);
            }
            _ => {

                continue;
            }
        }
    }

    match filename {
        Some(name) => Ok((name, posting_id, folder_names, asset_name)),
        None => Err("No file was uploaded".to_string()),
    }
}

async fn upload_to_supabase_storage(filename: &str, data: &[u8], _file_ext: &str) -> Result<(), String> {
    log::info!("Attempting to upload asset file to Supabase storage: {}", filename);
    log::debug!("Uploading {} bytes to Supabase storage: {}", data.len(), filename);
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    let upload_url = format!("{}/storage/v1/object/{}/{}", supabase_url, bucket_name, filename);
    log::debug!("Supabase upload URL: {}", upload_url);
    
    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        log::info!("Successfully uploaded asset file to Supabase storage: {}", filename);
        Ok(())
    } else {
        log::error!("Upload failed for file {} with status: {}", filename, response.status());
        Err(format!("Upload failed with status: {}", response.status()))
    }
}

pub async fn delete_asset_file(filename: &str) -> Result<(), String> {
    log::info!("Attempting to delete asset file from Supabase storage: {}", filename);
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    let delete_url = format!("{}/storage/v1/object/{}/{}", supabase_url, bucket_name, filename);
    log::debug!("Supabase delete URL: {}", delete_url);
    
    let response = client
        .delete(&delete_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
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

pub fn get_supabase_asset_url(filename: &str) -> String {
    log::debug!("Generating Supabase asset URL for file: {}", filename);
    let (supabase_url, _, bucket_name) = get_supabase_config();
    let url = format!("{}/storage/v1/object/public/{}/{}", supabase_url, bucket_name, filename);
    log::debug!("Generated Supabase asset URL: {}", url);
    url
}

pub async fn create_folder(folder_name: &str) -> Result<(), String> {
    log::info!("Attempting to create folder in Supabase storage: {}", folder_name);

    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    let placeholder_filename = format!("{}/placeholder.txt", sanitize(folder_name));
    let placeholder_data = b"Folder placeholder";
    log::debug!("Creating folder with placeholder file: {}", placeholder_filename);
    
    let upload_url = format!("{}/storage/v1/object/{}/{}", supabase_url, bucket_name, placeholder_filename);
    log::debug!("Supabase folder creation URL: {}", upload_url);
    
    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
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
pub async fn list_folder_contents(folder_name: &str) -> Result<Vec<FolderContent>, String> {
    log::info!("Attempting to list contents of folder in Supabase storage: {}", folder_name);
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    

    let list_url = format!("{}/storage/v1/object/list/{}", supabase_url, bucket_name);
    log::debug!("Supabase list URL: {}", list_url);
    
    let body = serde_json::json!({
        "prefix": folder_name,
        "limit": 100
    });
    
    let response = client
        .post(&list_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
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
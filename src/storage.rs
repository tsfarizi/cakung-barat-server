use actix_multipart::Multipart;
use futures::TryStreamExt;
use sanitize_filename::sanitize;
use std::path::Path;
use uuid::Uuid;
use reqwest;
use serde_json::Value;
use std::sync::Once;

#[derive(serde::Serialize, serde::Deserialize, Debug, utoipa::ToSchema)]
pub struct FolderContent {
    pub name: String,
    pub is_file: bool,
    pub size: Option<u64>,
}

use dotenvy::dotenv;

// Supabase storage configuration
fn get_supabase_config() -> (String, String, String) {
    dotenv().ok();
    let supabase_url = std::env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be set");
    let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY")
        .expect("SUPABASE_ANON_KEY must be set");
    let bucket_name = std::env::var("BUCKET_NAME")
        .unwrap_or_else(|_| "cakung-barat-supabase-bucket".to_string());
    
    (supabase_url, supabase_anon_key, bucket_name)
}

static INIT: Once = Once::new();

// Create a reqwest client with proper TLS configuration
fn create_client() -> reqwest::Client {
    INIT.call_once(|| {
        // Initialize openssl to load system certificates on Windows
        #[cfg(target_os = "windows")]
        {
            unsafe {
                openssl_probe::init_openssl_env_vars();
            }
        }
    });

    // Check if we're in production or development to decide on TLS verification
    let tls_verification = std::env::var("TLS_VERIFY")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);

    let mut builder = reqwest::Client::builder()
        .user_agent("cakung-barat-server/1.0");

    if !tls_verification {
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
                // Process the uploaded file
                let file_name = content_disposition.get_filename().ok_or_else(|| "No filename".to_string())?;
                let sanitized_filename = sanitize(&file_name);
                
                let ext = Path::new(&sanitized_filename)
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("");
                
                // Generate unique filename
                let unique_filename = format!("{}_{}.{}", Uuid::new_v4(), sanitized_filename.replace(".", "_"), ext);

                let mut field_data = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| e.to_string())? {
                    field_data.extend_from_slice(&chunk);
                }

                // Upload to Supabase storage
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
                // Parse comma-separated folder names
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
                // Skip any unexpected fields
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
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    // Construct the upload URL
    let upload_url = format!("{}/storage/v1/object/{}/{}", supabase_url, bucket_name, filename);
    
    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Upload failed with status: {}", response.status()))
    }
}

pub async fn delete_asset_file(filename: &str) -> Result<(), String> {
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    // Construct the delete URL
    let delete_url = format!("{}/storage/v1/object/{}/{}", supabase_url, bucket_name, filename);
    
    let response = client
        .delete(&delete_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Delete failed with status: {}", response.status()))
    }
}

pub fn get_supabase_asset_url(filename: &str) -> String {
    let (supabase_url, _, bucket_name) = get_supabase_config();
    format!("{}/storage/v1/object/public/{}/{}", supabase_url, bucket_name, filename)
}

pub async fn create_folder(folder_name: &str) -> Result<(), String> {
    // Supabase storage doesn't have traditional folders, but we can create a placeholder file
    // to represent a folder conceptually
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    let placeholder_filename = format!("{}/placeholder.txt", sanitize(folder_name));
    let placeholder_data = b"Folder placeholder";
    
    let upload_url = format!("{}/storage/v1/object/{}/{}", supabase_url, bucket_name, placeholder_filename);
    
    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
        .body(placeholder_data.to_vec())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Folder creation failed with status: {}", response.status()))
    }
}

pub async fn list_folder_contents(folder_name: &str) -> Result<Vec<FolderContent>, String> {
    let client = create_client();
    let (supabase_url, supabase_anon_key, bucket_name) = get_supabase_config();
    
    // Construct the list URL - get all files in a "folder" by prefix
    let list_url = format!("{}/storage/v1/object/list/{}", supabase_url, bucket_name);
    
    // Using query parameters to filter by folder prefix
    let params = [("prefix", folder_name)];
    
    let response = client
        .post(&list_url)
        .header("Authorization", format!("Bearer {}", supabase_anon_key))
        .header("apikey", &supabase_anon_key)
        .json(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let response_text = response.text().await.map_err(|e| e.to_string())?;
        let files: Vec<Value> = serde_json::from_str(&response_text).map_err(|e| e.to_string())?;
        
        let mut contents = Vec::new();
        for file in files {
            if let (Some(name), Some(size)) = (file.get("name"), file.get("size")) {
                contents.push(FolderContent {
                    name: name.as_str().unwrap_or("").to_string(),
                    is_file: true, // In Supabase storage, everything is a file
                    size: size.as_u64(),
                });
            }
        }
        
        Ok(contents)
    } else {
        Err(format!("List failed with status: {}", response.status()))
    }
}
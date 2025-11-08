use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use actix_web::web;
use futures_util::stream::StreamExt;
use log::{debug, info, error};
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

const ASSETS_DIR: &str = "./assets";

#[derive(Serialize, Deserialize, ToSchema)]
pub struct FolderContent {
    #[schema(example = "image.png")]
    pub name: String,
    #[schema(example = "/assets/uploads/image.png")]
    pub path: String,
    #[schema(example = "false")]
    pub is_dir: bool,
}

pub fn create_folder(folder_name: &str) -> std::io::Result<()> {
    let folder_path = Path::new(ASSETS_DIR).join(folder_name);
    info!("Creating folder at: {:?}", folder_path);
    fs::create_dir_all(folder_path)
}

pub fn list_folder_contents(folder_name: &str) -> std::io::Result<Vec<FolderContent>> {
    let folder_path = Path::new(ASSETS_DIR).join(folder_name);
    debug!("Listing contents of folder: {:?}", folder_path);
    let mut contents = Vec::new();

    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().into_string().unwrap_or_default();
        let is_dir = path.is_dir();
        let relative_path = path.strip_prefix(ASSETS_DIR).unwrap().to_str().unwrap_or_default();

        contents.push(FolderContent {
            name,
            path: format!("/assets/{}", relative_path),
            is_dir,
        });
    }

    Ok(contents)
}

pub async fn save_file(
    mut payload: actix_multipart::Multipart,
) -> Result<(String, Option<Uuid>, Vec<String>), String> {
    let mut filename: Option<String> = None;
    let mut posting_id: Option<Uuid> = None;
    let mut folders: Vec<String> = Vec::new();
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| e.to_string())?;
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or_default();

        match field_name {
            "file" => {
                let original_filename = content_disposition
                    .get_filename()
                    .ok_or("File name not found in Content-Disposition")?;
                let new_filename = format!(
                    "{}-{}",
                    Uuid::new_v4(),
                    sanitize(original_filename)
                );
                filename = Some(new_filename);

                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    bytes.extend_from_slice(&chunk.unwrap());
                }
                file_data = Some(bytes);
            },
            "posting_id" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    bytes.extend_from_slice(&chunk.unwrap());
                }
                if let Ok(id_str) = String::from_utf8(bytes) {
                    if let Ok(id) = Uuid::parse_str(&id_str) {
                        posting_id = Some(id);
                    }
                }
            },
            "folders" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    bytes.extend_from_slice(&chunk.unwrap());
                }
                if let Ok(folder_str) = String::from_utf8(bytes) {
                    folders.push(folder_str);
                }
            }
            _ => (),
        }
    }

    if let (Some(fname), Some(data)) = (filename, file_data) {
        let file_path = Path::new(ASSETS_DIR).join(&fname);
        info!("Saving file to: {:?}", file_path);
        let mut f = web::block(move || fs::File::create(file_path)).await.unwrap().unwrap();
        web::block(move || f.write_all(&data)).await.unwrap().unwrap();
        Ok((fname, posting_id, folders))
    } else {
        error!("Invalid multipart payload. File data or filename missing.");
        Err("Invalid multipart payload".to_string())
    }
}

pub fn get_asset_path(filename: &str) -> PathBuf {
    PathBuf::from(ASSETS_DIR).join(filename)
}

pub fn delete_asset_file(filename: &str) -> std::io::Result<()> {
    let file_path = get_asset_path(filename);
    info!("Deleting file at: {:?}", file_path);
    fs::remove_file(file_path)
}

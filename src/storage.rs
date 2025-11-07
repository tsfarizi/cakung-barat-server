use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use actix_web::web;
use futures_util::stream::StreamExt;
use sanitize_filename::sanitize;
use uuid::Uuid;

const ASSETS_DIR: &str = "./assets";

pub async fn save_file(
    mut payload: actix_multipart::Multipart,
) -> Result<(String, Uuid), String> {
    let mut filename: Option<String> = None;
    let mut posting_id: Option<Uuid> = None;

    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| e.to_string())?;
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or_default();

        if field_name == "file" {
            let original_filename = content_disposition
                .get_filename()
                .ok_or("File name not found in Content-Disposition")?;
            let new_filename = format!(
                "{}-{}",
                Uuid::new_v4(),
                sanitize(original_filename)
            );
            let file_path = Path::new(ASSETS_DIR).join(&new_filename);
            let mut f = web::block(move || fs::File::create(file_path)).await.unwrap().unwrap();
            while let Some(chunk) = field.next().await {
                let data = chunk.unwrap();
                f = web::block(move || f.write_all(&data).map(|_| f)).await.unwrap().unwrap();
            }
            filename = Some(new_filename);
        } else if field_name == "posting_id" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                bytes.extend_from_slice(&chunk.unwrap());
            }
            if let Ok(id_str) = String::from_utf8(bytes) {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    posting_id = Some(id);
                }
            }
        }
    }

    if let (Some(fname), Some(pid)) = (filename, posting_id) {
        Ok((fname, pid))
    } else {
        Err("Invalid multipart payload".to_string())
    }
}

pub fn get_asset_path(filename: &str) -> PathBuf {
    Path::new(ASSETS_DIR).join(filename)
}

pub fn delete_asset_file(filename: &str) -> std::io::Result<()> {
    let file_path = get_asset_path(filename);
    fs::remove_file(file_path)
}

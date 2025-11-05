use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use actix_web::web;
use futures_util::stream::StreamExt;
use uuid::Uuid;

const ASSETS_DIR: &str = "./assets";

pub async fn save_file(
    mut payload: actix_multipart::Multipart,
) -> Result<(String, String), String> {
    let mut field = payload.next().await.ok_or("Multipart payload is empty")??;

    let content_disposition = field.content_disposition();
    let original_filename = content_disposition
        .get_filename()
        .ok_or("File name not found in Content-Disposition")?;

    let new_filename = format!(
        "{}",
        sanitize_filename::sanitize(original_filename)
    );
    let file_path = Path::new(ASSETS_DIR).join(&new_filename);

    let mut f = web::block(|| fs::File::create(&file_path)).await??;

    while let Some(chunk) = field.next().await {
        let data = chunk?;
        f = web::block(move || f.write_all(&data).map(|_| f)).await??;
    }

    Ok((new_filename, original_filename.to_string()))
}

pub fn get_asset_path(filename: &str) -> PathBuf {
    Path::new(ASSETS_DIR).join(filename)
}

pub fn delete_asset_file(filename: &str) -> std::io::Result<()> {
    let file_path = get_asset_path(filename);
    fs::remove_file(file_path)
}

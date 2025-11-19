use actix_multipart::Multipart;
use actix_web::{
    HttpResponse, Responder,
    web::{self, Json, Path},
};
use futures::StreamExt;
use log::{debug, error, info};
use serde::Serialize;
use utoipa::ToSchema;
use tempfile::NamedTempFile;
use std::io::Write;
use sanitize_filename::sanitize;
use std::path::Path as StdPath;
use futures::TryStreamExt;
use std::sync::Arc;
use crate::ErrorResponse;
use crate::{asset::models::Asset, db::AppState};
use uuid::Uuid;

async fn multipart_save_with_storage_trait(
    mut payload: actix_multipart::Multipart,
    storage: &Arc<dyn crate::storage::ObjectStorage + Send + Sync>,
) -> Result<(String, Option<Uuid>, Vec<String>, Option<String>), String> {
    let mut filename: Option<String> = None;
    let mut posting_id: Option<Uuid> = None;
    let mut folder_names: Vec<String> = Vec::new();
    let mut asset_name: Option<String> = None;

    while let Some(mut field) = payload.try_next().await.map_err(|e| e.to_string())? {
        let content_disposition = field.content_disposition().ok_or("Content-Disposition not set")?;
        let field_name = content_disposition
            .get_name()
            .ok_or_else(|| "No field name".to_string())?;

        match field_name {
            "file" => {
                let file_name = content_disposition.get_filename().ok_or_else(|| "No filename".to_string())?;
                let sanitized_filename = sanitize(&file_name);

                let ext = StdPath::new(&sanitized_filename)
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("");

                let unique_filename = format!("{}_{}.{}", Uuid::new_v4(), sanitized_filename.replace(".", "_"), ext);

                let mut temp_file = NamedTempFile::new()
                    .map_err(|e| format!("Failed to create temporary file: {}", e))?;

                while let Some(chunk) = field.try_next().await.map_err(|e| e.to_string())? {
                    temp_file.write_all(&chunk)
                        .map_err(|e| format!("Failed to write chunk to temp file: {}", e))?;
                }

                let file_data = std::fs::read(temp_file.path()).map_err(|e| format!("Failed to read temp file: {}", e))?;
                storage.upload_file(&unique_filename, &file_data).await?;

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


#[derive(Serialize, ToSchema)]
pub struct FolderWithAssets {
    pub name: String,
    pub assets: Vec<Asset>,
}

#[derive(Serialize, ToSchema)]
pub struct AllAssetsResponse {
    pub folders: Vec<FolderWithAssets>,
}



#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    post,
    path = "/assets",
    request_body(content = inline(UploadAssetRequest), content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Asset created successfully", body = Asset),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Posting not found for asset", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn upload_asset(payload: Multipart, data: web::Data<AppState>) -> impl Responder {
    info!("Executing upload_asset handler");
    debug!("Attempting to save file from multipart payload.");
    match multipart_save_with_storage_trait(payload, &data.storage).await {
        Ok((filename, posting_id_opt, folder_names, asset_name)) => {
            info!("File saved successfully with filename: {}", filename);
            let name = asset_name.unwrap_or_else(|| filename.clone());
            let new_asset = Asset::new(
                name,
                filename.clone(),
                format!("/assets/serve/{}", filename),
                None,
            );

            debug!("Attempting to insert new asset into 'assets' table.");
            if let Err(e) = data.insert_asset(&new_asset).await {
                error!("Failed to insert asset into db: {}", e);
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to save asset"));
            }
            info!("Asset {:?} created and stored in database.", new_asset.id);

            let mut processed_folder_names = Vec::new();
            if folder_names.is_empty() {
                processed_folder_names.push("others".to_string());
            } else {
                for folder_name in folder_names {
                    if folder_name.is_empty() {
                        processed_folder_names.push("others".to_string());
                    } else {
                        processed_folder_names.push(folder_name);
                    }
                }
            }
            let unique_folder_names: Vec<String> = processed_folder_names
                .into_iter()
                .collect::<std::collections::HashSet<String>>()
                .into_iter()
                .collect();

            for folder_name in unique_folder_names {
                debug!(
                    "Associating asset {:?} with folder '{}'",
                    new_asset.id, folder_name
                );
                let folder_contents_result = data.get_folder_contents(&folder_name).await;
                let mut asset_ids = match folder_contents_result {
                    Ok(Some(ids)) => ids,
                    Ok(None) => Vec::new(),
                    Err(e) => {
                        error!("Database error when getting folder contents: {}", e);
                        return HttpResponse::InternalServerError()
                            .json(ErrorResponse::internal_error("Failed to retrieve folder contents"));
                    }
                };
                asset_ids.push(new_asset.id);
                if let Err(e) = data.insert_folder_contents(&folder_name, &asset_ids).await {
                    error!("Failed to associate asset with folder: {}", e);
                } else {
                    info!(
                        "Asset {:?} successfully associated with folder '{}'",
                        new_asset.id, folder_name
                    );
                }
            }

            if let Some(posting_id) = posting_id_opt {
                debug!(
                    "Associating asset {:?} with posting '{:?}'",
                    new_asset.id, posting_id
                );
                match data.get_posting_by_id_with_assets(&posting_id).await {
                    Ok(Some(mut posting)) => {
                        posting.asset_ids.push(new_asset.id);
                        if let Err(e) = data.upsert_posting_with_assets(&posting).await {
                            error!(
                                "Failed to update posting {} with new asset {}: {}",
                                posting.id, new_asset.id, e
                            );
                        } else {
                            info!(
                                "Asset {:?} successfully associated with posting '{:?}'",
                                new_asset.id, posting_id
                            );
                        }
                    }
                    Ok(None) => {
                        error!(
                            "Posting not found for asset association: posting_id='{:?}'",
                            posting_id
                        );
                    }
                    Err(e) => {
                        error!("Database error when fetching posting: {}", e);
                    }
                }
            }

            HttpResponse::Created().json(new_asset)
        }
        Err(e) => {
            error!("Failed during file upload process: {}", e);
            HttpResponse::BadRequest().json(ErrorResponse::bad_request(&e))
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    delete,
    path = "/assets/{id}",
    responses(
        (status = 204, description = "Asset deleted successfully"),
        (status = 404, description = "Asset not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the asset to delete")
    )
)]
pub async fn delete_asset(id: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let asset_id_to_delete = id.into_inner();
    delete_asset_by_id(asset_id_to_delete, data).await
}

async fn delete_asset_by_id(asset_id_to_delete: Uuid, data: web::Data<AppState>) -> impl Responder {
    info!(
        "Executing delete_asset handler for ID: {:?}",
        asset_id_to_delete
    );

    debug!(
        "Attempting to fetch asset with ID {:?} for deletion.",
        asset_id_to_delete
    );
    match data.get_asset_by_id(&asset_id_to_delete).await {
        Ok(Some(asset)) => {
            info!("Found asset {:?} to delete.", asset_id_to_delete);
            debug!(
                "Attempting to delete physical asset file: {}",
                &asset.filename
            );
            if let Err(e) = data.storage.delete_file(&asset.filename).await {
                error!(
                    "Failed to delete physical asset file {}: {}.",
                    asset.filename, e
                );
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to delete asset file"));
            }
            info!("Physical file {} deleted successfully.", asset.filename);

            debug!(
                "Attempting to delete asset record {:?} from 'assets' table.",
                asset_id_to_delete
            );
            if let Err(e) = data.delete_asset(&asset_id_to_delete).await {
                error!(
                    "Failed to delete asset from db, but file was deleted: {}",
                    e
                );
            }

            debug!(
                "Scanning postings to disassociate asset {:?}",
                asset_id_to_delete
            );
            if let Ok(postings) = data.get_all_postings_with_assets().await {
                for mut posting in postings {
                    if posting.asset_ids.contains(&asset_id_to_delete) {
                        debug!(
                            "Disassociating asset {:?} from posting {:?}",
                            asset_id_to_delete, posting.id
                        );
                        posting.asset_ids.retain(|id| *id != asset_id_to_delete);
                        if let Err(e) = data.upsert_posting_with_assets(&posting).await {
                            error!("Failed to update posting after disassociating asset: {}", e);
                        }
                    }
                }
            }

            debug!(
                "Scanning folders to disassociate asset {:?}",
                asset_id_to_delete
            );

            info!(
                "Asset {:?} deleted successfully from all records.",
                asset_id_to_delete
            );
            HttpResponse::NoContent().finish()
        }
        Ok(None) => {
            error!("Asset not found for deletion: {:?}", asset_id_to_delete);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Asset with ID {:?} not found",
                asset_id_to_delete
            )))
        }
        Err(e) => {
            error!("Failed to retrieve asset for deletion from database: {}", e);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve asset"))
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    get,
    path = "/assets/{id}",
    responses(
        (status = 200, description = "Asset found", body = Asset),
        (status = 404, description = "Asset not found", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the asset to retrieve")
    )
)]
pub async fn get_asset_by_id(id: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let asset_id = id.into_inner();
    info!("Executing get_asset_by_id handler for ID: {:?}", asset_id);
    debug!(
        "Received GET request to /assets/{:?} - this endpoint only supports GET and DELETE methods",
        asset_id
    );
    match data.get_asset_by_id(&asset_id).await {
        Ok(Some(asset)) => {
            info!("Successfully fetched asset with ID: {:?}", asset_id);
            HttpResponse::Ok().json(asset)
        }
        Ok(None) => {
            error!("Asset not found in database for ID: {:?}", asset_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Asset with ID {:?} not found",
                asset_id
            )))
        }
        Err(e) => {
            error!(
                "Failed to get asset by ID '{}' from database: {}",
                asset_id, e
            );
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve asset"))
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    get,
    path = "/assets",
    responses(
        (status = 200, description = "List of all assets, structured by folder", body = AllAssetsResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn get_all_assets_structured(data: web::Data<AppState>) -> impl Responder {
    info!("Executing get_all_assets_structured handler");
    debug!("Fetching all assets structured by folder using optimized SQL query.");

    // Get folder-asset associations efficiently
    let folder_assets_query = r#"
        SELECT
            f.name as folder_name,
            COALESCE(json_agg(
                json_build_object(
                    'id', a.id,
                    'name', a.name,
                    'filename', a.filename,
                    'url', a.url,
                    'description', a.description,
                    'created_at', a.created_at,
                    'updated_at', a.updated_at
                ) ORDER BY a.created_at DESC
            ) FILTER (WHERE a.id IS NOT NULL), '[]'::json) as assets_json
        FROM folders f
        LEFT JOIN asset_folders af ON f.id = af.folder_id
        LEFT JOIN assets a ON af.asset_id = a.id
        GROUP BY f.name
        ORDER BY f.name
    "#;

    #[derive(sqlx::FromRow, serde::Deserialize)]
    struct FolderAssetsRow {
        folder_name: String,
        assets_json: serde_json::Value,
    }

    let folder_results: Result<Vec<FolderAssetsRow>, _> = sqlx::query_as(folder_assets_query)
        .fetch_all(&data.pool)
        .await;

    match folder_results {
        Ok(folder_rows) => {
            let mut folders_with_assets: Vec<FolderWithAssets> = Vec::new();

            for row in folder_rows {
                let assets: Vec<Asset> = if row.assets_json.is_array() {
                    match serde_json::from_value(row.assets_json.clone()) {
                        Ok(assets) => assets,
                        Err(e) => {
                            error!("Failed to parse assets JSON for folder {}: {}", row.folder_name, e);
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                };

                folders_with_assets.push(FolderWithAssets {
                    name: row.folder_name,
                    assets,
                });
            }

            // Get unassigned assets separately
            let unassigned_query = r#"
                SELECT
                    id, name, filename, url, description, created_at, updated_at
                FROM assets
                WHERE id NOT IN (
                    SELECT DISTINCT asset_id
                    FROM asset_folders
                    WHERE asset_id IS NOT NULL
                )
                ORDER BY created_at DESC
            "#;

            let unassigned_assets: Result<Vec<Asset>, _> = sqlx::query_as(unassigned_query)
                .fetch_all(&data.pool)
                .await;

            match unassigned_assets {
                Ok(unassigned) => {
                    if !unassigned.is_empty() {
                        folders_with_assets.push(FolderWithAssets {
                            name: "others".to_string(),
                            assets: unassigned,
                        });
                    }

                    info!("Successfully fetched structured assets: {} folders", folders_with_assets.len());
                    let response = AllAssetsResponse {
                        folders: folders_with_assets,
                    };
                    HttpResponse::Ok().json(response)
                }
                Err(e) => {
                    error!("Failed to fetch unassigned assets: {}", e);
                    HttpResponse::InternalServerError()
                        .json(ErrorResponse::internal_error("Failed to retrieve unassigned assets"))
                }
            }
        }
        Err(e) => {
            error!("Failed to get structured assets from database: {}", e);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve structured assets"))
        }
    }
}


pub async fn serve_asset(req: actix_web::HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let filename: String = req.match_info().query("filename").into();
    info!("Executing serve_asset handler for filename: {}", &filename);

    debug!(
        "Searching for asset with filename '{}' in database.",
        &filename
    );
    match data.get_all_assets().await {
        Ok(assets) => {
            if let Some(asset) = assets.iter().find(|a| a.filename == filename) {
                info!("Asset found for filename: {}. Redirecting to Supabase storage.", &filename);
                let supabase_url = data.storage.get_asset_url(&asset.filename);
                return HttpResponse::TemporaryRedirect()
                    .append_header(("Location", supabase_url))
                    .finish();
            }
        }
        Err(e) => {
            error!(
                "Database error while trying to serve asset '{}': {}",
                &filename, e
            );
        }
    }

    error!("Asset not found for serving: {}", &filename);
    HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
        "Asset '{}' not found",
        filename
    )))
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    post,
    path = "/assets/folders",
    request_body(content = inline(CreateFolderRequest), content_type = "application/json"),
    responses(
        (status = 201, description = "Folder created successfully"),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn create_folder_handler(
    req: Json<CreateFolderRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    info!(
        "Executing create_folder_handler for folder: {}",
        &req.folder_name
    );

    if req.folder_name.is_empty() {
        error!("Folder name cannot be empty.");
        return HttpResponse::BadRequest()
            .json(ErrorResponse::bad_request("Folder name cannot be empty"));
    }

    debug!(
        "Attempting to create folder '{}' in Supabase storage.",
        &req.folder_name
    );
    match data.storage.create_folder(&req.folder_name).await {
        Ok(_) => {
            info!("Folder '{}' created in Supabase storage.", &req.folder_name);
            debug!(
                "Attempting to insert empty folder record '{}' into database.",
                &req.folder_name
            );
            if let Err(e) = data.insert_folder_contents(&req.folder_name, &vec![]).await {
                error!("Failed to create folder record in db: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::internal_error(
                    "Failed to create folder record",
                ));
            }
            info!(
                "Folder record '{}' created successfully in database.",
                &req.folder_name
            );
            HttpResponse::Created().finish()
        }
        Err(e) => {
            error!(
                "Failed to create folder '{}' in Supabase storage: {}",
                &req.folder_name, e
            );
            HttpResponse::BadRequest().json(ErrorResponse::bad_request(&e.to_string()))
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    get,
    path = "/assets/folders/{folder_name}",
    params(
        ("folder_name" = String, Path, description = "Name of the folder to list asset details from")
    ),
    responses(
        (status = 200, description = "A list of assets in the folder", body = Vec<Asset>),
        (status = 404, description = "Folder not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn list_folder_handler(
    folder_name: Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    let folder_name = folder_name.into_inner();
    info!("Executing list_folder_handler for folder: {}", &folder_name);

    if folder_name.is_empty() {
        error!("Folder name cannot be empty.");
        return HttpResponse::BadRequest()
            .json(ErrorResponse::bad_request("Folder name cannot be empty"));
    }

    debug!(
        "Attempting to get asset IDs for folder '{}' from database.",
        &folder_name
    );
    match data.get_folder_contents(&folder_name).await {
        Ok(Some(asset_ids)) => {
            let mut assets = Vec::new();
            for asset_id in asset_ids {
                match data.get_asset_by_id(&asset_id).await {
                    Ok(Some(asset)) => assets.push(asset),
                    Ok(None) => {
                        error!("Asset with ID {} found in folder but not in assets table.", asset_id);
                    }
                    Err(e) => {
                        error!("Failed to fetch asset {}: {}", asset_id, e);
                        return HttpResponse::InternalServerError().json(
                            ErrorResponse::internal_error("Failed to retrieve asset details"),
                        );
                    }
                }
            }
            info!(
                "Successfully fetched {} assets for folder '{}'",
                assets.len(),
                &folder_name
            );
            HttpResponse::Ok().json(assets)
        }
        Ok(None) => {
            error!("Folder not found in database: {}", &folder_name);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Folder '{}' not found",
                folder_name
            )))
        }
        Err(e) => {
            error!(
                "Failed to get folder contents for '{}': {}",
                &folder_name, e
            );
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve folder contents"))
        }
    }
}



#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadAssetRequest {
    #[allow(unused)]
    pub file: Vec<u8>,
    #[allow(unused)]
    pub posting_id: Option<Uuid>,
    #[allow(unused)]
    pub folders: Option<Vec<String>>,
    #[allow(unused)]
    pub name: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateFolderRequest {
    pub folder_name: String,
}


#[allow(dead_code)]
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CreateFolderForm {
    folder_name: String,
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    post,
    path = "/assets/by-ids",
    request_body(content = inline(GetAssetsByIdsRequest), content_type = "application/json"),
    responses(
        (status = 200, description = "List of assets found", body = Vec<Asset>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn get_assets_by_ids(
    req: web::Json<GetAssetsByIdsRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    info!("Executing get_assets_by_ids handler");
    debug!("Request received with {} IDs: {:?}", req.ids.len(), req.ids);

    // Check for duplicate IDs and log a warning
    let unique_ids: std::collections::HashSet<_> = req.ids.iter().collect();
    if unique_ids.len() != req.ids.len() {
        debug!("Duplicate IDs detected in request");
    }

    // Log the actual IDs being processed for debugging
    for (index, id) in req.ids.iter().enumerate() {
        debug!("Processing ID[{}]: {}", index, id);
    }

    debug!("Attempting to fetch assets for provided IDs from database.");
    match data.get_assets_by_ids(&req.ids).await {
        Ok(assets) => {
            info!("Successfully fetched {} assets out of {} requested IDs", assets.len(), req.ids.len());
            
            // Log details about the fetched assets
            for (index, asset) in assets.iter().enumerate() {
                debug!("Fetched asset[{}]: ID={}, filename='{}'", index, asset.id, asset.filename);
            }
            
            HttpResponse::Ok().json(assets)
        }
        Err(e) => {
            error!("Failed to fetch assets by IDs: {}", e);
            error!("Error details - Requested IDs: {:?}, Error: {}", req.ids, e);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve assets"))
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct GetAssetsByIdsRequest {
    pub ids: Vec<Uuid>,
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    post,
    path = "/assets/posts/{post_id}",
    request_body(content = inline(UploadAssetRequest), content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Asset uploaded to post successfully", body = Asset),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("post_id" = Uuid, Path, description = "ID of the post to upload assets to")
    )
)]
pub async fn upload_asset_to_post(
    path: Path<Uuid>,
    payload: Multipart,
    data: web::Data<AppState>,
) -> impl Responder {
    let post_id = path.into_inner();
    info!("Executing upload_asset_to_post handler for post ID: {}", post_id);

    // First, check if the post exists
    match data.get_post_by_id(&post_id).await {
        Ok(Some(post)) => {
            // Get or create the folder for this post
            let folder_id = match &post.folder_id {
                Some(folder_id) => folder_id.clone(),
                None => {
                    // Create a new folder for this post if it doesn't have one
                    let new_folder_id = format!("posts/{}", post_id);

                    // Create folder in storage
                    if let Err(e) = data.storage.create_folder(&new_folder_id).await {
                        error!("Failed to create folder for post {}: {}", post_id, e);
                        return HttpResponse::InternalServerError()
                            .json(ErrorResponse::internal_error("Failed to create post folder"));
                    }

                    // Update the post with the folder ID
                    let mut updated_post = post.clone();
                    updated_post.folder_id = Some(new_folder_id.clone());
                    if let Err(e) = data.update_post(&updated_post).await {
                        error!("Failed to update post {} with folder ID: {}", post_id, e);
                        return HttpResponse::InternalServerError()
                            .json(ErrorResponse::internal_error("Failed to update post with folder ID"));
                    }

                    new_folder_id
                }
            };

            // Process multiple file uploads
            let mut uploaded_assets = Vec::new();
            let mut errors = Vec::new();

            let mut payload = payload;
            while let Some(item) = payload.next().await {
                match item {
                    Ok(mut field) => {
                        let content_disposition = field.content_disposition();
                        if let Some(content_disposition) = content_disposition {
                            let field_name = content_disposition.get_name();
                            if let Some(field_name) = field_name {
                                if field_name.starts_with("file") { // Support multiple files like file, file1, file2, etc.
                                    let file_name = content_disposition.get_filename()
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("unnamed_file_{}.dat", uploaded_assets.len()));

                                    let ext = StdPath::new(&file_name)
                                        .extension()
                                        .and_then(std::ffi::OsStr::to_str)
                                        .unwrap_or("dat");

                                    let unique_filename = format!("{}_{}.{}", Uuid::new_v4(), file_name.replace(".", "_"), ext);

                                    // Stream the file data directly to collect it in memory
                                    let mut file_data = Vec::new();
                                    while let Some(chunk_result) = field.next().await {
                                        match chunk_result {
                                            Ok(data) => file_data.extend_from_slice(&data),
                                            Err(e) => {
                                                error!("Failed to read chunk: {}", e);
                                                errors.push(format!("Failed to read chunk: {}", e));
                                                break;
                                            }
                                        }
                                    }

                                    // Upload the file to storage using the trait
                                    let upload_result = data.storage.upload_file(&unique_filename, &file_data).await;

                                    if let Err(e) = upload_result {
                                        error!("Failed to upload file to Supabase: {}", e);
                                        errors.push(format!("Failed to upload file: {}", e));
                                        continue;
                                    }

                                    info!("File saved successfully with filename: {}", unique_filename);

                                    // Create asset record in database
                                    let new_asset = Asset::new(
                                        file_name.clone(), // Use original filename as name
                                        unique_filename.clone(),
                                        format!("/assets/serve/{}", unique_filename),
                                        None,
                                    );

                                    debug!("Attempting to insert new asset into 'assets' table.");
                                    if let Err(e) = data.insert_asset(&new_asset).await {
                                        error!("Failed to insert asset into db: {}", e);
                                        errors.push(format!("Failed to insert asset into db: {}", e));
                                        continue;
                                    }
                                    info!("Asset {:?} created and stored in database.", new_asset.id);

                                    // Associate the asset with the post folder
                                    let folder_contents_result = data.get_folder_contents(&folder_id).await;
                                    let mut asset_ids = match folder_contents_result {
                                        Ok(Some(ids)) => ids,
                                        Ok(None) => Vec::new(),
                                        Err(e) => {
                                            error!("Database error when getting folder contents for post: {}", e);
                                            errors.push(format!("Failed to retrieve folder contents for post: {}", e));
                                            continue;
                                        }
                                    };
                                    asset_ids.push(new_asset.id);
                                    if let Err(e) = data.insert_folder_contents(&folder_id, &asset_ids).await {
                                        error!("Failed to associate asset with post folder: {}", e);
                                        errors.push(format!("Failed to associate asset with post folder: {}", e));
                                    } else {
                                        info!(
                                            "Asset {:?} successfully associated with post folder '{}'",
                                            new_asset.id, folder_id
                                        );
                                    }

                                    uploaded_assets.push(new_asset);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to process multipart field: {}", e);
                        errors.push(format!("Failed to process multipart field: {}", e));
                    }
                }
            }

            if !errors.is_empty() {
                error!("Errors occurred during upload: {:?}", errors);
            }

            if uploaded_assets.is_empty() {
                error!("No files were uploaded for post ID: {}", post_id);
                return HttpResponse::BadRequest()
                    .json(ErrorResponse::bad_request("No files were uploaded"));
            }

            // Return the first asset (or we could return all uploaded assets)
            HttpResponse::Created().json(uploaded_assets[0].clone()) // Return first asset
        }
        Ok(None) => {
            error!("Post not found for ID: {}", post_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Post with ID {} not found", post_id
            )))
        }
        Err(e) => {
            error!("Database error when fetching post {}: {}", post_id, e);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve post"))
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    // Since proper testing requires a database connection,
    // we'll focus on ensuring the handler compiles correctly
    // Comprehensive tests would require a full test database setup

    #[test]
    fn test_get_assets_by_ids_request_struct() {
        // Test that the request struct is properly defined
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let request = super::GetAssetsByIdsRequest { ids };

        // Verify we can create the struct as expected
        assert_eq!(request.ids.len(), 2);
    }
}
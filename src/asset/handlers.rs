use actix_files::NamedFile;
use actix_web::{web::{self, Path, Json}, HttpResponse, Responder};
use actix_multipart::Multipart;
use log::{info, error, debug};
use serde::Serialize;
use utoipa::ToSchema;
use std::collections::HashSet;

use uuid::Uuid;
use crate::{db::AppState, asset::models::Asset, storage, posting::models::Posting};
use crate::ErrorResponse;

// --- New Response Models for get_all_assets_structured ---

#[derive(Serialize, ToSchema)]
pub struct FolderWithAssets {
    pub name: String,
    pub assets: Vec<Asset>,
}

#[derive(Serialize, ToSchema)]
pub struct AllAssetsResponse {
    pub folders: Vec<FolderWithAssets>,
    pub unassigned_assets: Vec<Asset>,
}

// --- New and Refactored Handlers ---

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
pub async fn upload_asset(
    payload: Multipart,
    data: web::Data<AppState>,
) -> impl Responder {
    info!("Executing upload_asset handler");
    debug!("Attempting to save file from multipart payload.");
    match storage::save_file(payload).await {
        Ok((filename, posting_id_opt, folder_names, asset_name)) => {
            info!("File saved successfully with filename: {}", filename);
            let asset_id = Uuid::new_v4();
            debug!("Generated new asset ID: {:?}", asset_id);
            let name = asset_name.unwrap_or_else(|| filename.clone());
            let new_asset = Asset {
                id: asset_id,
                name,
                filename: filename.clone(),
                url: format!("/assets/serve/{}", filename),
                description: None,
            };

            debug!("Attempting to insert new asset into 'assets' column family.");
            if let Err(e) = data.insert_item("assets", &asset_id, &new_asset) {
                error!("Failed to insert asset into db: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::internal_error("Failed to save asset"));
            }
            info!("Asset {:?} created and stored in database.", asset_id);

            for folder_name in folder_names {
                debug!("Associating asset {:?} with folder '{}'", asset_id, folder_name);
                let mut asset_ids = data.get_folder_contents(&folder_name).unwrap().unwrap_or_default();
                asset_ids.push(asset_id);
                data.insert_folder_contents(&folder_name, &asset_ids).unwrap();
                info!("Asset {:?} successfully associated with folder '{}'", asset_id, folder_name);
            }

            if let Some(posting_id) = posting_id_opt {
                debug!("Associating asset {:?} with posting '{:?}'", asset_id, posting_id);
                if let Ok(Some(mut posting)) = data.get_item::<Posting>("postings", &posting_id) {
                    posting.asset_ids.push(asset_id);
                    if let Err(e) = data.insert_item("postings", &posting.id, &posting) {
                        error!("Failed to update posting {} with new asset {}: {}", posting.id, asset_id, e);
                    } else {
                        info!("Asset {:?} successfully associated with posting '{:?}'", asset_id, posting_id);
                    }
                } else {
                    error!("Posting not found for asset association: posting_id='{:?}'", posting_id);
                }
            }

            HttpResponse::Created().json(new_asset)
        }
        Err(e) => {
            error!("Failed during file upload process: {}", e);
            HttpResponse::BadRequest().json(ErrorResponse::bad_request(&e))
        },
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
    )
)]
pub async fn delete_asset(
    id: Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let asset_id_to_delete = id.into_inner();
    info!("Executing delete_asset handler for ID: {:?}", asset_id_to_delete);

    debug!("Attempting to fetch asset with ID {:?} for deletion.", asset_id_to_delete);
    match data.get_item::<Asset>("assets", &asset_id_to_delete) {
        Ok(Some(asset)) => {
            info!("Found asset {:?} to delete.", asset_id_to_delete);
            debug!("Attempting to delete physical asset file: {}", &asset.filename);
            if let Err(e) = storage::delete_asset_file(&asset.filename) {
                error!("Failed to delete physical asset file {}: {}.", asset.filename, e);
                return HttpResponse::InternalServerError().json(ErrorResponse::internal_error("Failed to delete asset file"));
            }
            info!("Physical file {} deleted successfully.", asset.filename);

            debug!("Attempting to delete asset record {:?} from 'assets' column family.", asset_id_to_delete);
            if let Err(e) = data.delete_item("assets", &asset_id_to_delete) {
                error!("Failed to delete asset from db, but file was deleted: {}", e);
                // In a real app, you might want to handle this inconsistency.
            }

            debug!("Scanning postings to disassociate asset {:?}", asset_id_to_delete);
            if let Ok(postings) = data.get_all_items::<Posting>("postings") {
                for mut posting in postings {
                    if posting.asset_ids.contains(&asset_id_to_delete) {
                        debug!("Disassociating asset {:?} from posting {:?}", asset_id_to_delete, posting.id);
                        posting.asset_ids.retain(|id| *id != asset_id_to_delete);
                        data.insert_item("postings", &posting.id, &posting).unwrap();
                    }
                }
            }
            
            debug!("Scanning folders to disassociate asset {:?}", asset_id_to_delete);
            let cf = data.db.cf_handle("folders").unwrap();
            let iter = data.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
            for item in iter {
                let (folder_name_bytes, asset_ids_bytes) = item.unwrap();
                let folder_name = String::from_utf8(folder_name_bytes.to_vec()).unwrap();
                let mut asset_ids: Vec<Uuid> = serde_json::from_slice(&asset_ids_bytes).unwrap();
                if asset_ids.contains(&asset_id_to_delete) {
                    debug!("Disassociating asset {:?} from folder '{}'", asset_id_to_delete, folder_name);
                    asset_ids.retain(|id| *id != asset_id_to_delete);
                    data.insert_folder_contents(&folder_name, &asset_ids).unwrap();
                }
            }

            info!("Asset {:?} deleted successfully from all records.", asset_id_to_delete);
            HttpResponse::NoContent().finish()
        },
        Ok(None) => {
            error!("Asset not found for deletion: {:?}", asset_id_to_delete);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!("Asset with ID {:?} not found", asset_id_to_delete)))
        },
        Err(e) => {
            error!("Failed to retrieve asset for deletion from database: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::internal_error("Failed to retrieve asset"))
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
    debug!("Attempting to fetch item with ID {:?} from 'assets' column family.", asset_id);
    match data.get_item::<Asset>("assets", &asset_id) {
        Ok(Some(asset)) => {
            info!("Successfully fetched asset with ID: {:?}", asset_id);
            HttpResponse::Ok().json(asset)
        },
        Ok(None) => {
            error!("Asset not found in database for ID: {:?}", asset_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!("Asset with ID {:?} not found", asset_id)))
        },
        Err(e) => {
            error!("Failed to get asset by ID '{}' from database: {}", asset_id, e);
            HttpResponse::InternalServerError().json(ErrorResponse::internal_error("Failed to retrieve asset"))
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
    debug!("Fetching all assets from 'assets' column family.");
    let all_assets = match data.get_all_items::<Asset>("assets") {
        Ok(assets) => {
            info!("Successfully fetched {} assets.", assets.len());
            assets
        },
        Err(e) => {
            error!("Failed to get all assets from database: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::internal_error("Failed to retrieve assets"));
        }
    };

    let mut asset_ids_in_folders = HashSet::new();
    let mut folders_with_assets: Vec<FolderWithAssets> = Vec::new();

    debug!("Fetching all folders and their asset IDs.");
    let cf = data.db.cf_handle("folders").unwrap();
    let iter = data.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
    for item in iter {
        let (folder_name_bytes, asset_ids_bytes) = item.unwrap();
        let name = String::from_utf8(folder_name_bytes.to_vec()).unwrap();
        let asset_ids: Vec<Uuid> = serde_json::from_slice(&asset_ids_bytes).unwrap();
        debug!("Processing folder '{}' with {} assets.", name, asset_ids.len());
        
        let assets_in_folder: Vec<Asset> = asset_ids.iter().filter_map(|id| {
            asset_ids_in_folders.insert(*id);
            all_assets.iter().find(|a| a.id == *id).cloned()
        }).collect();

        folders_with_assets.push(FolderWithAssets {
            name,
            assets: assets_in_folder,
        });
    }
    info!("Processed {} folders.", folders_with_assets.len());

    debug!("Filtering for unassigned assets.");
    let unassigned_assets: Vec<Asset> = all_assets.into_iter()
        .filter(|asset| !asset_ids_in_folders.contains(&asset.id))
        .collect();
    info!("Found {} unassigned assets.", unassigned_assets.len());

    let response = AllAssetsResponse {
        folders: folders_with_assets,
        unassigned_assets,
    };

    HttpResponse::Ok().json(response)
}


// --- Unchanged Handlers (but might need routing changes in main.rs) ---

pub async fn serve_asset(
    req: actix_web::HttpRequest,
    data: web::Data<AppState>,
) -> impl Responder {
    let filename: String = req.match_info().query("filename").into();
    info!("Executing serve_asset handler for filename: {}", &filename);
    
    debug!("Searching for asset with filename '{}' in database.", &filename);
    match data.get_all_items::<Asset>("assets") {
        Ok(assets) => {
            if let Some(asset) = assets.iter().find(|a| a.filename == filename) {
                info!("Asset found for filename: {}. Serving file.", &filename);
                let file_path = storage::get_asset_path(&asset.filename);
                if let Ok(file) = NamedFile::open(file_path) {
                    return file.into_response(&req);
                } else {
                    error!("Asset record found for '{}', but physical file is missing at path: {:?}", &filename, storage::get_asset_path(&asset.filename));
                }
            }
        },
        Err(e) => {
            error!("Database error while trying to serve asset '{}': {}", &filename, e);
        }
    }
    
    error!("Asset not found for serving: {}", &filename);
    HttpResponse::NotFound().json(ErrorResponse::not_found(&format!("Asset '{}' not found", filename)))
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
pub async fn create_folder_handler(req: Json<CreateFolderRequest>, data: web::Data<AppState>) -> impl Responder {
    info!("Executing create_folder_handler for folder: {}", &req.folder_name);
    
    if req.folder_name.is_empty() {
        error!("Folder name cannot be empty.");
        return HttpResponse::BadRequest().json(ErrorResponse::bad_request("Folder name cannot be empty"));
    }
    
    debug!("Attempting to create folder '{}' on filesystem.", &req.folder_name);
    match storage::create_folder(&req.folder_name) {
        Ok(_) => {
            info!("Folder '{}' created on filesystem.", &req.folder_name);
            debug!("Attempting to insert empty folder record '{}' into database.", &req.folder_name);
            if let Err(e) = data.insert_folder_contents(&req.folder_name, &vec![]) {
                error!("Failed to create folder record in db: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::internal_error("Failed to create folder record"));
            }
            info!("Folder record '{}' created successfully in database.", &req.folder_name);
            HttpResponse::Created().finish()
        },
        Err(e) => {
            error!("Failed to create folder '{}' on filesystem: {}", &req.folder_name, e);
            HttpResponse::BadRequest().json(ErrorResponse::bad_request(&e.to_string()))
        },
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    get,
    path = "/assets/folders/{folder_name}",
    params(
        ("folder_name" = String, Path, description = "Name of the folder to list contents")
    ),
    responses(
        (status = 200, description = "Folder contents listed successfully", body = Vec<FolderContent>),
        (status = 404, description = "Folder not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn list_folder_handler(folder_name: Path<String>) -> impl Responder {
    let folder_name = folder_name.into_inner();
    info!("Executing list_folder_handler for folder: {}", &folder_name);
    
    if folder_name.is_empty() {
        error!("Folder name cannot be empty.");
        return HttpResponse::BadRequest().json(ErrorResponse::bad_request("Folder name cannot be empty"));
    }
    
    if folder_name.contains("..") || folder_name.starts_with('/') || folder_name.contains("//") {
        error!("Invalid folder name provided: {}", &folder_name);
        return HttpResponse::BadRequest().json(ErrorResponse::bad_request("Invalid folder name"));
    }
    
    debug!("Attempting to list contents of folder '{}' from filesystem.", &folder_name);
    match storage::list_folder_contents(&folder_name) {
        Ok(contents) => {
            info!("Successfully listed {} items in folder '{}'", contents.len(), &folder_name);
            HttpResponse::Ok().json(contents)
        },
        Err(e) => {
            error!("Failed to list folder contents for '{}': {}", &folder_name, e);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!("Folder '{}' not found", folder_name)))
        },
    }
}

// --- Request Structs ---

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadAssetRequest {
    #[allow(unused)]
    pub file: Vec<u8>,
    #[allow(unused)]
    #[schema(value_type = String, nullable = true)]
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

// This struct is used for documentation purposes to show the form fields in Swagger UI
#[allow(dead_code)]
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CreateFolderForm {
    folder_name: String,
}

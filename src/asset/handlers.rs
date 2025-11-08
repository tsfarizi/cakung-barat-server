use actix_files::NamedFile;
use actix_web::{web::{self, Path, Json}, HttpResponse, Responder};
use actix_multipart::Multipart;
use log::{info, error, debug};
use serde::Serialize;
use utoipa::ToSchema;
use std::collections::HashSet;

use crate::schema::Uuid;
use crate::{db::SharedAppState, asset::models::Asset, storage};

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
    data: web::Data<SharedAppState>,
) -> impl Responder {
    info!("Attempting to upload an asset");
    match storage::save_file(payload).await {
        Ok((filename, posting_id_opt, folder_names)) => {
            let asset_id = Uuid(::uuid::Uuid::new_v4());
            let new_asset = Asset {
                id: asset_id,
                filename: filename.clone(),
                url: format!("/assets/{}", filename),
                description: None,
            };

            // Add asset to the central assets map
            data.assets.write().insert(asset_id, new_asset.clone());
            info!("Asset {:?} created and stored centrally.", asset_id);

            // Associate asset with folders
            let mut folders_map = data.folders.write();
            for folder_name in folder_names {
                folders_map.entry(folder_name.clone()).or_default().push(asset_id);
                debug!("Asset {:?} associated with folder '{}'", asset_id, folder_name);
            }

            // Optionally, associate asset with a posting
            if let Some(posting_id) = posting_id_opt {
                if let Some(posting) = data.postings.write().get_mut(&Uuid(posting_id)) {
                    posting.asset_ids.push(asset_id);
                    debug!("Asset {:?} associated with posting '{:?}'", asset_id, posting_id);
                } else {
                    error!("Posting not found for asset association: posting_id='{:?}'", posting_id);
                    // Note: Asset is created anyway, just not linked. This could be debated.
                }
            }

            HttpResponse::Created().json(new_asset)
        }
        Err(e) => {
            error!("Failed to upload asset: {}", e);
            HttpResponse::BadRequest().body(e)
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
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let asset_id_to_delete = id.into_inner();
    info!("Attempting to delete asset with id: {:?}", asset_id_to_delete);

    // 1. Remove from central asset map and get the filename
    let asset_to_delete = data.assets.write().remove(&asset_id_to_delete);

    if let Some(asset) = asset_to_delete {
        // 2. Delete physical file
        if let Err(e) = storage::delete_asset_file(&asset.filename) {
            error!("Failed to delete physical asset file {}: {}. Re-inserting asset into state.", asset.filename, e);
            // Re-insert asset if file deletion fails to maintain consistency
            data.assets.write().insert(asset_id_to_delete, asset);
            return HttpResponse::InternalServerError().body("Failed to delete asset file");
        }
        info!("Physical file {} deleted.", asset.filename);

        // 3. Remove asset ID from all folders
        let mut folders_map = data.folders.write();
        folders_map.values_mut().for_each(|asset_ids| {
            asset_ids.retain(|id| *id != asset_id_to_delete);
        });
        debug!("Removed asset ID {:?} from all folder associations.", asset_id_to_delete);

        // 4. Remove asset ID from all postings
        let mut postings_map = data.postings.write();
        postings_map.values_mut().for_each(|posting| {
            posting.asset_ids.retain(|id| *id != asset_id_to_delete);
        });
        debug!("Removed asset ID {:?} from all posting associations.", asset_id_to_delete);

        info!("Asset {:?} deleted successfully from all records.", asset_id_to_delete);
        HttpResponse::NoContent().finish()
    } else {
        error!("Asset not found for deletion: {:?}", asset_id_to_delete);
        HttpResponse::NotFound().body("Asset not found")
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
pub async fn get_asset_by_id(id: Path<Uuid>, data: web::Data<SharedAppState>) -> impl Responder {
    let asset_id = id.into_inner();
    debug!("Request to get asset by id: {:?}", asset_id);
    if let Some(asset) = data.assets.read().get(&asset_id) {
        HttpResponse::Ok().json(asset)
    } else {
        HttpResponse::NotFound().body("Asset not found")
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
pub async fn get_all_assets_structured(data: web::Data<SharedAppState>) -> impl Responder {
    let assets_map = data.assets.read();
    let folders_map = data.folders.read();
    let mut asset_ids_in_folders = HashSet::new();

    let folders_with_assets: Vec<FolderWithAssets> = folders_map.iter().map(|(name, asset_ids)| {
        let assets_in_folder: Vec<Asset> = asset_ids.iter().filter_map(|id| {
            asset_ids_in_folders.insert(*id);
            assets_map.get(id).cloned()
        }).collect();
        FolderWithAssets {
            name: name.clone(),
            assets: assets_in_folder,
        }
    }).collect();

    let unassigned_assets: Vec<Asset> = assets_map.values()
        .filter(|asset| !asset_ids_in_folders.contains(&asset.id))
        .cloned()
        .collect();

    let response = AllAssetsResponse {
        folders: folders_with_assets,
        unassigned_assets,
    };

    HttpResponse::Ok().json(response)
}


// --- Unchanged Handlers (but might need routing changes in main.rs) ---

pub async fn serve_asset(
    req: actix_web::HttpRequest,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let filename: String = req.match_info().query("filename").into();
    debug!("Attempting to serve asset with filename: {}", &filename);
    
    // In the new model, we don't know the asset ID from the filename alone easily.
    // A better URL would be /assets/{id}/serve, but for now, let's find it by filename.
    let assets = data.assets.read();
    if let Some(asset) = assets.values().find(|a| a.filename == filename) {
        let file_path = storage::get_asset_path(&asset.filename);
        if let Ok(file) = NamedFile::open(file_path) {
            return file.into_response(&req);
        }
    }
    
    error!("Asset not found for serving: {}", &filename);
    HttpResponse::NotFound().body("Asset not found")
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
pub async fn create_folder_handler(req: Json<CreateFolderRequest>) -> impl Responder {
    info!("Attempting to create folder: {}", &req.folder_name);
    match storage::create_folder(&req.folder_name) {
        Ok(_) => {
            info!("Folder '{}' created successfully", &req.folder_name);
            HttpResponse::Created().finish()
        },
        Err(e) => {
            error!("Failed to create folder '{}': {}", &req.folder_name, e);
            HttpResponse::BadRequest().body(e.to_string())
        },
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    get,
    path = "/assets/folders/{folder_name:.*}",
    responses(
        (status = 200, description = "Folder contents listed successfully", body = Vec<FolderContent>),
        (status = 404, description = "Folder not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn list_folder_handler(folder_name: Path<String>) -> impl Responder {
    info!("Listing contents of folder: {}", &folder_name);
    match storage::list_folder_contents(&folder_name) {
        Ok(contents) => {
            debug!("Found {} items in folder '{}'", contents.len(), &folder_name);
            HttpResponse::Ok().json(contents)
        },
        Err(_) => {
            error!("Folder not found: {}", &folder_name);
            HttpResponse::NotFound().finish()
        },
    }
}

// --- Request Structs ---

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadAssetRequest {
    #[allow(unused)]
    pub file: Vec<u8>,
    #[allow(unused)]
    pub posting_id: Option<Uuid>,
    #[allow(unused)]
    pub folders: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateFolderRequest {
    pub folder_name: String,
}

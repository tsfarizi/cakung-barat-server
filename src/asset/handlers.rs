use actix_files::NamedFile;
use actix_web::{web::{self, Path, Json}, HttpResponse, Responder};
use actix_multipart::Multipart;
use log::{info, error, debug};
use crate::schema::Uuid;

use crate::{db::SharedAppState, asset::models::Asset, storage};

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    post,
    path = "/assets/folders",
    request_body(content = inline(CreateFolderRequest), content_type = "application/json"),
    responses(
        (status = 201, description = "Folder created successfully"),
        (status = 400, description = "Invalid request")
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
        (status = 404, description = "Folder not found")
    ),
    params(
        ("folder_name" = String, Path, description = "Name of the folder to list")
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

#[utoipa::path(
    context_path = "/api",
    tag = "Asset Service",
    post,
    path = "/assets",
    request_body(content = inline(UploadAssetRequest), content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Asset created successfully", body = Asset),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upload_asset(
    payload: Multipart,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    info!("Attempting to upload an asset");
    match storage::save_file(payload).await {
        Ok((filename, posting_id, folder)) => {
            let asset_id = ::uuid::Uuid::new_v4();
            let url = if let Some(ref f) = folder {
                format!("/assets/{}/{}", f, asset_id)
            } else {
                format!("/assets/{}", asset_id)
            };
            info!("Asset uploaded successfully: filename='{}', posting_id='{:?}', folder='{:?}'", &filename, &posting_id, &folder);
            let new_asset = Asset {
                id: Uuid(asset_id),
                filename,
                url,
                description: None,
                folder,
            };

            let mut postings = data.postings.write();
            if let Some(posting) = postings.get_mut(&Uuid(posting_id)) {
                posting.assets.push(new_asset.clone());
                debug!("Asset associated with posting '{:?}'", posting_id);
                HttpResponse::Created().json(new_asset)
            } else {
                error!("Posting not found for asset: posting_id='{:?}'", posting_id);
                HttpResponse::NotFound().body("Posting not found")
            }
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
        (status = 404, description = "Asset not found")
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the asset to delete")
    )
)]
pub async fn delete_asset(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let asset_id = id.into_inner();
    info!("Attempting to delete asset with id: {:?}", asset_id);
    let mut postings = data.postings.write();
    let mut asset_to_delete: Option<Asset> = None;

    for posting in postings.values_mut() {
        if let Some(index) = posting.assets.iter().position(|a| a.id.0 == asset_id.0) {
            asset_to_delete = Some(posting.assets.remove(index));
            break;
        }
    }

    if let Some(asset) = asset_to_delete {
        info!("Deleting asset file: '{}' from folder: '{:?}'", &asset.filename, &asset.folder);
        if storage::delete_asset_file(&asset.filename, asset.folder.as_deref()).is_ok() {
            info!("Asset '{:?}' deleted successfully", asset_id);
            HttpResponse::NoContent().finish()
        } else {
            error!("Failed to delete asset file for asset id: {:?}", asset_id);
            HttpResponse::InternalServerError().body("Failed to delete asset file")
        }
    } else {
        error!("Asset not found for deletion: {:?}", asset_id);
        HttpResponse::NotFound().body("Asset not found")
    }
}

pub async fn serve_asset(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let asset_id = id.into_inner();
    debug!("Attempting to serve asset with id: {:?}", asset_id);
    let postings = data.postings.read();
    for posting in postings.values() {
        if let Some(asset) = posting.assets.iter().find(|a| a.id.0 == asset_id.0) {
            let file_path = storage::get_asset_path(&asset.filename, asset.folder.as_deref());
            debug!("Serving asset file: '{:?}'", &file_path);
            if let Ok(file) = NamedFile::open(file_path) {
                return file.into_response(&req);
            }
        }
    }
    error!("Asset not found for serving: {:?}", asset_id);
    HttpResponse::NotFound().body("Asset not found")
}


#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadAssetRequest {
    #[allow(unused)]
    pub file: Vec<u8>,
    #[allow(unused)]
    pub posting_id: Uuid,
    #[allow(unused)]
    pub folder: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateFolderRequest {
    pub folder_name: String,
}

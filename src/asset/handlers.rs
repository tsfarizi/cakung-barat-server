use actix_files::NamedFile;
use actix_web::{web::{self, Path}, HttpResponse, Responder};
use actix_multipart::Multipart;
use crate::schema::Uuid;

use crate::{db::SharedAppState, asset::models::Asset, storage};

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
    match storage::save_file(payload).await {
        Ok((filename, posting_id)) => {
            let asset_id = ::uuid::Uuid::new_v4();
            let new_asset = Asset {
                id: Uuid(asset_id),
                filename,
                url: format!("/assets/{}", asset_id),
                description: None,
            };

            let mut postings = data.postings.write();
            if let Some(posting) = postings.get_mut(&Uuid(posting_id)) {
                posting.assets.push(new_asset.clone());
                HttpResponse::Created().json(new_asset)
            } else {
                HttpResponse::NotFound().body("Posting not found")
            }
        }
        Err(e) => HttpResponse::BadRequest().body(e),
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
    let mut postings = data.postings.write();
    let asset_id = id.into_inner();
    let mut asset_to_delete: Option<Asset> = None;

    for posting in postings.values_mut() {
        if let Some(index) = posting.assets.iter().position(|a| a.id.0 == asset_id.0) {
            asset_to_delete = Some(posting.assets.remove(index));
            break;
        }
    }

    if let Some(asset) = asset_to_delete {
        if storage::delete_asset_file(&asset.filename).is_ok() {
            HttpResponse::NoContent().finish()
        } else {
            HttpResponse::InternalServerError().body("Failed to delete asset file")
        }
    } else {
        HttpResponse::NotFound().body("Asset not found")
    }
}

pub async fn serve_asset(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let asset_id = id.into_inner();
    let postings = data.postings.read();
    for posting in postings.values() {
        if let Some(asset) = posting.assets.iter().find(|a| a.id.0 == asset_id.0) {
            let file_path = storage::get_asset_path(&asset.filename);
            if let Ok(file) = NamedFile::open(file_path) {
                return file.into_response(&req);
            }
        }
    }
    HttpResponse::NotFound().body("Asset not found")
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadAssetRequest {
    pub file: Vec<u8>,
    pub posting_id: Uuid,
}

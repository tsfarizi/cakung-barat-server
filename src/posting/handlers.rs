use actix_web::{web::{self, Path}, HttpResponse, Responder};
use log::{info, error, debug};
use serde::Serialize;
use utoipa::ToSchema;

use crate::schema::{NaiveDate, Uuid};
use crate::asset::models::Asset;
use crate::{db::SharedAppState, posting::models::{CreatePostingRequest, Posting, UpdatePostingRequest}};

// --- Response Model ---

#[derive(Debug, Serialize, ToSchema)]
pub struct PostingResponse {
    pub id: Uuid,
    pub judul: String,
    pub tanggal: NaiveDate,
    pub detail: String,
    pub assets: Vec<Asset>,
}

// --- Handlers ---

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    get,
    path = "/postings",
    responses(
        (status = 200, description = "List of all postings", body = [PostingResponse]),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn get_all_postings(data: web::Data<SharedAppState>) -> impl Responder {
    info!("Request to get all postings");
    let postings_map = data.postings.read();
    let assets_map = data.assets.read();

    let response: Vec<PostingResponse> = postings_map.values().map(|posting| {
        let assets: Vec<Asset> = posting.asset_ids.iter()
            .filter_map(|id| assets_map.get(id).cloned())
            .collect();
        PostingResponse {
            id: posting.id,
            judul: posting.judul.clone(),
            tanggal: posting.tanggal,
            detail: posting.detail.clone(),
            assets,
        }
    }).collect();

    debug!("Found and hydrated {} postings", response.len());
    HttpResponse::Ok().json(response)
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    get,
    path = "/postings/{id}",
    responses(
        (status = 200, description = "Posting found", body = PostingResponse),
        (status = 404, description = "Posting not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the posting to retrieve")
    )
)]
pub async fn get_posting_by_id(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let posting_id = id.into_inner();
    info!("Request to get posting by id: {:?}", posting_id);
    let postings_map = data.postings.read();

    if let Some(posting) = postings_map.get(&posting_id) {
        let assets_map = data.assets.read();
        let assets: Vec<Asset> = posting.asset_ids.iter()
            .filter_map(|id| assets_map.get(id).cloned())
            .collect();

        let response = PostingResponse {
            id: posting.id,
            judul: posting.judul.clone(),
            tanggal: posting.tanggal,
            detail: posting.detail.clone(),
            assets,
        };
        debug!("Posting found and hydrated: {:?}", posting_id);
        HttpResponse::Ok().json(response)
    } else {
        error!("Posting not found: {:?}", posting_id);
        HttpResponse::NotFound().body("Posting not found")
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    post,
    path = "/postings",
    request_body = CreatePostingRequest,
    responses(
        (status = 201, description = "Posting created successfully", body = PostingResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn create_posting(
    req: web::Json<CreatePostingRequest>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    info!("Attempting to create a new posting");
    let assets_map = data.assets.read();
    let asset_ids = req.asset_ids.clone().unwrap_or_default();

    // Validate that all provided asset IDs exist
    for id in &asset_ids {
        if !assets_map.contains_key(id) {
            let msg = format!("Asset with ID {:?} not found", id);
            error!("{}", &msg);
            return HttpResponse::BadRequest().body(msg);
        }
    }

    let mut postings = data.postings.write();
    let new_id = Uuid(::uuid::Uuid::new_v4());
    let current_date = NaiveDate(::chrono::Utc::now().date_naive());

    let new_posting = Posting {
        id: new_id,
        judul: req.judul.clone(),
        tanggal: current_date,
        detail: req.detail.clone(),
        asset_ids,
    };

    postings.insert(new_id, new_posting.clone());
    info!("New posting created with id: {:?}", new_id);

    // Construct and return the hydrated response
    let assets: Vec<Asset> = new_posting.asset_ids.iter()
        .filter_map(|id| assets_map.get(id).cloned())
        .collect();
    
    let response = PostingResponse {
        id: new_posting.id,
        judul: new_posting.judul,
        tanggal: new_posting.tanggal,
        detail: new_posting.detail,
        assets,
    };

    HttpResponse::Created().json(response)
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    put,
    path = "/postings/{id}",
    request_body = UpdatePostingRequest,
    responses(
        (status = 200, description = "Posting updated successfully", body = PostingResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Posting not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the posting to update")
    )
)]
pub async fn update_posting(
    id: Path<Uuid>,
    req: web::Json<UpdatePostingRequest>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let posting_id = id.into_inner();
    info!("Attempting to update posting with id: {:?}", posting_id);
    let mut postings = data.postings.write();

    if let Some(posting) = postings.get_mut(&posting_id) {
        if let Some(judul) = &req.judul {
            debug!("Updating posting title for id: {:?}", posting_id);
            posting.judul = judul.clone();
        }
        if let Some(detail) = &req.detail {
            debug!("Updating posting detail for id: {:?}", posting_id);
            posting.detail = detail.clone();
        }
        if let Some(asset_ids) = &req.asset_ids {
            let assets_map = data.assets.read();
            for id in asset_ids {
                if !assets_map.contains_key(id) {
                    let msg = format!("Asset with ID {:?} not found", id);
                    error!("{}", &msg);
                    return HttpResponse::BadRequest().body(msg);
                }
            }
            debug!("Updating posting asset IDs for id: {:?}", posting_id);
            posting.asset_ids = asset_ids.clone();
        }

        let assets_map = data.assets.read();
        let assets: Vec<Asset> = posting.asset_ids.iter()
            .filter_map(|id| assets_map.get(id).cloned())
            .collect();

        let response = PostingResponse {
            id: posting.id,
            judul: posting.judul.clone(),
            tanggal: posting.tanggal,
            detail: posting.detail.clone(),
            assets,
        };
        info!("Posting with id: {:?} updated successfully", posting_id);
        HttpResponse::Ok().json(response)
    } else {
        error!("Posting not found for update: {:?}", posting_id);
        HttpResponse::NotFound().body("Posting not found")
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    delete,
    path = "/postings/{id}",
    responses(
        (status = 204, description = "Posting deleted successfully"),
        (status = 404, description = "Posting not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the posting to delete")
    )
)]
pub async fn delete_posting(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let posting_id = id.into_inner();
    info!("Attempting to delete posting with id: {:?}", posting_id);
    let mut postings = data.postings.write();
    if postings.remove(&posting_id).is_some() {
        info!("Posting with id: {:?} deleted successfully", posting_id);
        HttpResponse::NoContent().finish()
    } else {
        error!("Posting not found for deletion: {:?}", posting_id);
        HttpResponse::NotFound().body("Posting not found")
    }
}

use actix_web::{
    HttpResponse, Responder,
    web::{self, Path},
};
use log::{debug, error, info};
use serde::Serialize;
use utoipa::ToSchema;

use crate::asset::models::Asset;
use crate::{
    ErrorResponse,
    db::AppState,
    posting::models::{CreatePostingRequest, Posting, UpdatePostingRequest},
};
use chrono::NaiveDate;
use futures::future;
use uuid::Uuid;



#[derive(Debug, Serialize, ToSchema)]
pub struct PostingResponse {
    pub id: Uuid,
    pub judul: String,
    pub tanggal: NaiveDate,
    pub detail: String,
    pub assets: Vec<Asset>,
}



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
pub async fn get_all_postings(data: web::Data<AppState>) -> impl Responder {
    info!("Executing get_all_postings handler");
    debug!("Attempting to fetch all postings with their associated assets.");
    match data.get_all_postings_with_assets().await {
        Ok(postings) => {
            info!(
                "Successfully fetched {} postings from the database.",
                postings.len()
            );
            debug!("Hydrating posting responses with their associated assets.");
            
            let mut response: Vec<PostingResponse> = Vec::new();
            
            for posting in &postings {
                debug!("Hydrating assets for posting ID: {:?}", posting.id);
                
                // Create futures for fetching all assets concurrently
                let asset_futures: Vec<_> = posting
                    .asset_ids
                    .iter()
                    .map(|id| {
                        debug!("Fetching asset with ID: {:?}", id);
                        data.get_asset_by_id(id)
                    })
                    .collect();
                
                // Execute all futures concurrently
                let asset_results = match future::try_join_all(asset_futures).await {
                    Ok(results) => results,
                    Err(e) => {
                        error!("Failed to fetch assets for posting {}: {}", posting.id, e);
                        return HttpResponse::InternalServerError()
                            .json(ErrorResponse::internal_error("Failed to retrieve assets"));
                    }
                };
                
                // Collect non-None assets
                let assets: Vec<Asset> = asset_results.into_iter()
                    .filter_map(|opt| opt)
                    .collect();
                
                debug!(
                    "Found {} assets for posting ID: {:?}",
                    assets.len(),
                    posting.id
                );
                
                response.push(PostingResponse {
                    id: posting.id,
                    judul: posting.judul.clone(),
                    tanggal: posting.tanggal,
                    detail: posting.detail.clone(),
                    assets,
                });
            }
            
            info!("Successfully hydrated all posting responses.");
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("Failed to get all postings from database: {}", e);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve postings"))
        }
    }
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
pub async fn get_posting_by_id(id: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let posting_id = id.into_inner();
    info!(
        "Executing get_posting_by_id handler for ID: {:?}",
        posting_id
    );
    debug!(
        "Attempting to fetch posting with ID {:?} with associated assets.",
        posting_id
    );
    match data.get_posting_by_id_with_assets(&posting_id).await {
        Ok(Some(posting)) => {
            info!("Successfully fetched posting with ID: {:?}", posting_id);
            debug!("Hydrating assets for posting ID: {:?}", posting.id);
            
            // Create futures for fetching all assets concurrently
            let asset_futures: Vec<_> = posting
                .asset_ids
                .iter()
                .map(|id| {
                    debug!("Fetching asset with ID: {:?}", id);
                    data.get_asset_by_id(id)
                })
                .collect();
            
            // Execute all futures concurrently
            let asset_results = match future::try_join_all(asset_futures).await {
                Ok(results) => results,
                Err(e) => {
                    error!("Failed to fetch assets for posting {}: {}", posting.id, e);
                    return HttpResponse::InternalServerError()
                        .json(ErrorResponse::internal_error("Failed to retrieve assets"));
                }
            };
            
            // Collect non-None assets
            let assets: Vec<Asset> = asset_results.into_iter()
                .filter_map(|opt| opt)
                .collect();
            
            debug!(
                "Found {} assets for posting ID: {:?}",
                assets.len(),
                posting.id
            );

            let response = PostingResponse {
                id: posting.id,
                judul: posting.judul.clone(),
                tanggal: posting.tanggal,
                detail: posting.detail.clone(),
                assets,
            };
            info!(
                "Successfully hydrated posting response for ID: {:?}",
                posting_id
            );
            HttpResponse::Ok().json(response)
        }
        Ok(None) => {
            error!("Posting not found in database for ID: {:?}", posting_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Posting with ID {:?} not found",
                posting_id
            )))
        }
        Err(e) => {
            error!(
                "Failed to get posting by ID '{}' from database: {}",
                posting_id, e
            );
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve posting"))
        }
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
    data: web::Data<AppState>,
) -> impl Responder {
    info!("Executing create_posting handler");
    let asset_ids = req.asset_ids.clone().unwrap_or_default();
    debug!(
        "Received request to create posting with {} asset IDs.",
        asset_ids.len()
    );

    for id in &asset_ids {
        debug!("Validating asset with ID: {:?}", id);
        match data.get_asset_by_id(id).await {
            Ok(Some(_)) => {
                debug!("Asset validation successful for ID: {:?}", id);
            }
            Ok(None) => {
                let msg = format!("Asset with ID {:?} not found", id);
                error!("Asset validation failed: {}", &msg);
                return HttpResponse::BadRequest().json(ErrorResponse::bad_request(&msg));
            }
            Err(e) => {
                error!(
                    "Database error during asset validation for ID {:?}: {}",
                    id, e
                );
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to validate asset"));
            }
        }
    }
    info!("All assets validated successfully.");

    let current_date = ::chrono::Utc::now().date_naive();
    let mut new_posting = Posting::new(
        req.judul.clone(),
        req.detail.clone(),
        asset_ids.clone(),
    );
    new_posting.tanggal = current_date;

    debug!("Attempting to upsert new posting with assets into database.");
    if let Err(e) = data.upsert_posting_with_assets(&new_posting).await {
        error!("Failed to upsert new posting into database: {}", e);
        return HttpResponse::InternalServerError()
            .json(ErrorResponse::internal_error("Failed to create posting"));
    }

    info!("New posting created successfully with ID: {:?}", new_posting.id);

    debug!("Hydrating response for new posting.");
    
    // Create futures for fetching all assets concurrently
    let asset_futures: Vec<_> = asset_ids
        .iter()
        .map(|id| {
            debug!("Fetching asset with ID: {:?}", id);
            data.get_asset_by_id(id)
        })
        .collect();
    
    // Execute all futures concurrently
    let asset_results = match future::try_join_all(asset_futures).await {
        Ok(results) => results,
        Err(e) => {
            error!("Failed to fetch assets for new posting {}: {}", new_posting.id, e);
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve assets"));
        }
    };
    
    // Collect non-None assets
    let assets: Vec<Asset> = asset_results.into_iter()
        .filter_map(|opt| opt)
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
    data: web::Data<AppState>,
) -> impl Responder {
    let posting_id = id.into_inner();
    info!("Executing update_posting handler for ID: {:?}", posting_id);

    debug!(
        "Attempting to fetch posting with ID {:?} for update.",
        posting_id
    );
    match data.get_posting_by_id_with_assets(&posting_id).await {
        Ok(Some(mut posting)) => {
            info!(
                "Found posting with ID {:?}. Proceeding with update.",
                posting_id
            );
            if let Some(judul) = &req.judul {
                debug!("Updating posting title for id: {:?}", posting_id);
                posting.judul = judul.clone();
            }
            if let Some(detail) = &req.detail {
                debug!("Updating posting detail for id: {:?}", posting_id);
                posting.detail = detail.clone();
            }
            if let Some(asset_ids) = &req.asset_ids {
                debug!("Validating asset IDs for update.");
                for id in asset_ids {
                    match data.get_asset_by_id(id).await {
                        Ok(Some(_)) => (),
                        Ok(None) => {
                            let msg = format!("Asset with ID {:?} not found", id);
                            error!("Asset validation failed during update: {}", &msg);
                            return HttpResponse::BadRequest()
                                .json(ErrorResponse::bad_request(&msg));
                        }
                        Err(e) => {
                            error!(
                                "Database error during asset validation for ID {:?}: {}",
                                id, e
                            );
                            return HttpResponse::InternalServerError()
                                .json(ErrorResponse::internal_error("Failed to validate asset"));
                        }
                    }
                }
                debug!("Updating posting asset IDs for id: {:?}", posting_id);
                posting.asset_ids = asset_ids.clone();
            }

            debug!(
                "Attempting to upsert updated posting with ID {:?} into database.",
                posting_id
            );
            if let Err(e) = data.upsert_posting_with_assets(&posting).await {
                error!("Failed to update posting in database: {}", e);
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to update posting"));
            }

            debug!("Hydrating response for updated posting.");
            
            // Create futures for fetching all assets concurrently
            let asset_futures: Vec<_> = posting
                .asset_ids
                .iter()
                .map(|id| {
                    debug!("Fetching asset with ID: {:?}", id);
                    data.get_asset_by_id(id)
                })
                .collect();
            
            // Execute all futures concurrently
            let asset_results = match future::try_join_all(asset_futures).await {
                Ok(results) => results,
                Err(e) => {
                    error!("Failed to fetch assets for updated posting {}: {}", posting.id, e);
                    return HttpResponse::InternalServerError()
                        .json(ErrorResponse::internal_error("Failed to retrieve assets"));
                }
            };
            
            // Collect non-None assets
            let assets: Vec<Asset> = asset_results.into_iter()
                .filter_map(|opt| opt)
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
        }
        Ok(None) => {
            error!("Posting not found for update: {:?}", posting_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Posting with ID {:?} not found",
                posting_id
            )))
        }
        Err(e) => {
            error!("Failed to retrieve posting for update from database: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::internal_error(
                "Failed to retrieve posting for update",
            ))
        }
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
pub async fn delete_posting(id: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let posting_id = id.into_inner();
    info!("Executing delete_posting handler for ID: {:?}", posting_id);

    debug!(
        "Attempting to delete posting with ID {:?} from database.",
        posting_id
    );
    match data.delete_posting(&posting_id).await {
        Ok(_) => {
            info!(
                "Posting with id: {:?} deleted successfully from database.",
                posting_id
            );
            HttpResponse::NoContent().finish()
        }
        Err(e) => {
            error!(
                "Failed to delete posting with ID {:?} from database: {}",
                posting_id, e
            );
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to delete posting"))
        }
    }
}

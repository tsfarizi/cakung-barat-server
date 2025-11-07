use actix_web::{web::{self, Path}, HttpResponse, Responder};
use crate::schema::{NaiveDate, Uuid};

use crate::{db::SharedAppState, posting::models::{CreatePostingRequest, Posting, UpdatePostingRequest}};

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    get,
    path = "/postings",
    responses(
        (status = 200, description = "List of all postings", body = [Posting])
    )
)]
pub async fn get_all_postings(data: web::Data<SharedAppState>) -> impl Responder {
    let postings = data.postings.read();
    let all_postings: Vec<Posting> = postings.values().cloned().collect();
    HttpResponse::Ok().json(all_postings)
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    get,
    path = "/postings/{id}",
    responses(
        (status = 200, description = "Posting found", body = Posting),
        (status = 404, description = "Posting not found")
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the posting to retrieve")
    )
)]
pub async fn get_posting_by_id(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let postings = data.postings.read();
    if let Some(posting) = postings.get(&id.into_inner()) {
        HttpResponse::Ok().json(posting)
    } else {
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
        (status = 201, description = "Posting created successfully", body = Posting),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_posting(
    req: web::Json<CreatePostingRequest>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let mut postings = data.postings.write();
    let new_id = ::uuid::Uuid::new_v4();
    let current_date = ::chrono::Utc::now().date_naive();

    let assets = req.assets.clone().unwrap_or_default().into_iter().map(|mut asset| {
        if asset.id.0.is_nil() {
            asset.id = Uuid(::uuid::Uuid::new_v4());
        }
        asset.url = format!("/assets/{}", asset.id.0);
        asset
    }).collect();

    let new_posting = Posting {
        id: Uuid(new_id),
        judul: req.judul.clone(),
        tanggal: NaiveDate(current_date),
        detail: req.detail.clone(),
        assets,
    };

    postings.insert(Uuid(new_id), new_posting.clone());
    HttpResponse::Created().json(new_posting)
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    put,
    path = "/postings/{id}",
    request_body = UpdatePostingRequest,
    responses(
        (status = 200, description = "Posting updated successfully", body = Posting),
        (status = 404, description = "Posting not found"),
        (status = 400, description = "Invalid request")
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
    let mut postings = data.postings.write();
    let posting_id = id.into_inner();

    if let Some(posting) = postings.get_mut(&posting_id) {
        if let Some(judul) = &req.judul {
            posting.judul = judul.clone();
        }
        if let Some(detail) = &req.detail {
            posting.detail = detail.clone();
        }
        if let Some(assets) = &req.assets {
            posting.assets = assets.clone().into_iter().map(|mut asset| {
                if asset.id.0.is_nil() {
                    asset.id = Uuid(::uuid::Uuid::new_v4());
                }
                asset.url = format!("/assets/{}", asset.id.0);
                asset
            }).collect();
        }
        HttpResponse::Ok().json(posting.clone())
    } else {
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
        (status = 404, description = "Posting not found")
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the posting to delete")
    )
)]
pub async fn delete_posting(
    id: Path<Uuid>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let mut postings = data.postings.write();
    if postings.remove(&id.into_inner()).is_some() {
        HttpResponse::NoContent().finish()
    } else {
        HttpResponse::NotFound().body("Posting not found")
    }
}

use actix_web::{
    HttpResponse, Responder,
    web::{self, Path, Query},
};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    ErrorResponse,
    db::AppState,
    posting::models::{CreatePostingRequest, Post, UpdatePostingRequest},
};
use chrono::{NaiveDate};
use uuid::Uuid;

use crate::posting::multipart_parser::MultipartParser;


#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PostingResponse {
    pub id: Uuid,
    pub title: String,
    pub category: String,
    pub date: NaiveDate,
    pub excerpt: String,
    pub folder_id: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub asset_ids: Vec<Uuid>,  // Added for asset associations
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i32,

    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_page() -> i32 {
    1
}

fn default_limit() -> i32 {
    20
}



#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    get,
    path = "/postings",
    responses(
        (status = 200, description = "List of posts with pagination", body = [Post]),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("page" = Option<i32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<i32>, Query, description = "Number of items per page (default: 20)")
    )
)]
pub async fn get_all_postings(data: web::Data<AppState>, pagination: Query<PaginationParams>) -> impl Responder {
    info!("Executing get_all_postings handler with pagination");
    debug!("Attempting to fetch posts with pagination: page={}, limit={}", pagination.page, pagination.limit);

    let offset = (pagination.page - 1) * pagination.limit;

    match data.get_posts_smart_cached(pagination.limit, offset).await {
        Ok(posts) => {
            info!(
                "Successfully fetched {} posts using smart cache strategy.",
                posts.len()
            );
            HttpResponse::Ok().json(posts)
        }
        Err(e) => {
            error!("Failed to get posts: {}", e);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve posts"))
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    get,
    path = "/postings/{id}",
    responses(
        (status = 200, description = "Post found", body = Post),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the post to retrieve")
    )
)]
pub async fn get_posting_by_id(id: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let post_id = id.into_inner();
    info!(
        "Executing get_posting_by_id handler for ID: {:?}",
        post_id
    );
    debug!("Attempting to fetch post with ID {:?}.", post_id);
    match data.get_post_by_id(&post_id).await {
        Ok(Some(post)) => {
            info!("Successfully fetched post with ID: {:?}", post_id);
            HttpResponse::Ok().json(post)
        }
        Ok(None) => {
            error!("Post not found in database for ID: {:?}", post_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Post with ID {:?} not found",
                post_id
            )))
        }
        Err(e) => {
            error!(
                "Failed to get post by ID '{}' from database: {}",
                post_id, e
            );
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to retrieve post"))
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    post,
    path = "/postings",
    request_body(content = inline(CreatePostingRequest), content_type = "application/json"),
    responses(
        (status = 201, description = "Post created successfully", body = Post),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn create_posting(
    req: actix_web::web::Either<web::Json<CreatePostingRequest>, actix_multipart::Multipart>,
    data: web::Data<AppState>,
) -> impl Responder {
    info!("Executing create_posting handler");
    debug!("Received request to create post.");

    match req {
        actix_web::web::Either::Left(json_req) => {
            let folder_id = format!("posts/{}", Uuid::new_v4());

            let new_post = Post::new(
                json_req.title.clone(),
                json_req.category.clone(),
                json_req.excerpt.clone(),
                Some(folder_id),
            );

            debug!("Attempting to insert new post into database.");
            if let Err(e) = data.insert_post(&new_post).await {
                error!("Failed to insert new post into database: {}", e);
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to create post"));
            }

            info!("New post created successfully with ID: {:?}", new_post.id);
            HttpResponse::Created().json(new_post)
        }
        actix_web::web::Either::Right(multipart) => {
            let parsed_data = match MultipartParser::parse_posting_multipart(multipart).await {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to parse multipart data: {}", e);
                    return e.into();
                }
            };

            // Create a new post with a folder for its assets
            let folder_id = format!("posts/{}", Uuid::new_v4());
            let new_post = Post::new(
                parsed_data.title,
                parsed_data.category,
                parsed_data.excerpt,
                Some(folder_id.clone()),
            );

            // Insert the post into the database
            debug!("Attempting to insert new post into database.");
            if let Err(e) = data.insert_post(&new_post).await {
                error!("Failed to insert new post into database: {}", e);
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to create post"));
            }

            info!("New post created successfully with ID: {:?}", new_post.id);

            // Handle file uploads and associate them with the post folder
            for (i, item) in parsed_data.files_data.iter().enumerate() {
                let (file_data, original_filename) = item;
                // Create a unique filename for storage
                let file_extension = std::path::Path::new(&original_filename)
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("dat");

                let storage_filename = format!("{}_{:03}.{}",
                    new_post.id,
                    i,
                    file_extension
                );

                let result = data.storage.upload_file(&storage_filename, &file_data).await;

                match result {
                    Ok(_) => {
                        info!("File uploaded successfully to Supabase: {}", storage_filename);

                        let asset = crate::asset::models::Asset::new(
                            original_filename.clone(),
                            storage_filename.clone(),
                            format!("/assets/serve/{}", storage_filename),
                            None,
                        );

                        if let Err(e) = data.insert_asset(&asset).await {
                            error!("Failed to insert asset into db: {}", e);
                            // Continue processing other files even if one fails
                            continue;
                        }

                        // Associate the asset with the post's folder
                        match data.get_folder_contents(&folder_id).await {
                            Ok(Some(mut asset_ids)) => {
                                asset_ids.push(asset.id);
                                if let Err(e) = data.insert_folder_contents(&folder_id, &asset_ids).await {
                                    error!("Failed to associate asset with post folder: {}", e);
                                } else {
                                    info!("Asset {:?} associated with folder {}", asset.id, &folder_id);
                                }
                            }
                            Ok(None) => {
                                // Folder doesn't exist yet, create it with this asset
                                let asset_ids = vec![asset.id];
                                if let Err(e) = data.insert_folder_contents(&folder_id, &asset_ids).await {
                                    error!("Failed to create post folder: {}", e);
                                } else {
                                    info!("Created folder {} with asset {:?}", &folder_id, asset.id);
                                }
                            }
                            Err(e) => {
                                error!("Database error when getting folder contents: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upload file to Supabase: {}", e);
                    }
                }
            }

            HttpResponse::Created().json(new_post)
        }
    }
}
#[utoipa::path(
    context_path = "/api",
    tag = "Posting Service",
    put,
    path = "/postings/{id}",
    request_body = UpdatePostingRequest,
    responses(
        (status = 200, description = "Post updated successfully", body = Post),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the post to update")
    )
)]
pub async fn update_posting(
    id: Path<Uuid>,
    req: web::Json<UpdatePostingRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let post_id = id.into_inner();
    info!("Executing update_posting handler for ID: {:?}", post_id);

    debug!(
        "Attempting to fetch post with ID {:?} for update.",
        post_id
    );
    match data.get_post_by_id(&post_id).await {
        Ok(Some(mut post)) => {
            info!(
                "Found post with ID {:?}. Proceeding with update.",
                post_id
            );
            if let Some(title) = &req.title {
                debug!("Updating post title for id: {:?}", post_id);
                post.title = title.clone();
            }
            if let Some(category) = &req.category {
                debug!("Updating post category for id: {:?}", post_id);
                post.category = category.clone();
            }
            if let Some(excerpt) = &req.excerpt {
                debug!("Updating post excerpt for id: {:?}", post_id);
                post.excerpt = excerpt.clone();
            }
            if let Some(folder_id) = &req.folder_id {
                debug!("Updating post folder_id for id: {:?}", post_id);
                post.folder_id = Some(folder_id.clone());
            }

            debug!(
                "Attempting to update post with ID {:?} in database.",
                post_id
            );
            if let Err(e) = data.update_post(&post).await {
                error!("Failed to update post in database: {}", e);
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to update post"));
            }

            info!("Post with id: {:?} updated successfully", post_id);
            HttpResponse::Ok().json(post)
        }
        Ok(None) => {
            error!("Post not found for update: {:?}", post_id);
            HttpResponse::NotFound().json(ErrorResponse::not_found(&format!(
                "Post with ID {:?} not found",
                post_id
            )))
        }
        Err(e) => {
            error!("Failed to retrieve post for update from database: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::internal_error(
                "Failed to retrieve post for update",
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
        (status = 204, description = "Post deleted successfully"),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "ID of the post to delete")
    )
)]
pub async fn delete_posting(id: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let post_id = id.into_inner();
    info!("Executing delete_posting handler for ID: {:?}", post_id);

    debug!(
        "Attempting to delete post with ID {:?} from database.",
        post_id
    );
    match data.delete_post(&post_id).await {
        Ok(_) => {
            info!(
                "Post with id: {:?} deleted successfully from database.",
                post_id
            );
            HttpResponse::NoContent().finish()
        }
        Err(e) => {
            error!(
                "Failed to delete post with ID {:?} from database: {}",
                post_id, e
            );
            HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error("Failed to delete post"))
        }
    }
}


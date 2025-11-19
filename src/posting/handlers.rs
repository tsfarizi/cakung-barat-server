use actix_web::{
    HttpResponse, Responder,
    web::{self, Path, Query},
};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use futures::StreamExt;

use crate::{
    ErrorResponse,
    db::AppState,
    posting::models::{CreatePostingRequest, Post, UpdatePostingRequest},
};
use chrono::{NaiveDate};
use uuid::Uuid;

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

    // Calculate offset
    let offset = (pagination.page - 1) * pagination.limit;

    // For the default case (page=1, limit=20) or cwhen requesting first page with small limit,
    // use cached version to benefit from the N+1 query fix
    if pagination.page == 1 && pagination.limit <= 50 {
        match data.get_all_posts_cached().await {
            Ok(posts) => {
                info!(
                    "Successfully fetched {} posts from cache and applying pagination.",
                    posts.len()
                );
                // Apply pagination manually to cached results for first page
                let paginated_posts: Vec<crate::posting::models::Post> = posts.into_iter()
                    .skip(offset as usize)
                    .take(pagination.limit as usize)
                    .collect();
                HttpResponse::Ok().json(paginated_posts)
            }
            Err(e) => {
                error!("Failed to get posts from cache, falling back to database: {}", e);
                // Fallback to paginated query
                match data.get_posts_paginated(pagination.limit, offset).await {
                    Ok(posts) => {
                        info!(
                            "Successfully fetched {} posts from the database.",
                            posts.len()
                        );
                        HttpResponse::Ok().json(posts)
                    }
                    Err(e) => {
                        error!("Failed to get posts from database: {}", e);
                        HttpResponse::InternalServerError()
                            .json(ErrorResponse::internal_error("Failed to retrieve posts"))
                    }
                }
            }
        }
    } else {
        // For other pagination requests (including page > 1 or larger limits), use paginated query directly
        match data.get_posts_paginated(pagination.limit, offset).await {
            Ok(posts) => {
                info!(
                    "Successfully fetched {} posts from the database.",
                    posts.len()
                );
                HttpResponse::Ok().json(posts)
            }
            Err(e) => {
                error!("Failed to get posts from database: {}", e);
                HttpResponse::InternalServerError()
                    .json(ErrorResponse::internal_error("Failed to retrieve posts"))
            }
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
            // Handle regular JSON request
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
        actix_web::web::Either::Right(mut multipart) => {
            // Handle multipart request with files
            // Parse the multipart data to extract JSON metadata and files
            let mut title = String::new();
            let mut category = String::new();
            let mut excerpt = String::new();
            let mut files_data: Vec<(Vec<u8>, String)> = Vec::new(); // (file_data, original_filename)

            while let Some(item) = multipart.next().await {
                let mut field = item.unwrap();
                let content_disposition = field.content_disposition().unwrap();
                let name = content_disposition.get_name().unwrap();

                // Extract the filename before consuming the field
                let maybe_filename = content_disposition.get_filename().map(|s| s.to_string());

                if name == "metadata" {
                    // Handle JSON metadata
                    let mut buffer = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let data_chunk = chunk.unwrap();
                        buffer.extend_from_slice(&data_chunk);
                    }
                    let metadata_str = String::from_utf8(buffer).unwrap();
                    let metadata: CreatePostingRequest = serde_json::from_str(&metadata_str).unwrap();
                    title = metadata.title;
                    category = metadata.category;
                    excerpt = metadata.excerpt;
                } else if name.starts_with("file") {
                    // Handle uploaded files
                    let mut file_buffer = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let data_chunk = chunk.unwrap();
                        file_buffer.extend_from_slice(&data_chunk);
                    }

                    // Use the pre-extracted filename
                    let original_filename = match maybe_filename {
                        Some(fname) => fname,
                        None => format!("file_{}.dat", files_data.len()),
                    };

                    files_data.push((file_buffer, original_filename));
                }
            }

            // Create a new post with a folder for its assets
            let folder_id = format!("posts/{}", Uuid::new_v4());
            let new_post = Post::new(
                title,
                category,
                excerpt,
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
            for (i, (file_data, original_filename)) in files_data.iter().enumerate() {
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

                // Upload the file data directly to Supabase storage using the new public function
                let result = crate::storage::upload_file_to_supabase(
                    &storage_filename,
                    &file_data,
                    &data.http_client,
                    &data.supabase_config
                ).await;

                match result {
                    Ok(_) => {
                        info!("File uploaded successfully to Supabase: {}", storage_filename);

                        // Create asset record in database
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


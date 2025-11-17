use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, web};
use chrono;
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

mod asset;
mod db;
mod posting;
mod storage;

use crate::db::AppState;

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    error: String,
    message: String,
    timestamp: String,
}

impl ErrorResponse {
    pub fn new(error_type: &str, message: &str) -> Self {
        Self {
            error: error_type.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self::new("NotFound", message)
    }

    pub fn bad_request(message: &str) -> Self {
        Self::new("BadRequest", message)
    }

    pub fn internal_error(message: &str) -> Self {
        Self::new("InternalServerError", message)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    #[derive(ToSchema)]
    #[schema(value_type = String)]
    struct Uuid;

    #[derive(ToSchema)]
    #[schema(value_type = String)]
    struct NaiveDate;

    #[derive(ToSchema)]
    #[schema(value_type = String)]
    struct DateTime;

    #[derive(OpenApi)]
    #[openapi(
        paths(
            posting::handlers::get_all_postings,
            posting::handlers::get_posting_by_id,
            posting::handlers::create_posting,
            posting::handlers::update_posting,
            posting::handlers::delete_posting,
            asset::handlers::upload_asset,
            asset::handlers::delete_asset,
            asset::handlers::get_asset_by_id,
            asset::handlers::get_all_assets_structured,
            asset::handlers::create_folder_handler,
            asset::handlers::list_folder_handler,
            asset::handlers::get_assets_by_ids,
        ),
        components(
            schemas(
                posting::models::Posting,
                posting::models::Post,
                asset::models::Asset,
                posting::models::CreatePostingRequest,
                posting::models::UpdatePostingRequest,
                asset::handlers::UploadAssetRequest,
                asset::handlers::CreateFolderRequest,
                asset::handlers::GetAssetsByIdsRequest,
                posting::handlers::PostingResponse,
                asset::handlers::AllAssetsResponse,
                asset::handlers::FolderWithAssets,
                storage::FolderContent,
                ErrorResponse,
                Uuid,
                NaiveDate,
                DateTime
            )
        ),
        tags(
            (name = "Posting Service", description = "Posting CRUD endpoints."),
            (name = "Asset Service", description = "Asset and Folder endpoints.")
        ),
        servers(
            (url = "https://cakung-barat-server-1065513777845.asia-southeast1.run.app", description = "Production server"),
            (url = "https://5w4m7wvp-8080.asse.devtunnels.ms", description = "Staging server"),
            (url = "http://127.0.0.1:8080", description = "Localhost Staging server")
        )
    )]
    struct ApiDoc;

    let app_state = web::Data::new(AppState::new().await.unwrap());

    log::info!("Starting server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        let app_state = app_state.clone();
        let cors = Cors::default()
            .allowed_origin("https://cakung-barat-server-1065513777845.asia-southeast1.run.app")
            .allowed_origin("https://tsfarizi.github.io")
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:8080")
            .allowed_origin("http://127.0.0.1:8080")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(app_state)
            .service(
                web::scope("/api")
                    .service(
                        web::resource("/postings")
                            .route(web::get().to(posting::handlers::get_all_postings))
                            .route(web::post().to(posting::handlers::create_posting)),
                    )
                    .service(
                        web::resource("/postings/{id}")
                            .route(web::get().to(posting::handlers::get_posting_by_id))
                            .route(web::put().to(posting::handlers::update_posting))
                            .route(web::delete().to(posting::handlers::delete_posting)),
                    )
                    .service(
                        web::resource("/assets")
                            .route(web::get().to(asset::handlers::get_all_assets_structured))
                            .route(web::post().to(asset::handlers::upload_asset)),
                    )
                    .service(
                        web::resource("/assets/folders")
                            .route(web::post().to(asset::handlers::create_folder_handler)),
                    )
                    .service(
                        web::resource("/assets/folders/{folder_name:.*}")
                            .route(web::get().to(asset::handlers::list_folder_handler)),
                    )
                    .service(
                        web::resource("/assets/by-ids")
                            .route(web::post().to(asset::handlers::get_assets_by_ids)),
                    )
                    .service(
                        web::resource("/assets/{id}")
                            .route(web::get().to(asset::handlers::get_asset_by_id))
                            .route(web::delete().to(asset::handlers::delete_asset)),
                    )

            )
            .service(
                web::resource("/assets/serve/{filename:.*}")
                    .route(web::get().to(asset::handlers::serve_asset)),
            )
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-doc/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

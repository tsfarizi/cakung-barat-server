use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::{http::header, web, App, HttpServer};
use actix_web_prometheus::PrometheusMetricsBuilder;
use chrono;
use dotenvy;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

pub mod asset;
pub mod db;
pub mod organization;
pub mod posting;
pub mod storage;

pub use crate::db::AppState;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: String,
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

pub async fn run() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    #[derive(OpenApi)]
    #[openapi(
        paths(
            crate::posting::handlers::get_all_postings,
            crate::posting::handlers::create_posting,
            crate::posting::handlers::get_posting_by_id,
            crate::posting::handlers::update_posting,
            crate::posting::handlers::delete_posting,
            crate::asset::handlers::upload_asset,
            crate::asset::handlers::upload_asset_to_post,
            crate::asset::handlers::delete_asset,
            crate::asset::handlers::get_asset_by_id,
            crate::asset::handlers::get_all_assets_structured,
            crate::asset::handlers::create_folder_handler,
            crate::asset::handlers::list_folder_handler,
            crate::asset::handlers::get_assets_by_ids,
            crate::organization::routes::get_all_members,
            crate::organization::routes::create_member,
            crate::organization::routes::update_member,
            crate::organization::routes::delete_member
        ),
        components(
            schemas(
                posting::models::PostWithAssets,
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
                organization::model::OrganizationMember,
                organization::model::CreateMemberRequest,
                organization::model::UpdateMemberRequest,
            )
        ),
        tags(
            (name = "Posting Service", description = "Posting CRUD endpoints."),
            (name = "Asset Service", description = "Asset and Folder endpoints."),
            (name = "Organization", description = "Organization Structure endpoints.")
        ),
        servers(
            (url = "https://cakung-barat-server-1065513777845.asia-southeast2.run.app", description = "Production server"),
            (url = "https://5w4m7wvp-8080.asse.devtunnels.ms", description = "Staging server"),
            (url = "http://127.0.0.1:8080", description = "Localhost Staging server")
        )
    )]
    struct ApiDoc;

    dotenvy::dotenv().ok(); // Load .env file
    let supabase_config = crate::storage::SupabaseConfig::from_env().unwrap();
    let app_state = match AppState::new_with_config(supabase_config).await {
        Ok(state) => web::Data::new(state),
        Err(e) => {
            log::error!("Failed to connect to database. Please check your SUPABASE_DATABASE_URL in .env and ensure the database is running. Error: {}", e);
            std::process::exit(1);
        }
    };

    let prometheus = PrometheusMetricsBuilder::new("cakung_barat_server")
        .endpoint("/metrics")
        .build()
        .expect("Failed to create Prometheus metrics middleware");

    log::info!("Starting server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        let app_state = app_state.clone();
        let prometheus = prometheus.clone();
        let cors = Cors::default()
            .allowed_origin("https://cakung-barat-server-1065513777845.asia-southeast2.run.app")
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
            .wrap(Compress::default())
            .wrap(prometheus)
            .wrap(cors)
            .app_data(app_state)
            .service(
                web::scope("/api")
                    .configure(organization::routes::config) // Register organization routes
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
                        web::resource("/assets/posts/{post_id}")
                            .route(web::post().to(asset::handlers::upload_asset_to_post)),
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
                    ),
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
    .backlog(8192)
    .max_connections(25000)
    .keep_alive(actix_web::http::KeepAlive::Os)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

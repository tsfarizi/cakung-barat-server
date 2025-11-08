use std::sync::Arc;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use serde::Serialize;

mod db;
mod storage;
mod posting;
mod asset;
mod schema;

use crate::{
    db::AppState,
};

#[derive(Serialize, ToSchema)]
struct ErrorResponse {
    message: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe { std::env::set_var("RUST_LOG", "info"); }
    env_logger::init();

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
        ),
        components(
            schemas(
                posting::models::Posting,
                asset::models::Asset,
                posting::models::CreatePostingRequest,
                posting::models::UpdatePostingRequest,
                asset::handlers::UploadAssetRequest,
                asset::handlers::CreateFolderRequest,
                posting::handlers::PostingResponse,
                asset::handlers::AllAssetsResponse,
                asset::handlers::FolderWithAssets,
                storage::FolderContent,
                ErrorResponse,
                schema::Uuid,
                schema::NaiveDate,
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

    let app_state = web::Data::new(Arc::new(AppState::new()));

    log::info!("Starting server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://tsfarizi.github.io")
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:8080")
            .allowed_origin("http://127.0.0.1:8080")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![actix_web::http::header::AUTHORIZATION, actix_web::http::header::ACCEPT])
            .allowed_header(actix_web::http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .service(
                web::scope("/api")
                    .service(web::resource("/postings")
                        .route(web::get().to(posting::handlers::get_all_postings))
                        .route(web::post().to(posting::handlers::create_posting)))
                    .service(web::resource("/postings/{id}")
                        .route(web::get().to(posting::handlers::get_posting_by_id))
                        .route(web::put().to(posting::handlers::update_posting))
                        .route(web::delete().to(posting::handlers::delete_posting)))
                    .service(web::resource("/assets")
                        .route(web::get().to(asset::handlers::get_all_assets_structured))
                        .route(web::post().to(asset::handlers::upload_asset)))
                    .service(web::resource("/assets/{id}")
                        .route(web::get().to(asset::handlers::get_asset_by_id))
                        .route(web::delete().to(asset::handlers::delete_asset)))
                    .service(web::resource("/assets/folders")
                        .route(web::post().to(asset::handlers::create_folder_handler)))
                    .service(web::resource("/assets/folders/{folder_name:.*}")
                        .route(web::get().to(asset::handlers::list_folder_handler)))
            )
            .service(web::resource("/assets/serve/{filename:.*}")
                .route(web::get().to(asset::handlers::serve_asset)))
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-doc/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

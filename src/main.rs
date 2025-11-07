mod db;
mod storage;
mod posting;
mod asset;
mod schema;

use actix_web::{web, App, HttpServer};
use utoipa::{OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    db::AppState,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
        ),
        components(
            schemas(
                posting::models::Posting,
                posting::models::CreatePostingRequest,
                posting::models::UpdatePostingRequest,
                asset::models::Asset,
                asset::handlers::UploadAssetRequest,
                schema::Uuid,
                schema::NaiveDate,
            )
        ),
        tags(
            (name = "Posting Service", description = "Posting CRUD endpoints."),
            (name = "Asset Service", description = "Asset CRUD endpoints.")
        )
    )]
    struct ApiDoc;

    let app_state = web::Data::new(AppState::new());

    HttpServer::new(move || {
        App::new()
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
                        .route(web::post().to(asset::handlers::upload_asset)))
            )
            .service(web::resource("/assets/{id}")
                .route(web::get().to(asset::handlers::serve_asset))
                .route(web::delete().to(asset::handlers::delete_asset)))
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-doc/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

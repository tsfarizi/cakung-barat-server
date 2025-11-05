mod models;
mod db;
mod handlers;

use actix_web::{web, App, HttpServer};
use utoipa::{OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    db::AppState,
    handlers::{create_posting, delete_posting, get_all_postings, get_posting_by_id, update_posting,
        __path_create_posting, __path_delete_posting, __path_get_all_postings, __path_get_posting_by_id, __path_update_posting
    },
    models::{Asset, CreatePostingRequest, Posting, UpdatePostingRequest},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            get_all_postings,
            get_posting_by_id,
            create_posting,
            update_posting,
            delete_posting,
        ),
        components(
            schemas(Posting, Asset, CreatePostingRequest, UpdatePostingRequest)
        ),
        tags(
            (name = "Posting Service", description = "Posting CRUD endpoints.")
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
                        .route(web::get().to(get_all_postings))
                        .route(web::post().to(create_posting)))
                    .service(web::resource("/postings/{id}")
                        .route(web::get().to(get_posting_by_id))
                        .route(web::put().to(update_posting))
                        .route(web::delete().to(delete_posting))),
            )
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-doc/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
//! MCP HTTP/SSE Handlers for Actix-Web.

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::mcp::rpc::RpcRequest;
use crate::mcp::service::McpService;

/// MCP State for Actix-Web
pub struct McpState {
    pub service: McpService,
    pub tx: broadcast::Sender<String>,
}

impl McpState {
    pub fn new(service: McpService) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { service, tx }
    }
}

/// SSE handler - GET /sse
/// Establishes SSE connection and sends initial endpoint event
pub async fn sse_handler(state: web::Data<Arc<McpState>>, _req: HttpRequest) -> impl Responder {
    log::info!("Client connected to SSE stream");

    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx);

    // Create SSE stream with initial endpoint event
    let initial_event = format!("event: endpoint\ndata: /sse\n\n");

    let event_stream =
        futures::stream::once(
            async move { Ok::<_, std::io::Error>(web::Bytes::from(initial_event)) },
        )
        .chain(stream.map(|msg| match msg {
            Ok(data) => Ok(web::Bytes::from(format!("data: {}\n\n", data))),
            Err(_) => Ok(web::Bytes::from("event: error\ndata: stream error\n\n")),
        }));

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(event_stream)
}

/// RPC handler - POST /sse
/// Handles JSON-RPC requests
pub async fn rpc_handler(
    state: web::Data<Arc<McpState>>,
    body: web::Json<RpcRequest>,
) -> impl Responder {
    log::info!("Received MCP request: {}", body.method);

    if let Some(response) = state.service.handle_request(body.into_inner()) {
        return HttpResponse::Ok()
            .content_type("application/json")
            .json(response);
    }

    // Notifications return 202 Accepted
    HttpResponse::Accepted().finish()
}

/// Configure MCP routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/sse")
            .route(web::get().to(sse_handler))
            .route(web::post().to(rpc_handler)),
    );
}

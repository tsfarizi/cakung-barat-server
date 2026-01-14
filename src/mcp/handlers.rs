//! MCP Stateless HTTP Handlers for Actix-Web.
//!
//! This implementation uses stateless HTTP POST for Cloud Run / serverless compatibility.
//! No SSE connections are maintained - each request is independent.

use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

use crate::mcp::rpc::RpcRequest;
use crate::mcp::service::McpService;

/// MCP State for Actix-Web (stateless version)
pub struct McpState {
    pub service: McpService,
}

impl McpState {
    pub fn new(service: McpService) -> Self {
        Self { service }
    }
}

/// RPC handler - POST /mcp
/// Handles JSON-RPC requests in stateless mode
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

/// Configure MCP routes (stateless)
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/mcp").route(web::post().to(rpc_handler)));

    // Keep /sse route for backward compatibility (same as /mcp)
    cfg.service(web::resource("/sse").route(web::post().to(rpc_handler)));
}

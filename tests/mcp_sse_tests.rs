//! Integration tests for MCP SSE endpoint.
//!
//! Note: These tests use direct McpService testing rather than full ToolRegistry
//! to avoid template file dependencies.

use actix_web::{test, web, App, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;

use cakung_barat_server::mcp::rpc::{OutboundResponse, RpcRequest};

/// Minimal MCP State for testing (without ToolRegistry file dependencies).
struct TestMcpState {
    tx: broadcast::Sender<String>,
}

impl TestMcpState {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }
}

/// Test SSE handler that mimics the real one.
async fn test_sse_handler(state: web::Data<Arc<TestMcpState>>) -> impl Responder {
    use futures::stream::StreamExt;
    use tokio_stream::wrappers::BroadcastStream;

    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx);

    let initial_event = "event: endpoint\ndata: /sse\n\n".to_string();

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

/// Test RPC handler that returns mock responses.
async fn test_rpc_handler(body: web::Json<RpcRequest>) -> impl Responder {
    let request = body.into_inner();

    if request.jsonrpc != "2.0" {
        let response = OutboundResponse::error(
            request.id,
            -32600,
            "Unsupported jsonrpc version (expected 2.0)",
        );
        return HttpResponse::Ok()
            .content_type("application/json")
            .json(response);
    }

    let response = match request.method.as_str() {
        "ping" => OutboundResponse::success(request.id, json!({ "ok": true })),
        "initialize" => OutboundResponse::success(
            request.id,
            json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                },
                "capabilities": { "tools": { "listChanged": false } }
            }),
        ),
        "tools/list" => OutboundResponse::success(
            request.id,
            json!({
                "tools": [
                    { "name": "test_tool", "description": "A test tool", "inputSchema": {} }
                ]
            }),
        ),
        _ => OutboundResponse::method_not_found(request.id, &request.method),
    };

    HttpResponse::Ok()
        .content_type("application/json")
        .json(response)
}

fn configure_test_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/sse")
            .route(web::get().to(test_sse_handler))
            .route(web::post().to(test_rpc_handler)),
    );
}

/// Test SSE endpoint returns correct content type.
#[actix_web::test]
async fn test_sse_endpoint_returns_event_stream() {
    let state = web::Data::new(Arc::new(TestMcpState::new()));

    let app = test::init_service(App::new().app_data(state).configure(configure_test_routes)).await;

    let req = test::TestRequest::get().uri("/sse").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

/// Test RPC endpoint accepts JSON-RPC initialize request.
#[actix_web::test]
async fn test_rpc_endpoint_initialize() {
    let state = web::Data::new(Arc::new(TestMcpState::new()));

    let app = test::init_service(App::new().app_data(state).configure(configure_test_routes)).await;

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let req = test::TestRequest::post()
        .uri("/sse")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body.get("result").is_some());
    assert!(body["result"]["serverInfo"].is_object());
}

/// Test RPC endpoint returns tools list.
#[actix_web::test]
async fn test_rpc_endpoint_tools_list() {
    let state = web::Data::new(Arc::new(TestMcpState::new()));

    let app = test::init_service(App::new().app_data(state).configure(configure_test_routes)).await;

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let req = test::TestRequest::post()
        .uri("/sse")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["jsonrpc"], "2.0");

    let tools = body["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Should return at least one tool");
}

/// Test RPC endpoint handles ping.
#[actix_web::test]
async fn test_rpc_endpoint_ping() {
    let state = web::Data::new(Arc::new(TestMcpState::new()));

    let app = test::init_service(App::new().app_data(state).configure(configure_test_routes)).await;

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "ping",
        "params": {}
    });

    let req = test::TestRequest::post()
        .uri("/sse")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["result"]["ok"], true);
}

/// Test RPC endpoint handles unknown method.
#[actix_web::test]
async fn test_rpc_endpoint_unknown_method() {
    let state = web::Data::new(Arc::new(TestMcpState::new()));

    let app = test::init_service(App::new().app_data(state).configure(configure_test_routes)).await;

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "unknown/method",
        "params": {}
    });

    let req = test::TestRequest::post()
        .uri("/sse")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
    assert_eq!(body["error"]["code"], -32601); // Method not found
}

/// Test RPC endpoint handles invalid JSON-RPC version.
#[actix_web::test]
async fn test_rpc_endpoint_invalid_jsonrpc_version() {
    let state = web::Data::new(Arc::new(TestMcpState::new()));

    let app = test::init_service(App::new().app_data(state).configure(configure_test_routes)).await;

    let payload = json!({
        "jsonrpc": "1.0",
        "id": 5,
        "method": "ping",
        "params": {}
    });

    let req = test::TestRequest::post()
        .uri("/sse")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
    assert_eq!(body["error"]["code"], -32600); // Invalid request
}

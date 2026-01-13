//! MCP Service - Core JSON-RPC 2.0 request handler.

use crate::mcp::rpc::{OutboundResponse, RpcRequest};
use crate::mcp::tools::ToolRegistry;
use log::{info, warn};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Core MCP request handler.
#[derive(Clone)]
pub struct McpService {
    registry: Arc<ToolRegistry>,
}

impl McpService {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    pub fn handle_request(&self, request: RpcRequest) -> Option<OutboundResponse> {
        if request.jsonrpc != "2.0" {
            warn!("received unsupported jsonrpc version: {}", request.jsonrpc);
            return Some(OutboundResponse::error(
                request.id.clone(),
                -32600,
                "Unsupported jsonrpc version (expected 2.0)",
            ));
        }

        let RpcRequest {
            method, params, id, ..
        } = request;

        match method.as_str() {
            "initialize" => Some(self.handle_initialize(id, params)),
            "tools/list" => Some(self.handle_list_tools(id)),
            "tools/call" => Some(self.handle_call_tool(id, params)),
            "resources/list" => Some(self.handle_resources_list(id)),
            "resources/read" => Some(self.handle_resources_read(id, params)),
            "resources/templates/list" => Some(self.handle_resource_templates_list(id)),
            "prompts/list" => Some(self.handle_prompts_list(id)),
            "prompts/get" => Some(self.handle_prompts_get(id, params)),
            "ping" => Some(OutboundResponse::success(id, json!({ "ok": true }))),
            method if method.starts_with("notifications/") => {
                info!("received client notification: {}", method);
                None
            }
            other => Some(OutboundResponse::method_not_found(id, other)),
        }
    }

    fn handle_initialize(&self, id: Option<Value>, params: Option<Value>) -> OutboundResponse {
        let parsed: InitializeParams = match parse_params(params) {
            Ok(value) => value,
            Err(message) => return OutboundResponse::invalid_params(id, message),
        };

        info!(
            "client requested initialization: {} v{}",
            parsed.client_info.name,
            parsed
                .client_info
                .version
                .clone()
                .unwrap_or_else(|| "unknown".into())
        );

        let result = InitializeResult {
            protocol_version: PROTOCOL_VERSION.to_string(),
            server_info: ImplementationInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Cakung Barat MCP Server".to_string()),
            },
            capabilities: ServerCapabilities {
                tools: ToolsCapability {
                    list_changed: false,
                },
            },
        };

        OutboundResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_list_tools(&self, id: Option<Value>) -> OutboundResponse {
        let tools = self.registry.list_tools();
        let payload = ListToolsResult {
            tools,
            next_cursor: None,
        };

        OutboundResponse::success(id, serde_json::to_value(payload).unwrap())
    }

    fn handle_call_tool(&self, id: Option<Value>, params: Option<Value>) -> OutboundResponse {
        let parsed: CallToolParams = match parse_params(params) {
            Ok(value) => value,
            Err(message) => return OutboundResponse::invalid_params(id, message),
        };

        let result = self.registry.call_tool(&parsed.name, parsed.arguments);
        OutboundResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_resources_list(&self, id: Option<Value>) -> OutboundResponse {
        let payload = ListResourcesResult {
            resources: Vec::new(),
            next_cursor: None,
        };
        OutboundResponse::success(id, serde_json::to_value(payload).unwrap())
    }

    fn handle_resources_read(&self, id: Option<Value>, params: Option<Value>) -> OutboundResponse {
        let parsed: ResourceReadParams = match parse_params(params) {
            Ok(value) => value,
            Err(message) => return OutboundResponse::invalid_params(id, message),
        };

        let message = format!("Resource '{}' tidak ditemukan.", parsed.uri);
        OutboundResponse::error(id, -32000, message)
    }

    fn handle_resource_templates_list(&self, id: Option<Value>) -> OutboundResponse {
        let payload = ResourceTemplateListResult {
            templates: Vec::new(),
            next_cursor: None,
        };
        OutboundResponse::success(id, serde_json::to_value(payload).unwrap())
    }

    fn handle_prompts_list(&self, id: Option<Value>) -> OutboundResponse {
        let payload = PromptListResult {
            prompts: Vec::new(),
            next_cursor: None,
        };
        OutboundResponse::success(id, serde_json::to_value(payload).unwrap())
    }

    fn handle_prompts_get(&self, id: Option<Value>, params: Option<Value>) -> OutboundResponse {
        let parsed: PromptGetParams = match parse_params(params) {
            Ok(value) => value,
            Err(message) => return OutboundResponse::invalid_params(id, message),
        };

        let message = format!("Prompt '{}' tidak tersedia.", parsed.name);
        OutboundResponse::error(id, -32001, message)
    }
}

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    #[serde(rename = "clientInfo")]
    client_info: ClientInfo,
}

#[derive(Debug, Deserialize)]
struct ClientInfo {
    name: String,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Serialize)]
struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    #[serde(rename = "serverInfo")]
    server_info: ImplementationInfo,
    capabilities: ServerCapabilities,
}

#[derive(Debug, Serialize)]
struct ImplementationInfo {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServerCapabilities {
    tools: ToolsCapability,
}

#[derive(Debug, Serialize)]
struct ToolsCapability {
    #[serde(rename = "listChanged")]
    list_changed: bool,
}

#[derive(Debug, Serialize)]
struct ListToolsResult {
    tools: Vec<crate::mcp::tools::registry::ToolDescriptor>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallToolParams {
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ListResourcesResult {
    resources: Vec<ResourceDescriptor>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResourceDescriptor {
    uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResourceReadParams {
    uri: String,
}

#[derive(Debug, Serialize)]
struct ResourceTemplateListResult {
    templates: Vec<Value>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct PromptListResult {
    prompts: Vec<PromptDescriptor>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct PromptDescriptor {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    arguments: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct PromptGetParams {
    name: String,
}

fn parse_params<T: DeserializeOwned>(params: Option<Value>) -> Result<T, String> {
    match params {
        Some(value) => serde_json::from_value(value).map_err(|err| err.to_string()),
        None => serde_json::from_value(Value::Null).map_err(|err| err.to_string()),
    }
}

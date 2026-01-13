//! MCP (Model Context Protocol) Module
//!
//! Provides JSON-RPC 2.0 over HTTP/SSE for AI model integration.

pub mod content;
pub mod generators;
pub mod handlers;
pub mod rpc;
pub mod service;
pub mod tools;

pub use handlers::{config, McpState};
pub use service::McpService;
